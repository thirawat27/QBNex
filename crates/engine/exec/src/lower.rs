//! Loader for QIR (`docs/dev/qir.md`, crate `qb-ir`) onto the
//! typed register VM.
//!
//! QIR is already typed and register based; what this pass adds:
//!
//! - **Variables become registers.** Scalars (locals, byval params, globals,
//!   statics) live in registers: globals and statics in the main frame (SUBs
//!   reach them with `getg`/`setg`), BYREF params are copied in/out. A
//!   `load` is usually an alias of the variable's register (no instruction),
//!   a `store` usually retargets the instruction that produced the value.
//! - **Addresses are symbolic.** `addr`/`aref`/`aaddr` produce no value: a
//!   pointer register remembers *what* it points at, and the only uses
//!   accepted are a call argument (BYREF / array parameter), `ld`/`st` at
//!   offset 0 of an array element (`lde1`/`ste1`, ...) and the variable or
//!   array of COMMON / CHAIN (`chain.get` / `chain.put`).
//! - **Constants are registers** of the frame template (no `const` executed).
//! - Fusions (compare + branch, `acc`, backward
//!   jumps as event safe points), never across `line` or `label`.
//!
//! Programs that need addressable storage (TYPE, `STRING * n`, pointer
//! arithmetic, GET/PUT of variables) are lowered in memory mode, where only
//! the variables that need an address live in `mem::Mem`.

use std::collections::HashMap;

use crate::bytecode::*;
use qb_ir::{self as ir, BinOp, CmpOp, DimKind, Inst, Lit, ResumeKind, StoreTy, Ty as IT, UnOp, VarKind};

type Res<T> = Result<T, String>;
/// A function's slots and its result slot (with its type).
type FuncSlots = (Vec<Slot>, Option<(Slot, Ty)>);

/// What each QIR type is in the VM, in one table: integer width, register-mode
/// storage type (arrays, builtin arguments), memory access type, DECLARE
/// LIBRARY argument type and CHAIN/COMMON encoding. `None` = the type has no
/// such form.
struct TyMap {
    w: Option<W>,
    v: Option<Ty>,
    m: MTy,
    x: Option<XTy>,
    chain: Option<ChainK>,
}

fn ty_map(t: IT) -> TyMap {
    let int = |w, v, m, x, size, signed| TyMap { w: Some(w), v: Some(v), m, x: Some(x), chain: Some(ChainK::Int { size, signed }) };
    match t {
        IT::I8 => int(W::W8, Ty::I16, MTy::I8, XTy::I8, 1, true),
        IT::U8 => int(W::U8, Ty::I16, MTy::U8, XTy::U8, 1, false),
        IT::I16 => int(W::W16, Ty::I16, MTy::I16, XTy::I16, 2, true),
        IT::U16 => int(W::U16, Ty::I32, MTy::U16, XTy::U16, 2, false),
        IT::I32 => int(W::W32, Ty::I32, MTy::I32, XTy::I32, 4, true),
        IT::U32 => int(W::U32, Ty::I64, MTy::U32, XTy::U32, 4, false),
        IT::I64 => int(W::W64, Ty::I64, MTy::I64, XTy::I64, 8, true),
        IT::U64 => int(W::U64, Ty::I64, MTy::U64, XTy::U64, 8, false),
        IT::F32 => TyMap { w: None, v: Some(Ty::F32), m: MTy::F32, x: Some(XTy::F32), chain: Some(ChainK::F32) },
        IT::F64 => TyMap { w: None, v: Some(Ty::F64), m: MTy::F64, x: Some(XTy::F64), chain: Some(ChainK::F64) },
        // _FLOAT values live in register pairs and memory only
        IT::F80 => TyMap { w: None, v: None, m: MTy::F80, x: None, chain: None },
        IT::Str => TyMap { w: None, v: Some(Ty::Str), m: MTy::Str, x: Some(XTy::Str), chain: Some(ChainK::Str) },
        IT::Ptr => TyMap { w: None, v: None, m: MTy::Ptr, x: Some(XTy::Ptr), chain: None },
    }
}

fn width(t: IT) -> Res<W> {
    ty_map(t).w.ok_or_else(|| format!("no integer width for {}", t.name()))
}

/// VM type used for array storage and builtin arguments.
fn vty(t: IT) -> Res<Ty> {
    ty_map(t).v.ok_or_else(|| if t == IT::F80 { "_FLOAT (80-bit)".into() } else { "pointer value".into() })
}

fn cmp(op: CmpOp) -> Cmp {
    match op {
        CmpOp::Eq => Cmp::Eq,
        CmpOp::Ne => Cmp::Ne,
        CmpOp::Lt => Cmp::Lt,
        CmpOp::Le => Cmp::Le,
        CmpOp::Gt => Cmp::Gt,
        CmpOp::Ge => Cmp::Ge,
    }
}

/// Registers an instruction reads. Built without allocating: the scans over a
/// function body below ask for them once per instruction they pass, which made
/// a fresh `Vec` per call a large part of loading a big program.
#[derive(Clone, Copy)]
enum Uses<'a> {
    Few([u32; 2], u8),
    List(&'a [u32]),
    /// `dim` bounds: (lower, upper) per dimension
    Pairs(&'a [(u32, u32)]),
}

impl Uses<'_> {
    const NONE: Uses<'static> = Uses::Few([0; 2], 0);
    fn one(a: u32) -> Self {
        Uses::Few([a, 0], 1)
    }
    fn two(a: u32, b: u32) -> Self {
        Uses::Few([a, b], 2)
    }
    fn opt(a: Option<u32>) -> Self {
        a.map_or(Uses::NONE, Uses::one)
    }
    fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        const NO: &[u32] = &[];
        const NO_PAIRS: &[(u32, u32)] = &[];
        let (few, list, pairs) = match self {
            Uses::Few(a, n) => (&a[..*n as usize], NO, NO_PAIRS),
            Uses::List(l) => (NO, *l, NO_PAIRS),
            Uses::Pairs(p) => (NO, NO, *p),
        };
        few.iter().chain(list).copied().chain(pairs.iter().flat_map(|&(a, b)| [a, b]))
    }
    fn contains(&self, r: &u32) -> bool {
        self.iter().any(|u| u == *r)
    }
    /// How many times `r` is read.
    fn count(&self, r: u32) -> u32 {
        self.iter().filter(|&u| u == r).count() as u32
    }
}

/// Register defined and registers read by an instruction.
fn regs(i: &Inst) -> (Option<u32>, Uses<'_>) {
    use Inst::*;
    match i {
        Line(_) | IncLine { .. } | Label(_) | Jmp(_) | Gosub(_) | GRet | GRetL(_) | OnError(_) | Resume(_) | Restore(_) | End | Stop => {
            (None, Uses::NONE)
        }
        OnTimer { slot, secs, .. } => (None, Uses::two(*slot, *secs)),
        OnKey { key, .. } | OnPlay { n: key, .. } => (None, Uses::one(*key)),
        OnStrig { n, ctl, .. } => (None, Uses::two(*n, *ctl)),
        Const { dst, .. } => (Some(*dst), Uses::NONE),
        Mov { dst, src, .. } | Cvt { dst, src, .. } | Trunc { dst, src, .. } => (Some(*dst), Uses::one(*src)),
        Bin { dst, a, b, .. } | Cmp { dst, a, b, .. } | Concat { dst, a, b } => (Some(*dst), Uses::two(*a, *b)),
        Un { dst, a, .. } => (Some(*dst), Uses::one(*a)),
        Load { dst, .. } | Addr { dst, .. } | ARef { dst, .. } => (Some(*dst), Uses::NONE),
        Store { src, .. } => (None, Uses::one(*src)),
        AAddr { dst, idx, .. } => (Some(*dst), Uses::List(idx)),
        Ld { dst, ptr, .. } | LdFs { dst, ptr, .. } => (Some(*dst), Uses::one(*ptr)),
        St { ptr, src, .. } | StFs { ptr, src, .. } => (None, Uses::two(*ptr, *src)),
        PAdd { dst, ptr, off } => (Some(*dst), Uses::two(*ptr, *off)),
        Copy { dst, src, .. } => (None, Uses::two(*dst, *src)),
        Dim { bounds, .. } => (None, Uses::Pairs(bounds)),
        Erase { .. } => (None, Uses::NONE),
        LBound { dst, dim, .. } | UBound { dst, dim, .. } => (Some(*dst), Uses::one(*dim)),
        Jz { c, .. } | Jnz { c, .. } | Error(c) => (None, Uses::one(*c)),
        Call { dst, args, .. } => (*dst, Uses::List(args)),
        Ret(v) => (None, Uses::opt(*v)),
        Rt { dst, args, .. } => (*dst, Uses::List(args)),
        Read { dst, .. } => (Some(*dst), Uses::NONE),
        System(c) => (None, Uses::opt(*c)),
    }
}

/// Register type written by an instruction (None = no register result).
fn def_ty(m: &ir::Module, fnames: &HashMap<&str, &ir::Func>, i: &Inst) -> Option<IT> {
    use Inst::*;
    Some(match i {
        Const { ty, .. } | Mov { ty, .. } | Bin { ty, .. } | Un { ty, .. } | Load { ty, .. } | Ld { ty, .. } | Read { ty, .. } => *ty,
        Cmp { .. } | LBound { .. } | UBound { .. } => IT::I32,
        Cvt { to, .. } | Trunc { to, .. } => *to,
        Concat { .. } | LdFs { .. } => IT::Str,
        Addr { .. } | ARef { .. } | AAddr { .. } | PAdd { .. } => IT::Ptr,
        Call { dst: Some(_), func, .. } => match fnames.get(func.as_str()) {
            Some(f) => f.ret?,
            // DECLARE LIBRARY: result type of `meta extfn`
            // (no _FLOAT: such a function is not callable, `xfn`)
            None => m.extfn(func)?.ret.filter(|t| *t != IT::F80)?,
        },
        Rt { dst: Some(_), name, .. } => ir::builtins::lookup(name)?.ret?,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug)]
enum Var {
    N(R, IT),
    S(R),
    /// array operand (may carry G), element type
    A(R, IT),
    NG(R, IT),
    SG(R),
    /// memory mode: register holding the variable's address
    M(R, MK),
    /// memory mode: main-frame register holding a module-level variable's address
    MG(R, MK),
    /// register mode: BYREF scalar parameter that refers to its argument
    /// (register = reference, see [`Slot::R`]); type of the variable
    R(R, IT),
}

/// Kind of a memory-backed variable.
#[derive(Clone, Copy, Debug)]
enum MK {
    Scalar,
    FStr(u32),
    Udt,
    Arr,
}

fn mty(t: IT) -> MTy {
    ty_map(t).m
}

/// Names a function's body refers to through variables.
fn var_names(f: &ir::Func) -> std::collections::HashSet<&str> {
    let mut n = std::collections::HashSet::new();
    for i in &f.body {
        match i {
            Inst::Load { var, .. }
            | Inst::Store { var, .. }
            | Inst::Addr { var, .. }
            | Inst::ARef { var, .. }
            | Inst::AAddr { var, .. }
            | Inst::Dim { var, .. }
            | Inst::Erase { var }
            | Inst::LBound { var, .. }
            | Inst::UBound { var, .. } => {
                n.insert(var.as_str());
            }
            _ => {}
        }
    }
    n
}

#[derive(Clone, Copy, Debug)]
enum Rg {
    N(R),
    /// `_FLOAT` register pair (significand, sign/exponent)
    X(R),
    /// string register or K constant
    S(R),
    /// pointer register: what it points at (set by addr/aref/aaddr)
    P,
}

#[derive(Clone, Copy, Debug)]
enum Ptr {
    Place(Place),
    Elem { a: R, i: R, n: u32, ety: IT },
}

#[derive(Default)]
struct Frame {
    num_init: Vec<u64>,
    n_str: u32,
    arr_tys: Vec<Ty>,
    consts: HashMap<(u64, bool), R>,
    /// the registers of `consts` (asked for at every store)
    const_regs: std::collections::HashSet<R>,
    /// first numeric register that is not a variable (temporaries, QIR registers, constants)
    n_vars: u32,
    n_svars: u32,
}

impl Frame {
    fn new_n(&mut self, init: u64) -> R {
        self.num_init.push(init);
        self.num_init.len() as R - 1
    }
    fn new_s(&mut self) -> R {
        self.n_str += 1;
        self.n_str - 1
    }
    fn konst(&mut self, bits: u64, float: bool) -> R {
        if let Some(&r) = self.consts.get(&(bits, float)) {
            return r;
        }
        let r = self.new_n(bits);
        self.consts.insert((bits, float), r);
        self.const_regs.insert(r);
        r
    }
}

/// Per-function QIR register tables (`Lw::function`). QIR numbers registers
/// densely from 0 in each function, so they are arrays indexed by register
/// (hash maps here were a large part of loading a big program).
struct FnRegs {
    rty: Vec<Option<IT>>,
    defs: Vec<u32>,
    uses: Vec<u32>,
    /// QIR register -> VM register (allocated on first sight)
    rmap: Vec<Option<Rg>>,
    /// QIR register -> the register whose VM register it reuses (a widening cvt)
    alias_src: Vec<Option<u32>>,
}

impl FnRegs {
    /// VM register of QIR register `r` (allocated in frame `fr` on first sight).
    fn reg(&mut self, fr: &mut Frame, mem: bool, r: u32) -> Res<Rg> {
        if let Some(x) = self.rmap[r as usize] {
            return Ok(x);
        }
        let t = self.rty[r as usize].ok_or_else(|| format!("register %{r} has no type"))?;
        let x = match t {
            IT::Str => Rg::S(fr.new_s()),
            IT::F80 => {
                let r = fr.new_n(0);
                fr.new_n(0);
                Rg::X(r)
            }
            IT::Ptr if mem => Rg::N(fr.new_n(0)),
            IT::Ptr => Rg::P,
            _ => {
                vty(t)?;
                Rg::N(fr.new_n(0))
            }
        };
        self.rmap[r as usize] = Some(x);
        Ok(x)
    }

    fn n(&mut self, fr: &mut Frame, mem: bool, r: u32) -> Res<R> {
        match self.reg(fr, mem, r)? {
            Rg::N(x) => Ok(x),
            _ => Err(format!("%{r} is not numeric")),
        }
    }

    fn x(&mut self, fr: &mut Frame, mem: bool, r: u32) -> Res<R> {
        match self.reg(fr, mem, r)? {
            Rg::X(x) => Ok(x),
            _ => Err(format!("%{r} is not _FLOAT")),
        }
    }

    fn s(&mut self, fr: &mut Frame, mem: bool, r: u32) -> Res<R> {
        match self.reg(fr, mem, r)? {
            Rg::S(x) => Ok(x),
            _ => Err(format!("%{r} is not a string")),
        }
    }

    /// Defined and read once. A widening cvt makes its result share the VM
    /// register of its source: the fusions that take the only reader for
    /// granted need the source read once too (by that cvt).
    fn single(&self, mut r: u32) -> bool {
        loop {
            if self.defs[r as usize] != 1 || self.uses[r as usize] != 1 {
                return false;
            }
            match self.alias_src[r as usize] {
                Some(s) => r = s,
                None => return true,
            }
        }
    }
}

/// The register macros of a lowering function: `$s` is the `Lw`, `$rg` the
/// [`FnRegs`], `$k` the function index.
macro_rules! reg_macros {
    ($s:ident, $rg:ident, $k:ident) => {
        #[allow(unused_macros)]
        macro_rules! reg {
            ($r:expr) => {
                $rg.reg(&mut $s.frames[$k], $s.mem, $r)?
            };
        }
        #[allow(unused_macros)]
        macro_rules! nreg {
            ($r:expr) => {
                $rg.n(&mut $s.frames[$k], $s.mem, $r)?
            };
        }
        #[allow(unused_macros)]
        macro_rules! xreg {
            ($r:expr) => {
                $rg.x(&mut $s.frames[$k], $s.mem, $r)?
            };
        }
        #[allow(unused_macros)]
        macro_rules! sreg {
            ($r:expr) => {
                $rg.s(&mut $s.frames[$k], $s.mem, $r)?
            };
        }
    };
}

fn lit_num(ty: IT, l: &Lit) -> Res<u64> {
    Ok(match l {
        Lit::Int(n) => {
            if ty.is_float() {
                let v = *n as f64;
                (if ty == IT::F32 { f64::from(v as f32) } else { v }).to_bits()
            } else {
                *n as i64 as u64
            }
        }
        Lit::Float(t) => {
            let v = if ty == IT::F32 && !t.starts_with("0x") {
                t.parse::<f32>().map(f64::from).unwrap_or_else(|_| ir::float_value(t).unwrap_or(0.0))
            } else {
                ir::float_value(t).unwrap_or(0.0)
            };
            (if ty == IT::F32 { f64::from(v as f32) } else { v }).to_bits()
        }
        Lit::Str(_) => return Err("string literal in numeric const".into()),
    })
}

/// Value of an f80 literal (`0x` bits, inf/nan, or correctly rounded decimal).
fn lit_f80(l: &Lit) -> qb_float80::F80 {
    use qb_float80::F80;
    match l {
        Lit::Int(n) => F80::from_f64(*n as f64),
        Lit::Float(t) => match ir::f80_bits(t) {
            Some((se, m)) => F80 { mant: m, se },
            None => match t.as_str() {
                "inf" => F80::from_f64(f64::INFINITY),
                "-inf" => F80::from_f64(f64::NEG_INFINITY),
                "nan" => F80::from_f64(f64::NAN),
                _ => qb_float80::dec::parse(t.as_bytes()).0,
            },
        },
        Lit::Str(_) => F80::from_i64(0),
    }
}

fn check_store_ty(t: &StoreTy) -> Res<IT> {
    match t {
        StoreTy::Scalar(IT::F80) => Ok(IT::F80),
        StoreTy::Scalar(t) => {
            vty(*t)?;
            Ok(*t)
        }
        StoreTy::FStr(_) => Err("STRING * n".into()),
        StoreTy::Udt(_) => Err("TYPE (UDT)".into()),
    }
}

struct Lw<'a> {
    ir: &'a ir::Module,
    m: Module,
    kstr: HashMap<Vec<u8>, u32>,
    /// label key -> pc
    labels: HashMap<String, u32>,
    fixups: Vec<(u32, String)>,
    barrier: u32,
    fidx: HashMap<&'a str, usize>,
    /// QIR functions by name (`ir::Module::func` searches the list)
    fnames: HashMap<&'a str, &'a ir::Func>,
    /// module-level variables as seen from main (registers of frame 0)
    globals: HashMap<String, Var>,
    frames: Vec<Frame>,
    /// memory mode: variables that need an address live in `Mem` blocks
    mem: bool,
    /// (scope, name) of variables whose address is taken; scope = function index, MAX = module
    taken: std::collections::HashSet<(usize, String)>,
    /// register mode: (function, parameter) of the BYREF parameters that refer to
    /// their argument instead of holding a copy (`byref.rs`)
    refs: std::collections::HashSet<(usize, usize)>,
    /// instructions run on entry of each function (storage of memory-backed variables)
    prologue: Vec<Vec<Ins>>,
    /// register-table entries the functions may still allocate (see `function`)
    reg_budget: usize,
    /// memory mode: QIR instructions before this index of the function body are already
    /// lowered (field copies merged into one `CopyM`, see `copy_run`)
    copy_skip: usize,
    /// register mode: QIR instructions of `g$ = g$ + a [+ b]` on a module-level string
    /// (`app_chain`): at its index, nothing or the `AppGS` (g, a, b) is emitted
    app_plan: Vec<(usize, Option<AppGs>)>,
}

/// An `AppGS` to emit: the module-level string and the QIR registers of the parts.
type AppGs = (R, u32, Option<u32>);

/// Lowers a QIR module to bytecode, or says why it cannot.
/// Register mode first; programs that need addressable storage (TYPE,
/// `STRING * n`, pointer arithmetic, GET/PUT of variables) are lowered in
/// memory mode, where only the variables that need an address live in memory.
pub fn compile(src: &ir::Module) -> Res<Module> {
    ir::validate(src).map_err(|e| e.to_string())?;
    match compile_mode(src, false) {
        Ok(m) => Ok(m),
        Err(e1) => compile_mode(src, true).map_err(|e2| format!("{e1}; memory mode: {e2}")),
    }
}

fn compile_mode(src: &ir::Module, mem: bool) -> Res<Module> {
    if !mem && !src.types.is_empty() {
        return Err("TYPE (UDT)".into());
    }
    let main_i = src.funcs.iter().position(|f| f.name == "main").ok_or("no main")?;
    // function order: main first (frame 0 holds the globals)
    let mut order = vec![main_i];
    order.extend((0..src.funcs.len()).filter(|&i| i != main_i));
    let fidx: HashMap<&str, usize> = order.iter().enumerate().map(|(k, &i)| (src.funcs[i].name.as_str(), k)).collect();
    let mut lw = Lw {
        ir: src,
        m: Module {
                data: src.data.clone(),
                srcfiles: src.srcfiles.clone(),
                console: src.meta("console").map(|v| v == "only"),
                screenhide: src.meta("screenhide").is_some(),
                option_base: i64::from(src.meta("optionbase") == Some("1")),
                xfns: src.extfns.iter().filter_map(xfn).collect(),
                ..Module::default()
            },
        kstr: HashMap::new(),
        labels: HashMap::new(),
        fixups: Vec::new(),
        barrier: 0,
        copy_skip: 0,
        app_plan: Vec::new(),
        fidx,
        fnames: src.funcs.iter().map(|f| (f.name.as_str(), f)).collect(),
        globals: HashMap::new(),
        frames: Vec::from_fn(order.len(), |_| Frame::default()),
        mem,
        taken: std::collections::HashSet::new(),
        refs: if mem { std::collections::HashSet::new() } else { crate::byref::reference_params(src, &order) },
        prologue: vec![Vec::new(); order.len()],
        reg_budget: src.funcs.iter().map(|f| f.body.len()).sum::<usize>().saturating_mul(4) + (1 << 16),
    };
    if mem {
        for (k, &fi) in order.iter().enumerate() {
            let f = &src.funcs[fi];
            let own: std::collections::HashSet<&str> = f.params.iter().chain(&f.locals).map(|d| d.name.as_str()).collect();
            for i in &f.body {
                if let Inst::Addr { var, .. } | Inst::ARef { var, .. } | Inst::AAddr { var, .. } = i {
                    let scope = if own.contains(var.as_str()) { k } else { usize::MAX };
                    lw.taken.insert((scope, var.clone()));
                }
            }
        }
    }
    // module-level storage in frame 0: globals, then every function's statics
    for g in &src.globals {
        let v = lw.alloc_var(0, g, usize::MAX)?;
        lw.globals.insert(g.name.clone(), v);
    }
    let mut statics: Vec<HashMap<String, Var>> = vec![HashMap::new(); order.len()];
    for (k, &fi) in order.iter().enumerate() {
        for l in src.funcs[fi].locals.iter().filter(|l| l.kind == VarKind::Static) {
            let v = lw.alloc_var(0, l, k)?;
            statics[k].insert(l.name.clone(), v);
        }
    }
    let mut funcs = Vec::new();
    let mut const_sets: Vec<std::collections::HashSet<R>> = Vec::new();
    for (k, &fi) in order.iter().enumerate() {
        let f = &src.funcs[fi];
        let entry = lw.m.code.len() as u32;
        let (params, ret) = lw.function(k, f, &statics[k])?;
        let end = lw.m.code.len() as u32;
        let fr = std::mem::take(&mut lw.frames[k]);
        const_sets.push(fr.consts.values().copied().collect());
        funcs.push(Func {
            name: f.name.clone(),
            entry,
            end,
            num_init: fr.num_init,
            n_str: fr.n_str,
            arr_tys: fr.arr_tys,
            params,
            ret,
        });
    }
    for (pc, key) in std::mem::take(&mut lw.fixups) {
        let t = *lw.labels.get(&key).ok_or_else(|| format!("unknown label {key}"))?;
        *lw.m.code[pc as usize].target_mut().expect("fixup on a jump") = t;
    }
    let mut m = lw.m;
    for (f, consts) in funcs.iter().zip(&const_sets) {
        let zero = |r: R| consts.contains(&r) && f.num_init[r as usize] == 0;
        fuse_for_tests(&mut m.code, f.entry as usize..f.end as usize, &zero);
    }
    m.funcs = funcs;
    // SUB _GL (`meta glsub <function>`): called with no arguments at every display frame
    if let Some(name) = src.meta("glsub") {
        let f = m.funcs.iter().position(|f| f.name == name).ok_or_else(|| format!("meta glsub: unknown function {name}"))?;
        m.calls.push(CallDesc { args: Vec::new(), dest: None });
        m.gl_sub = Some((f as u32, m.calls.len() as u32 - 1));
    }
    m.validate().map_err(|e| format!("internal: {e}"))?;
    Ok(m)
}

impl<'a> Lw<'a> {
    fn pc(&self) -> u32 {
        self.m.code.len() as u32
    }

    fn emit(&mut self, i: Ins) -> u32 {
        self.m.code.push(i);
        self.pc() - 1
    }

    fn kstr(&mut self, s: &[u8]) -> R {
        if let Some(&k) = self.kstr.get(s) {
            return k | K;
        }
        let k = self.m.kstr.len() as u32;
        self.m.kstr.push(s.to_vec());
        self.kstr.insert(s.to_vec(), k);
        k | K
    }

    /// Storage for a declared variable in frame `k` (0 = module level).
    fn alloc_var(&mut self, k: usize, d: &ir::VarDecl, scope: usize) -> Res<Var> {
        if self.mem && (d.array || !matches!(d.ty, StoreTy::Scalar(_)) || self.taken.contains(&(scope, d.name.clone()))) {
            let (mk, mv) = self.mvar(d)?;
            self.m.mvars.push(mv);
            let v = self.m.mvars.len() as u32 - 1;
            let r = self.frames[k].new_n(0);
            self.prologue[k].push(Ins::NewV { d: r, v });
            return Ok(Var::M(r, mk));
        }
        let t = check_store_ty(&d.ty)?;
        let fr = &mut self.frames[k];
        Ok(if d.array {
            fr.arr_tys.push(vty(t)?);
            Var::A(fr.arr_tys.len() as R - 1, t)
        } else if t == IT::Str {
            Var::S(fr.new_s())
        } else if t == IT::F80 {
            let r = fr.new_n(0);
            fr.new_n(0);
            Var::N(r, t)
        } else {
            Var::N(fr.new_n(0), t)
        })
    }

    /// Memory layout of a variable declaration.
    fn mvar(&self, d: &ir::VarDecl) -> Res<(MK, MVar)> {
        // (storage starts as zero bytes, a `STRING * n` too)
        let (mk, size, strs) = match &d.ty {
            StoreTy::Scalar(t) => (MK::Scalar, t.size(), *t == IT::Str),
            StoreTy::FStr(n) => (MK::FStr(*n), *n, false),
            StoreTy::Udt(u) => (MK::Udt, self.ir.udt(u).ok_or_else(|| format!("unknown TYPE {u}"))?.size, false),
        };
        let mk = if d.array { MK::Arr } else { mk };
        Ok((mk, MVar { size, strs, array: d.array }))
    }

    /// QIR function by name.
    fn callee(&self, name: &str) -> Option<&'a ir::Func> {
        self.fnames.get(name).copied()
    }

    fn function(&mut self, k: usize, f: &'a ir::Func, statics: &HashMap<String, Var>) -> Res<FuncSlots> {
        let fkey = |l: &str| format!("{k}\u{1}{l}");
        // variables: this function's own names (statics, params, locals) over the
        // globals (module view). Looked up in that order instead of copying every
        // global into a per-function map, which cost globals x functions copies.
        let globals = std::mem::take(&mut self.globals);
        let mut vars: HashMap<&'a str, Var> = HashMap::new();
        for l in f.locals.iter().filter(|l| l.kind == VarKind::Static) {
            if let Some(v) = statics.get(&l.name) {
                vars.insert(l.name.as_str(), to_global(*v));
            }
        }
        let mut params = Vec::new();
        for (j, p) in f.params.iter().enumerate() {
            let byref = matches!(p.kind, VarKind::Param { byref: true }) || p.array;
            if !self.mem && byref && !p.array && self.refs.contains(&(k, j)) {
                // a reference to the argument (a copy could be told apart from it)
                let t = check_store_ty(&p.ty)?;
                if t == IT::F80 {
                    return Err("BYREF _FLOAT".into());
                }
                let r = self.frames[k].new_n(0);
                params.push(Slot::R(r));
                vars.insert(p.name.as_str(), Var::R(r, t));
                continue;
            }
            if self.mem && byref {
                // memory mode: BYREF / array parameters are the caller's pointers
                let (mk, _) = self.mvar(p)?;
                let r = self.frames[k].new_n(0);
                params.push(Slot::N(r));
                vars.insert(p.name.as_str(), Var::M(r, mk));
                continue;
            }
            if self.mem && (!matches!(p.ty, StoreTy::Scalar(_)) || self.taken.contains(&(k, p.name.clone()))) {
                // BYVAL parameter whose address is taken: copy the value into memory on entry
                let t = check_store_ty(&p.ty)?;
                let (mk, mv) = self.mvar(p)?;
                self.m.mvars.push(mv);
                let v = self.m.mvars.len() as u32 - 1;
                let ptr_r = self.frames[k].new_n(0);
                self.prologue[k].push(Ins::NewV { d: ptr_r, v });
                if t == IT::Str {
                    let sr = self.frames[k].new_s();
                    params.push(Slot::S(sr));
                    self.prologue[k].push(Ins::StM { t: MTy::Str, p: ptr_r, off: 0, s: sr });
                } else if t == IT::F80 {
                    let nr = self.frames[k].new_n(0);
                    self.frames[k].new_n(0);
                    params.push(Slot::N(nr));
                    params.push(Slot::N(nr + 1));
                    self.prologue[k].push(Ins::StM { t: MTy::F80, p: ptr_r, off: 0, s: nr });
                } else {
                    let nr = self.frames[k].new_n(0);
                    params.push(Slot::N(nr));
                    self.prologue[k].push(Ins::StM { t: mty(t), p: ptr_r, off: 0, s: nr });
                }
                vars.insert(p.name.as_str(), Var::M(ptr_r, mk));
                continue;
            }
            if matches!(p.ty, StoreTy::Scalar(IT::F80)) && !p.array {
                // BYVAL _FLOAT: the register pair is passed as two arguments
                let Var::N(r, _) = self.alloc_var(k, p, k)? else { return Err("bad _FLOAT parameter".into()) };
                params.push(Slot::N(r));
                params.push(Slot::N(r + 1));
                vars.insert(p.name.as_str(), Var::N(r, IT::F80));
                continue;
            }
            let v = self.alloc_var(k, p, k)?;
            params.push(match v {
                Var::N(r, _) => Slot::N(r),
                Var::S(r) => Slot::S(r),
                Var::A(r, _) => Slot::A(r),
                _ => unreachable!(),
            });
            vars.insert(p.name.as_str(), v);
        }
        for l in f.locals.iter().filter(|l| l.kind != VarKind::Static) {
            let v = self.alloc_var(k, l, k)?;
            vars.insert(l.name.as_str(), v);
        }
        if self.mem && k > 0 {
            // module-level memory variables: fetch their addresses once per call
            let mut fetch = Vec::new();
            // by name: HashMap order differs between runs, and the registers must not
            // (a native image built ahead of time checks the lowered program)
            let mut names: Vec<&'a str> = var_names(f)
                .into_iter()
                .filter(|n| matches!(lookup_var(&vars, &globals, k, n), Some(Var::MG(..))))
                .collect();
            names.sort_unstable();
            for n in names {
                if let Some(Var::MG(g, mk)) = lookup_var(&vars, &globals, k, n) {
                    let r = self.frames[k].new_n(0);
                    fetch.push(Ins::GetG { d: r, g });
                    vars.insert(n, Var::M(r, mk));
                }
            }
            self.prologue[k].extend(fetch);
        }
        let ret = match f.ret {
            None => None,
            Some(IT::Str) => Some((Slot::S(self.frames[k].new_s()), Ty::Str)),
            Some(IT::F80) => {
                let r = self.frames[k].new_n(0);
                self.frames[k].new_n(0);
                Some((Slot::N(r), Ty::F80))
            }
            Some(t) => {
                let ty = vty(t)?;
                Some((Slot::N(self.frames[k].new_n(0)), ty))
            }
        };
        {
            let fr = &mut self.frames[k];
            fr.n_vars = fr.num_init.len() as u32;
            fr.n_svars = fr.n_str;
        }
        // Per-register tables of this function. QIR numbers registers densely from
        // 0 in each function, so they are arrays indexed by register (hash maps here
        // were a large part of loading a big program).
        let n_regs = f.body.iter().filter_map(|i| {
            let (d, us) = regs(i);
            d.into_iter().chain(us.iter()).max()
        }).max().map_or(0, |r| r as usize + 1);
        // Registers are dense, so the highest one is about the number of
        // instructions that define one. Sparse numbers (`%16000000` in a small
        // function) would make every table below that large: the whole program
        // gets tables for 4 registers per instruction (and 64K more), no more.
        if n_regs > self.reg_budget {
            return Err(format!("{}: register %{} is far above the program's {} instructions", f.name, n_regs - 1, self.ir.funcs.iter().map(|f| f.body.len()).sum::<usize>()));
        }
        self.reg_budget -= n_regs;
        // register types and use counts
        let mut rty = vec![None::<IT>; n_regs];
        let mut defs = vec![0u32; n_regs];
        let mut uses = vec![0u32; n_regs];
        for i in &f.body {
            let (d, us) = regs(i);
            if let Some(d) = d {
                defs[d as usize] += 1;
                if let Some(t) = def_ty(self.ir, &self.fnames, i) {
                    rty[d as usize] = Some(t);
                }
            }
            for u in us.iter() {
                uses[u as usize] += 1;
            }
        }
        // QIR registers: constants defined once become constant registers / K strings
        let mut rmap = vec![None::<Rg>; n_regs];
        for i in &f.body {
            if let Inst::Const { ty, dst, val } = i
                && defs[*dst as usize] == 1 && rmap[*dst as usize].is_none() {
                    let rg = if *ty == IT::Str {
                        let Lit::Str(s) = val else { return Err("bad string const".into()) };
                        Rg::S(self.kstr(s))
                    } else if *ty == IT::F80 {
                        let x = lit_f80(val);
                        let fr = &mut self.frames[k];
                        let r = fr.new_n(x.mant);
                        fr.new_n(u64::from(x.se));
                        Rg::X(r)
                    } else if *ty == IT::Ptr {
                        if !self.mem {
                            return Err("pointer constant".into());
                        }
                        Rg::N(self.frames[k].konst(0, false))
                    } else {
                        vty(*ty)?;
                        let bits = lit_num(*ty, val)?;
                        Rg::N(self.frames[k].konst(bits, ty.is_float()))
                    };
                    rmap[*dst as usize] = Some(rg);
                }
        }
        let mut ptrs = vec![None::<Ptr>; n_regs];
        // register mode: QIR type of the variable (or array element) an addr / aref points at
        let mut ptr_ty = vec![None::<IT>; n_regs];
        // QIR register -> variable whose register it aliases
        let mut alias_var = vec![None::<&str>; n_regs];
        let mut rg = FnRegs { rty, defs, uses, rmap, alias_src: vec![None; n_regs] };
        let prologue = std::mem::take(&mut self.prologue[k]);
        self.emit_prologue(prologue);
        self.barrier = self.pc();
        let body = &f.body;
        self.copy_skip = 0;
        self.app_plan.clear();
        for (ii, ins) in body.iter().enumerate() {
            if ii < self.copy_skip {
                continue;
            }
            reg_macros!(self, rg, k);
            if let Some(at) = self.app_plan.iter().position(|p| p.0 == ii) {
                if let (_, Some((g, a, b))) = self.app_plan.swap_remove(at) {
                    let a = sreg!(a);
                    let b = match b {
                        Some(b) => sreg!(b),
                        None => NONE,
                    };
                    self.emit(Ins::AppGS { g, a, b });
                }
                continue;
            }
            macro_rules! var {
                ($n:expr) => {
                    lookup_var(&vars, &globals, k, $n.as_str()).ok_or_else(|| format!("unknown variable {}", $n))?
                };
            }
            if self.mem && self.lower_mem(k, &mut rg, &vars, &globals, body, ii)? {
                continue;
            }
            // _FLOAT instructions (register pairs)
            let f80_case = match ins {
                Inst::Const { ty: IT::F80, .. }
                | Inst::Mov { ty: IT::F80, .. }
                | Inst::Bin { ty: IT::F80, .. }
                | Inst::Un { ty: IT::F80, .. }
                | Inst::Cmp { ty: IT::F80, .. }
                | Inst::Load { ty: IT::F80, .. }
                | Inst::Store { ty: IT::F80, .. }
                | Inst::Read { ty: IT::F80, .. } => true,
                Inst::Cvt { to, from, .. } | Inst::Trunc { to, from, .. } => *to == IT::F80 || *from == IT::F80,
                _ => false,
            };
            if f80_case {
                self.lower_f80(k, &mut rg, &vars, &globals, ins)?;
                continue;
            }
            match ins {
                Inst::Line(n) => {
                    let pc = self.pc();
                    self.m.stmts.push((pc, *n));
                    self.barrier = pc;
                }
                Inst::IncLine { line, file } => {
                    // belongs to the statement the `line` before it started
                    if let Some(k) = self.m.stmts.len().checked_sub(1) {
                        self.m.incls.push((k as u32, *line, *file));
                    }
                }
                Inst::Label(l) => {
                    let pc = self.pc();
                    if self.labels.insert(fkey(l), pc).is_some() {
                        return Err(format!("duplicate label {l}"));
                    }
                    self.barrier = pc;
                }
                Inst::Const { ty, dst, val } => {
                    if rg.defs[*dst as usize] == 1 {
                        continue; // constant register
                    }
                    match reg!(*dst) {
                        Rg::N(d) => {
                            let bits = lit_num(*ty, val)?;
                            let kk = self.m.nconst.len() as u32;
                            self.m.nconst.push(bits);
                            self.emit(Ins::Ldk { d, k: kk });
                        }
                        Rg::S(d) => {
                            let Lit::Str(s) = val else { return Err("bad string const".into()) };
                            let kk = self.kstr(s);
                            self.emit(Ins::MovS { d, s: kk });
                        }
                        Rg::P | Rg::X(_) => return Err("pointer constant".into()),
                    }
                }
                Inst::Mov { dst, src, .. } => match (reg!(*dst), reg!(*src)) {
                    (Rg::N(d), Rg::N(s)) => {
                        self.emit(Ins::Mov { d, s });
                    }
                    (Rg::S(d), Rg::S(s)) => {
                        self.emit(Ins::MovS { d, s });
                    }
                    _ => return Err("pointer mov".into()),
                },
                Inst::Bin { op, ty, dst, a, b } => {
                    let (d, x, y) = (nreg!(*dst), nreg!(*a), nreg!(*b));
                    if ty.is_float() {
                        self.emit(match op {
                            BinOp::Add => Ins::AddF { d, a: x, b: y },
                            BinOp::Sub => Ins::SubF { d, a: x, b: y },
                            BinOp::Mul => Ins::MulF { d, a: x, b: y },
                            BinOp::Div => Ins::DivFI { d, a: x, b: y },
                            BinOp::Pow => Ins::PowF { d, a: x, b: y },
                            _ => return Err("bad float op".into()),
                        });
                        if *ty == IT::F32 {
                            self.emit(Ins::RoundF32 { d, s: d });
                        }
                    } else {
                        let w = width(*ty)?;
                        let (ins, wrapped) = match (op, w) {
                            (BinOp::Add, W::W16) => (Ins::AddI16 { d, a: x, b: y }, true),
                            (BinOp::Add, W::W32) => (Ins::AddI32 { d, a: x, b: y }, true),
                            (BinOp::Add, _) => (Ins::AddI64 { d, a: x, b: y }, w == W::W64),
                            (BinOp::Sub, W::W16) => (Ins::SubI16 { d, a: x, b: y }, true),
                            (BinOp::Sub, W::W32) => (Ins::SubI32 { d, a: x, b: y }, true),
                            (BinOp::Sub, _) => (Ins::SubI64 { d, a: x, b: y }, w == W::W64),
                            (BinOp::Mul, W::W16) => (Ins::MulI16 { d, a: x, b: y }, true),
                            (BinOp::Mul, W::W32) => (Ins::MulI32 { d, a: x, b: y }, true),
                            (BinOp::Mul, _) => (Ins::MulI64 { d, a: x, b: y }, w == W::W64),
                            (BinOp::IDiv, _) => (Ins::IDiv { d, a: x, b: y, w }, true),
                            (BinOp::Mod, _) => (Ins::Mod { d, a: x, b: y, w }, true),
                            (BinOp::And, _) => (Ins::And { d, a: x, b: y }, true),
                            (BinOp::Or, _) => (Ins::Or { d, a: x, b: y }, true),
                            (BinOp::Xor, _) => (Ins::Xor { d, a: x, b: y }, true),
                            (BinOp::Shl, _) => (Ins::Shl { d, a: x, b: y, w }, true),
                            (BinOp::Shr, _) => (Ins::Shr { d, a: x, b: y, w }, true),
                            _ => return Err("bad int op".into()),
                        };
                        self.emit(ins);
                        if !wrapped {
                            self.emit(Ins::Wrap { d, s: d, w });
                        }
                    }
                }
                Inst::Un { op, ty, dst, a } => {
                    let (d, s) = (nreg!(*dst), nreg!(*a));
                    match (op, ty.is_float()) {
                        (UnOp::Neg, true) => {
                            self.emit(Ins::Un { op: Un::NegF, d, s });
                        }
                        (UnOp::Neg, false) => {
                            self.emit(Ins::Un { op: Un::NegI(width(*ty)?), d, s });
                        }
                        (UnOp::Not, _) => {
                            self.emit(Ins::Un { op: Un::Not, d, s });
                            self.emit(Ins::Wrap { d, s: d, w: width(*ty)? });
                        }
                    }
                }
                Inst::Cmp { op, ty, dst, a, b } => {
                    let d = nreg!(*dst);
                    let c = cmp(*op);
                    if *ty == IT::Str {
                        let (x, y) = (sreg!(*a), sreg!(*b));
                        self.emit(Ins::CmpS { c, d, a: x, b: y });
                    } else {
                        let (x, y) = (nreg!(*a), nreg!(*b));
                        self.emit(if ty.is_float() {
                            Ins::CmpF { c, d, a: x, b: y }
                        } else if *ty == IT::U64 {
                            Ins::CmpU { c, d, a: x, b: y }
                        } else {
                            Ins::CmpI { c, d, a: x, b: y }
                        });
                    }
                }
                Inst::Cvt { to, from, dst, src } | Inst::Trunc { to, from, dst, src } => {
                    let trunc = matches!(ins, Inst::Trunc { .. });
                    if to.is_int() && from.is_int() && widens(*from, *to) && rg.defs[*dst as usize] == 1 && rg.rmap[*dst as usize].is_none() {
                        // widening an already normalized integer changes nothing: reuse the register
                        let s = nreg!(*src);
                        let n_uses = rg.uses[*dst as usize];
                        let ok = match alias_var[*src as usize] {
                            Some(v) => alias_ok(body, ii, *dst, v, n_uses),
                            None => stable_until_uses(body, ii, *dst, n_uses, &|i: &Inst| regs(i).0 == Some(*src)),
                        };
                        if ok {
                            rg.rmap[*dst as usize] = Some(Rg::N(s));
                            rg.alias_src[*dst as usize] = Some(*src);
                            if let Some(v) = alias_var[*src as usize] {
                                alias_var[*dst as usize] = Some(v);
                            }
                            continue;
                        }
                    }
                    let (d, s) = (nreg!(*dst), nreg!(*src));
                    match (to.is_float(), from.is_float()) {
                        (true, true) => {
                            self.emit(if *to == IT::F32 { Ins::RoundF32 { d, s } } else { Ins::Mov { d, s } });
                        }
                        (true, false) => {
                            self.emit(if *from == IT::U64 { Ins::IToFU { d, s } } else { Ins::IToF { d, s } });
                            if *to == IT::F32 {
                                self.emit(Ins::RoundF32 { d, s: d });
                            }
                        }
                        (false, true) => {
                            self.emit(Ins::CvtFI { d, s, w: width(*to)?, trunc });
                        }
                        (false, false) => {
                            self.emit(Ins::Wrap { d, s, w: width(*to)? });
                        }
                    }
                }
                Inst::Concat { dst, a, b } => {
                    let (d, x, y) = (sreg!(*dst), sreg!(*a), sreg!(*b));
                    self.emit(Ins::Concat { d, a: x, b: y });
                }
                Inst::Load { ty, dst, var } => {
                    let v = var!(var);
                    match v {
                        Var::N(r, _) | Var::S(r) => {
                            let alias = rg.defs[*dst as usize] == 1 && rg.rmap[*dst as usize].is_none() && alias_ok(body, ii, *dst, var, rg.uses[*dst as usize]);
                            let is_s = matches!(v, Var::S(_));
                            if alias {
                                rg.rmap[*dst as usize] = Some(if is_s { Rg::S(r) } else { Rg::N(r) });
                                alias_var[*dst as usize] = Some(var.as_str());
                            } else if is_s {
                                let d = sreg!(*dst);
                                self.emit(Ins::MovS { d, s: r });
                            } else {
                                let d = nreg!(*dst);
                                self.emit(Ins::Mov { d, s: r });
                            }
                        }
                        Var::NG(g, _) => {
                            let d = nreg!(*dst);
                            self.emit(Ins::GetG { d, g });
                        }
                        Var::SG(g) if rg.single(*dst) && rg.rmap[*dst as usize].is_none() && str_read_only(body, ii, *dst) => {
                            // its only reader takes the string from the module-level register: no copy
                            rg.rmap[*dst as usize] = Some(Rg::S(g | GS));
                        }
                        Var::SG(g) if let Some((steps, a, b)) = app_chain(body, ii, var, &|r| rg.single(r)) => {
                            // g$ = g$ + a [+ b]: appended in place (the load copied all of g$)
                            let last = *steps.iter().rev().nth(1).expect("a concat and the store");
                            for &j in &steps {
                                self.app_plan.push((j, if j == last { Some((g, a, b)) } else { None }));
                            }
                        }
                        Var::SG(g) => {
                            let d = sreg!(*dst);
                            self.emit(Ins::GetGS { d, g });
                        }
                        Var::R(r, IT::Str) => {
                            let d = sreg!(*dst);
                            self.emit(Ins::LdRS { d, r });
                        }
                        Var::R(r, _) => {
                            let d = nreg!(*dst);
                            self.emit(Ins::LdR { d, r });
                        }
                        Var::A(..) => return Err("load of an array".into()),
                        Var::M(..) | Var::MG(..) => return Err("memory variable outside memory mode".into()),
                    }
                    let _ = ty;
                }
                Inst::Store { ty, var, src } => {
                    let v = var!(var);
                    match v {
                        Var::N(r, t) => {
                            let s = nreg!(*src);
                            let produced = rg.single(*src) && s >= self.frames[k].n_vars && !self.is_const(k, s) && self.last_writes(false, s);
                            if *ty == IT::F32 && !(produced && matches!(self.m.code.last(), Some(Ins::RoundF32 { .. }))) && !self.is_const(k, s) {
                                self.emit(Ins::RoundF32 { d: r, s });
                            } else if s != r && !(produced && self.retarget(false, s, r)) {
                                self.emit(Ins::Mov { d: r, s });
                            }
                            if t.is_int() {
                                // every QIR register on the VM register is read once
                                let once = |x: R| {
                                    let mut on = rg.rmap.iter().enumerate().filter(|(_, g)| matches!(g, Some(Rg::N(y)) if *y == x)).peekable();
                                    on.peek().is_some() && on.all(|(v, _)| rg.single(v as u32))
                                };
                                self.fuse_acc(k, r, once);
                            }
                        }
                        Var::S(r) => {
                            let s = sreg!(*src);
                            if s != r {
                                let produced = rg.single(*src) && s & K == 0 && s >= self.frames[k].n_svars && self.last_writes(true, s);
                                if !(produced && self.retarget(true, s, r)) {
                                    let tmp = s & K == 0 && s >= self.frames[k].n_svars && rg.single(*src);
                                    self.emit(if tmp { Ins::TakeS { d: r, s } } else { Ins::MovS { d: r, s } });
                                }
                            }
                        }
                        Var::NG(g, _) => {
                            let s = nreg!(*src);
                            if *ty == IT::F32 && !self.is_const(k, s) {
                                let t = self.frames[k].new_n(0);
                                self.emit(Ins::RoundF32 { d: t, s });
                                self.emit(Ins::SetG { g, s: t });
                            } else {
                                self.emit(Ins::SetG { g, s });
                            }
                        }
                        Var::SG(g) => {
                            let s = sreg!(*src);
                            self.emit(Ins::SetGS { g, s });
                        }
                        Var::R(r, IT::Str) => {
                            let s = sreg!(*src);
                            self.emit(Ins::StRS { r, s });
                        }
                        Var::R(r, _) => {
                            let s = nreg!(*src);
                            if *ty == IT::F32 && !self.is_const(k, s) {
                                let t = self.frames[k].new_n(0);
                                self.emit(Ins::RoundF32 { d: t, s });
                                self.emit(Ins::StR { r, s: t });
                            } else {
                                self.emit(Ins::StR { r, s });
                            }
                        }
                        Var::A(..) => return Err("store to an array".into()),
                        Var::M(..) | Var::MG(..) => return Err("memory variable outside memory mode".into()),
                    }
                }
                Inst::Addr { dst, var } => {
                    let (p, t) = match var!(var) {
                        Var::N(r, t) => (Place::N(r, vty(t)?), t),
                        Var::NG(g, t) => (Place::NG(g, vty(t)?), t),
                        Var::S(r) => (Place::S(r), IT::Str),
                        Var::SG(g) => (Place::SG(g), IT::Str),
                        Var::R(r, t) => (Place::R(r, vty(t)?), t),
                        Var::A(..) => return Err("addr of an array".into()),
                        Var::M(..) | Var::MG(..) => return Err("memory variable outside memory mode".into()),
                    };
                    reg!(*dst);
                    ptrs[*dst as usize] = Some(Ptr::Place(p));
                    ptr_ty[*dst as usize] = Some(t);
                }
                Inst::ARef { dst, var } => {
                    let Var::A(a, t) = var!(var) else { return Err("aref of a scalar".into()) };
                    reg!(*dst);
                    ptrs[*dst as usize] = Some(Ptr::Place(Place::A(a)));
                    ptr_ty[*dst as usize] = Some(t);
                }
                Inst::AAddr { dst, var, idx } => {
                    let Var::A(a, ety) = var!(var) else { return Err("aaddr of a scalar".into()) };
                    let n = idx.len() as u32;
                    // one subscript that stays unchanged until the address is used: read it in place
                    // A BYREF argument is copied back after the callee ran, and the callee
                    // may have changed the subscript variable (SHARED): the place then
                    // needs its own copy of the subscript.
                    // A register defined once has all its reads behind it after `uses`
                    // of them: the scans below stop there. Scanning to the next definition
                    // (the end of the function, for such a register) made the loader
                    // quadratic in the number of array accesses of a function.
                    let reads = if rg.defs[*dst as usize] == 1 { rg.uses[*dst as usize] } else { u32::MAX };
                    let mut to_call = false;
                    let mut seen = 0;
                    for i in &body[ii + 1..] {
                        let (d, us) = regs(i);
                        if seen >= reads || d == Some(*dst) {
                            break;
                        }
                        seen += us.count(*dst);
                        if matches!(i, Inst::Call { args, .. } if args.contains(dst)) {
                            to_call = true;
                            break;
                        }
                    }
                    let direct = match idx.as_slice() {
                        [x] if !to_call => {
                            let x = *x;
                            let cnt = if reads == u32::MAX { uses_before_redef(body, ii, *dst) } else { reads };
                            let var = alias_var[x as usize];
                            stable_until_uses(body, ii, *dst, cnt, &|i: &Inst| {
                                regs(i).0 == Some(x) || matches!((i, var), (Inst::Store { var: v, .. }, Some(w)) if v == w)
                            })
                        }
                        _ => false,
                    };
                    let base = if direct {
                        nreg!(idx[0])
                    } else {
                        let base = self.frames[k].num_init.len() as R;
                        for _ in 0..n {
                            self.frames[k].new_n(0);
                        }
                        for (j, r) in idx.iter().enumerate() {
                            let s = nreg!(*r);
                            self.emit(Ins::Mov { d: base + j as R, s });
                        }
                        base
                    };
                    reg!(*dst);
                    ptrs[*dst as usize] = Some(Ptr::Elem { a, i: base, n, ety });
                    let read = if reads == u32::MAX { used_before_redef(body, ii, *dst) } else { reads > 0 };
                    if !read {
                        // address never used: still check the subscripts (error 9)
                        if ety == IT::Str {
                            let t = self.frames[k].new_s();
                            self.emit(if n == 1 { Ins::LdE1S { d: t, a, i: base } } else { Ins::LdES { d: t, a, i: base, n } });
                        } else {
                            let t = self.frames[k].new_n(0);
                            self.emit(if n == 1 { Ins::LdE1 { d: t, a, i: base } } else { Ins::LdE { d: t, a, i: base, n } });
                        }
                    }
                }
                Inst::Ld { ty, dst, ptr, off } | Inst::St { ty, ptr, off, src: dst } => {
                    let store = matches!(ins, Inst::St { .. });
                    if *off != 0 {
                        return Err("ld/st at an offset".into());
                    }
                    let Some(Ptr::Elem { a, i, n, ety }) = ptrs[*ptr as usize] else {
                        return Err("ld/st through a pointer that is not an array element".into());
                    };
                    if ety != *ty {
                        return Err("ld/st type differs from the element type".into());
                    }
                    if *ty == IT::Str {
                        let s = sreg!(*dst);
                        self.emit(match (store, n) {
                            (false, 1) => Ins::LdE1S { d: s, a, i },
                            (false, _) => Ins::LdES { d: s, a, i, n },
                            (true, 1) => Ins::StE1S { a, i, s },
                            (true, _) => Ins::StES { a, i, n, s },
                        });
                    } else {
                        let s = nreg!(*dst);
                        let s = if store && *ty == IT::F32 && !self.is_const(k, s) {
                            let t = self.frames[k].new_n(0);
                            self.emit(Ins::RoundF32 { d: t, s });
                            t
                        } else {
                            s
                        };
                        self.emit(match (store, n) {
                            (false, 1) => Ins::LdE1 { d: s, a, i },
                            (false, _) => Ins::LdE { d: s, a, i, n },
                            (true, 1) => Ins::StE1 { a, i, s },
                            (true, _) => Ins::StE { a, i, n, s },
                        });
                    }
                }
                Inst::LdFs { .. } | Inst::StFs { .. } => return Err("STRING * n field".into()),
                Inst::PAdd { .. } => return Err("pointer arithmetic".into()),
                Inst::Copy { .. } => return Err("memory copy".into()),
                Inst::Dim { kind, var, bounds } => {
                    let Var::A(a, _) = var!(var) else { return Err("dim of a scalar".into()) };
                    let n = bounds.len() as u32;
                    let base = self.frames[k].num_init.len() as R;
                    for _ in 0..2 * n {
                        self.frames[k].new_n(0);
                    }
                    for (j, (lo, hi)) in bounds.iter().enumerate() {
                        let (lo, hi) = (nreg!(*lo), nreg!(*hi));
                        self.emit(Ins::Mov { d: base + 2 * j as R, s: lo });
                        self.emit(Ins::Mov { d: base + 2 * j as R + 1, s: hi });
                    }
                    let mode = match kind {
                        DimKind::Dim => DimMode::Dim,
                            DimKind::Static => DimMode::Static,
                        DimKind::Redim => DimMode::Redim,
                        DimKind::RedimPreserve => DimMode::RedimPreserve,
                    };
                    self.emit(Ins::Dim { a, dims: base, n, mode });
                }
                Inst::Erase { var } => {
                    let Var::A(a, _) = var!(var) else { return Err("erase of a scalar".into()) };
                    self.emit(Ins::Erase { a });
                }
                Inst::LBound { dst, var, dim } | Inst::UBound { dst, var, dim } => {
                    let Var::A(a, _) = var!(var) else { return Err("bound of a scalar".into()) };
                    let (d, dm) = (nreg!(*dst), nreg!(*dim));
                    self.emit(Ins::Bound { d, a, dim: dm, upper: matches!(ins, Inst::UBound { .. }) });
                }
                Inst::Jmp(l) => self.jump(Ins::Jmp { t: 0 }, &fkey(l)),
                Inst::Jz { c, target } | Inst::Jnz { c, target } => {
                    let on_true = matches!(ins, Inst::Jnz { .. });
                    let r = nreg!(*c);
                    let mut j = if on_true { Ins::JnzI { s: r, t: 0 } } else { Ins::JzI { s: r, t: 0 } };
                    if rg.single(*c) && self.pc() > self.barrier {
                        let fused = match self.m.code.last().copied() {
                            Some(Ins::CmpI { c: op, d, a, b }) if d == r => Some(Ins::BrI { c: if on_true { op } else { op.inverse() }, a, b, t: 0 }),
                            Some(Ins::CmpF { c: op, d, a, b }) if d == r => Some(Ins::BrF { c: if on_true { op } else { op.negated() }, a, b, t: 0 }),
                            // native BrS reads string operands from the frame, not from memory
                            Some(Ins::CmpS { c: op, d, a, b }) if d == r && !mem_str(a) && !mem_str(b) => Some(Ins::BrS { c: if on_true { op } else { op.inverse() }, a, b, t: 0 }),
                            Some(Ins::CmpU { c: op, d, a, b }) if d == r => Some(Ins::BrU { c: if on_true { op } else { op.inverse() }, a, b, t: 0 }),
                            _ => None,
                        };
                        if let Some(b) = fused {
                            self.m.code.pop();
                            j = b;
                        }
                    }
                    self.jump(j, &fkey(target));
                }
                Inst::Gosub(l) => {
                    let pc = self.emit(Ins::Gosub { t: 0 });
                    self.fixups.push((pc, fkey(l)));
                }
                Inst::GRet => {
                    self.emit(Ins::Return);
                }
                Inst::GRetL(l) => {
                    let pc = self.emit(Ins::ReturnL { t: 0 });
                    self.fixups.push((pc, fkey(l)));
                }
                Inst::Call { func, .. } if self.callee(func).is_none() => {
                    // DECLARE LIBRARY calls pass real addresses: memory mode only
                    return Err(format!("{func}: DECLARE LIBRARY call (memory mode)"));
                }
                Inst::Call { dst, func, args } => {
                    let callee = self.callee(func).ok_or_else(|| format!("unknown function {func}"))?;
                    let fi = self.fidx[func.as_str()] as u32;
                    let mut out = Vec::new();
                    for (r, p) in args.iter().zip(&callee.params) {
                        let byref = matches!(p.kind, VarKind::Param { byref: true }) || p.array;
                        if byref {
                            let pl = match ptrs[*r as usize] {
                                Some(Ptr::Place(pl)) => pl,
                                Some(Ptr::Elem { a, i, n, ety }) => Place::E { a, i, n, ty: vty(ety)? },
                                None => return Err(format!("BYREF argument %{r} is not an address")),
                            };
                            self.m.places.push(pl);
                            out.push(Arg::Ref(self.m.places.len() as u32 - 1));
                        } else {
                            match reg!(*r) {
                                Rg::N(x) => out.push(Arg::N(x)),
                                Rg::S(x) => out.push(Arg::S(x)),
                                Rg::X(x) => {
                                    out.push(Arg::N(x));
                                    out.push(Arg::N(x + 1));
                                }
                                Rg::P => return Err("pointer passed by value".into()),
                            }
                        }
                    }
                    let dest = match dst {
                        Some(d) => Some(match reg!(*d) {
                            Rg::N(x) | Rg::S(x) | Rg::X(x) => x,
                            Rg::P => return Err("pointer result".into()),
                        }),
                        None => None,
                    };
                    self.m.calls.push(CallDesc { args: out, dest });
                    let desc = self.m.calls.len() as u32 - 1;
                    self.emit(Ins::Call { f: fi, desc });
                }
                Inst::Ret(v) => {
                    if k == 0 {
                        self.emit(Ins::End);
                        continue;
                    }
                    if let (Some(v), Some((slot, _))) = (v, ret) {
                        match (slot, reg!(*v)) {
                            (Slot::N(d), Rg::N(s)) => {
                                self.emit(Ins::Mov { d, s });
                            }
                            (Slot::S(d), Rg::S(s)) => {
                                self.emit(Ins::MovS { d, s });
                            }
                            (Slot::N(d), Rg::X(s)) => {
                                self.emit(Ins::MovX { d, s });
                            }
                            _ => return Err("bad return value".into()),
                        }
                    }
                    self.emit(Ins::Ret);
                }
                Inst::Rt { dst: Some(dst), name, args }
                    if matches!(name.as_str(), "LEN" | "LEFT$" | "RIGHT$" | "MID$" | "MID$.n" | "CHR$" | "ASC" | "ASC.n" | "INSTR" | "INSTR.n") =>
                {
                    // hot string builtins: dedicated instructions, same errors as the shared runtime
                    let native = match (name.as_str(), args.as_slice()) {
                        ("LEN", [x]) => Ins::LenS { d: nreg!(*dst), s: sreg!(*x) },
                        ("ASC", [x]) => Ins::Asc { d: nreg!(*dst), s: sreg!(*x) },
                        ("ASC.n", [x, i]) => Ins::AscAt { d: nreg!(*dst), s: sreg!(*x), i: nreg!(*i) },
                        ("INSTR", [x, y]) => Ins::Instr { d: nreg!(*dst), start: NONE, a: sreg!(*x), b: sreg!(*y) },
                        ("INSTR.n", [n, x, y]) => Ins::Instr { d: nreg!(*dst), start: nreg!(*n), a: sreg!(*x), b: sreg!(*y) },
                        ("CHR$", [x]) => Ins::Chr { d: sreg!(*dst), s: nreg!(*x) },
                        ("LEFT$", [x, n]) => Ins::Left { d: sreg!(*dst), s: sreg!(*x), n: nreg!(*n) },
                        ("RIGHT$", [x, n]) => Ins::Right { d: sreg!(*dst), s: sreg!(*x), n: nreg!(*n) },
                        ("MID$", [x, st]) => Ins::Mid { d: sreg!(*dst), s: sreg!(*x), start: nreg!(*st), n: NONE },
                        ("MID$.n", [x, st, n]) => Ins::Mid { d: sreg!(*dst), s: sreg!(*x), start: nreg!(*st), n: nreg!(*n) },
                        _ => return Err(format!("bad arguments for {name}")),
                    };
                    self.emit(native);
                }
                Inst::Rt { dst: None, name, args } if !self.mem && is_chain_rt(name) => {
                    // COMMON / CHAIN of a variable or array: no address needed, the
                    // instruction names the variable's place (no memory mode for it)
                    let ptr = args.first().copied().ok_or("CHAIN without a variable")?;
                    let (Some(Ptr::Place(pl)), Some(t)) = (ptrs[ptr as usize], ptr_ty[ptr as usize]) else {
                        return Err(format!("{name} of something that is not a variable"));
                    };
                    let k = ty_map(t).chain.ok_or_else(|| format!("{name} of a {} variable", t.name()))?;
                    let strs = name.ends_with('S');
                    let whole = matches!(name.as_str(), "CHAINPUTA" | "CHAINGETA" | "CHAINPUTAS" | "CHAINGETAS");
                    if strs != (k == ChainK::Str) || whole != matches!(pl, Place::A(_)) {
                        return Err(format!("{name} does not match its variable"));
                    }
                    self.m.places.push(pl);
                    let pl = self.m.places.len() as u32 - 1;
                    self.emit(Ins::Chain { pl, get: name.starts_with("CHAINGET"), k });
                }
                Inst::Rt { dst, name, args } => {
                    let b = ir::builtins::lookup(name).ok_or_else(|| format!("unknown builtin {name}"))?;
                    let mut regs_ = Vec::new();
                    for (r, t) in args.iter().zip(b.args) {
                        if *t == IT::Ptr {
                            if !self.mem {
                                return Err(format!("builtin {name} takes an address"));
                            }
                            regs_.push((nreg!(*r), Ty::I64));
                            continue;
                        }
                        if *t == IT::F80 {
                            regs_.push((xreg!(*r), Ty::F80));
                            continue;
                        }
                        let vt = vty(*t)?;
                        let x = match (reg!(*r), vt) {
                            (Rg::S(x), Ty::Str) => x,
                            (Rg::N(x), _) => x,
                            _ => return Err("bad builtin argument".into()),
                        };
                        regs_.push((x, if vt == Ty::Str { Ty::Str } else if t.is_float() { Ty::F64 } else { Ty::I64 }));
                    }
                    let rt = match b.ret {
                        None => None,
                        Some(t) => Some(if t == IT::Str {
                            Ty::Str
                        } else if t == IT::F80 {
                            Ty::F80
                        } else if t == IT::Ptr && self.mem {
                            Ty::I64
                        } else if t.is_float() {
                            vty(t)?
                        } else {
                            vty(t)?;
                            Ty::I64
                        }),
                    };
                    let d = match dst {
                        Some(d) => match reg!(*d) {
                            Rg::N(x) | Rg::S(x) | Rg::X(x) => x,
                            Rg::P => return Err("pointer result".into()),
                        },
                        None => NONE,
                    };
                    let f = match self.m.rtfns.iter().position(|(n, t)| *n == b.name && *t == rt) {
                        Some(f) => f as u32,
                        None => {
                            self.m.rtfns.push((b.name, rt));
                            self.m.rtfns.len() as u32 - 1
                        }
                    };
                    let d = if rt.is_none() { NONE } else { d };
                    self.m.bargs.push(regs_);
                    let args = self.m.bargs.len() as u32 - 1;
                    self.emit(Ins::Rt { f, d, args });
                }
                Inst::OnError(l) => match l {
                    None => {
                        self.emit(Ins::OnError { t: NONE });
                    }
                    Some(l) => {
                        let pc = self.emit(Ins::OnError { t: 0 });
                        self.fixups.push((pc, format!("0\u{1}{l}")));
                    }
                },
                Inst::OnTimer { slot, secs, target } => {
                    let (s, x) = (nreg!(*slot), nreg!(*secs));
                    let pc = self.emit(Ins::OnTimer { s, x, t: 0 });
                    self.fixups.push((pc, format!("0\u{1}{target}")));
                }
                Inst::OnKey { key, target } => {
                    let s = nreg!(*key);
                    let pc = self.emit(Ins::OnKey { s, t: 0 });
                    self.fixups.push((pc, format!("0\u{1}{target}")));
                }
                Inst::OnPlay { n, target } => {
                    let s = nreg!(*n);
                    let pc = self.emit(Ins::OnPlay { s, t: 0 });
                    self.fixups.push((pc, format!("0\u{1}{target}")));
                }
                Inst::OnStrig { n, ctl, target } => {
                    let (s, x) = (nreg!(*n), nreg!(*ctl));
                    let pc = self.emit(Ins::OnStrig { s, x, t: 0 });
                    self.fixups.push((pc, format!("0\u{1}{target}")));
                }
                Inst::Resume(r) => match r {
                    ResumeKind::Same => {
                        self.emit(Ins::Resume { k: ResumeK::Same, t: 0 });
                    }
                    ResumeKind::Next => {
                        self.emit(Ins::Resume { k: ResumeK::Next, t: 0 });
                    }
                    ResumeKind::Label(l) => {
                        let pc = self.emit(Ins::Resume { k: ResumeK::Label, t: 0 });
                        self.fixups.push((pc, format!("0\u{1}{l}")));
                    }
                },
                Inst::Error(c) => {
                    let s = nreg!(*c);
                    self.emit(Ins::Raise { s });
                }
                Inst::Read { ty, dst } => {
                    if *ty == IT::Str {
                        let d = sreg!(*dst);
                        self.emit(Ins::ReadS { d });
                    } else {
                        // QB type code of the target (subs_functions: bits | ISFLOAT | ISUNSIGNED)
                        use qb_runtime::numparse::{ISFLOAT, ISUNSIGNED};
                        let typ = match *ty {
                            IT::I8 => 8,
                            IT::U8 => 8 | ISUNSIGNED,
                            IT::I16 => 16,
                            IT::U16 => 16 | ISUNSIGNED,
                            IT::I32 => 32,
                            IT::U32 => 32 | ISUNSIGNED,
                            IT::I64 => 64,
                            IT::U64 => 64 | ISUNSIGNED,
                            IT::F32 => 32 | ISFLOAT,
                            _ => 64 | ISFLOAT,
                        };
                        let d = nreg!(*dst);
                        self.emit(Ins::ReadN { d, typ });
                    }
                }
                Inst::Restore(l) => {
                    let i = match l {
                        None => 0,
                        Some(l) => self.ir.data_labels.iter().find(|(n, _)| n == l).map(|(_, i)| *i).ok_or_else(|| format!("unknown data label {l}"))?,
                    };
                    self.emit(Ins::Restore { i: i as u32 });
                }
                Inst::End | Inst::Stop => {
                    self.emit(Ins::End);
                }
                Inst::System(c) => {
                    let s = match c {
                        Some(c) => nreg!(*c),
                        None => NONE,
                    };
                    self.emit(Ins::System { s });
                }
            }
        }
        // falling off the end = ret (end in main)
        self.emit(if k == 0 { Ins::End } else { Ins::Ret });
        self.globals = globals;
        Ok((params, ret))
    }

    /// Memory mode: the instructions on memory-backed variables and pointers.
    /// `Ok(false)`: `body[ii]` is lowered as in register mode.
    fn lower_mem(&mut self, k: usize, rg: &mut FnRegs, vars: &HashMap<&'a str, Var>, globals: &HashMap<String, Var>, body: &'a [Inst], ii: usize) -> Res<bool> {
        reg_macros!(self, rg, k);
        let ins = &body[ii];
        let mvar = |n: &String| match lookup_var(vars, globals, k, n) {
            Some(Var::M(p, mk)) => Some((p, mk)),
            _ => None,
        };
        match ins {
            Inst::Load { ty, dst, var } if mvar(var).is_some() => {
                let (p, mk) = mvar(var).unwrap();
                match mk {
                    MK::FStr(len) => {
                        let d = sreg!(*dst);
                        self.emit(Ins::LdFs { d, p, off: 0, len });
                    }
                    _ if *ty == IT::Str => {
                        if rg.single(*dst) && rg.rmap[*dst as usize].is_none() && p & (K | M) == 0 && str_read_only(body, ii, *dst) {
                            // its only reader takes the string from memory: no copy
                            rg.rmap[*dst as usize] = Some(Rg::S(p | M));
                        } else {
                            let d = sreg!(*dst);
                            self.emit(Ins::LdM { t: MTy::Str, d, p, off: 0 });
                        }
                    }
                    _ => {
                        let d = if *ty == IT::F80 { xreg!(*dst) } else { nreg!(*dst) };
                        self.emit(Ins::LdM { t: mty(*ty), d, p, off: 0 });
                    }
                }
                return Ok(true);
            }
            Inst::Store { ty, var, src } if mvar(var).is_some() => {
                let (p, mk) = mvar(var).unwrap();
                match mk {
                    MK::FStr(len) => {
                        let sr = sreg!(*src);
                        self.emit(Ins::StFs { p, off: 0, len, s: sr });
                    }
                    _ if *ty == IT::Str => {
                        let sr = sreg!(*src);
                        self.emit(Ins::StM { t: MTy::Str, p, off: 0, s: sr });
                    }
                    _ => {
                        let sr = if *ty == IT::F80 { xreg!(*src) } else { nreg!(*src) };
                        self.emit(Ins::StM { t: mty(*ty), p, off: 0, s: sr });
                    }
                }
                return Ok(true);
            }
            Inst::Addr { dst, var } | Inst::ARef { dst, var } => {
                let (p, _) = mvar(var).ok_or_else(|| format!("address of {var} without storage"))?;
                if rg.defs[*dst as usize] == 1 && rg.rmap[*dst as usize].is_none() {
                    rg.rmap[*dst as usize] = Some(Rg::N(p));
                } else {
                    let d = nreg!(*dst);
                    self.emit(Ins::Mov { d, s: p });
                }
                return Ok(true);
            }
            Inst::AAddr { dst, var, idx } => {
                let (p, _) = mvar(var).ok_or_else(|| format!("array {var} without storage"))?;
                let n = idx.len() as u32;
                let base = if n == 1 {
                    nreg!(idx[0])
                } else {
                    let base = self.frames[k].num_init.len() as R;
                    for _ in 0..n {
                        self.frames[k].new_n(0);
                    }
                    for (j, r) in idx.iter().enumerate() {
                        let s = nreg!(*r);
                        self.emit(Ins::Mov { d: base + j as R, s });
                    }
                    base
                };
                let d = nreg!(*dst);
                self.emit(Ins::AAddrM { d, a: p, i: base, n });
                return Ok(true);
            }
            Inst::Ld { ty, dst, ptr, off } => {
                let pr = nreg!(*ptr);
                let d = match ty {
                    IT::Str => sreg!(*dst),
                    IT::F80 => xreg!(*dst),
                    _ => nreg!(*dst),
                };
                self.emit(Ins::LdM { t: mty(*ty), d, p: pr, off: *off });
                return Ok(true);
            }
            Inst::St { ty, ptr, off, src } => {
                let pr = nreg!(*ptr);
                let sr = match ty {
                    IT::Str => sreg!(*src),
                    IT::F80 => xreg!(*src),
                    _ => nreg!(*src),
                };
                self.emit(Ins::StM { t: mty(*ty), p: pr, off: *off, s: sr });
                return Ok(true);
            }
            Inst::LdFs { dst, ptr, off, len } => {
                let (pr, d) = (nreg!(*ptr), sreg!(*dst));
                self.emit(Ins::LdFs { d, p: pr, off: *off, len: *len });
                return Ok(true);
            }
            Inst::StFs { ptr, off, len, src } => {
                let (pr, sr) = (nreg!(*ptr), sreg!(*src));
                self.emit(Ins::StFs { p: pr, off: *off, len: *len, s: sr });
                return Ok(true);
            }
            Inst::PAdd { dst, ptr, off } => {
                let (pr, o, d) = (nreg!(*ptr), nreg!(*off), nreg!(*dst));
                self.emit(Ins::PAddM { d, p: pr, o });
                return Ok(true);
            }
            Inst::Copy { dst, src, size } => {
                let (d, sr) = (nreg!(*dst), nreg!(*src));
                let n = *size + self.copy_run(k, rg, body, ii);
                self.emit(Ins::CopyM { d, s: sr, n });
                return Ok(true);
            }
            Inst::Dim { kind, var, bounds } => {
                let (p, _) = mvar(var).ok_or_else(|| format!("array {var} without storage"))?;
                let n = bounds.len() as u32;
                let base = self.frames[k].num_init.len() as R;
                for _ in 0..2 * n {
                    self.frames[k].new_n(0);
                }
                for (j, (lo, hi)) in bounds.iter().enumerate() {
                    let (lo, hi) = (nreg!(*lo), nreg!(*hi));
                    self.emit(Ins::Mov { d: base + 2 * j as R, s: lo });
                    self.emit(Ins::Mov { d: base + 2 * j as R + 1, s: hi });
                }
                let mode = match kind {
                    DimKind::Dim => DimMode::Dim,
                    DimKind::Static => DimMode::Static,
                    DimKind::Redim => DimMode::Redim,
                    DimKind::RedimPreserve => DimMode::RedimPreserve,
                };
                self.emit(Ins::DimM { a: p, dims: base, n, mode });
                return Ok(true);
            }
            Inst::Erase { var } => {
                let (p, _) = mvar(var).ok_or_else(|| format!("array {var} without storage"))?;
                self.emit(Ins::EraseM { a: p });
                return Ok(true);
            }
            Inst::LBound { dst, var, dim } | Inst::UBound { dst, var, dim } => {
                let (p, _) = mvar(var).ok_or_else(|| format!("array {var} without storage"))?;
                let (d, dm) = (nreg!(*dst), nreg!(*dim));
                self.emit(Ins::BoundM { d, a: p, dim: dm, upper: matches!(ins, Inst::UBound { .. }) });
                return Ok(true);
            }
            Inst::Call { dst, func, args } if self.callee(func).is_none() => {
                // DECLARE LIBRARY function (meta extfn): pointers are real addresses here
                let f = self.m.xfns.iter().position(|x| x.name == *func).ok_or_else(|| format!("unknown function {func}"))? as u32;
                let mut regs = Vec::new();
                for r in args {
                    regs.push(match reg!(*r) {
                        Rg::N(x) | Rg::S(x) => x,
                        _ => return Err("_FLOAT argument of a DECLARE LIBRARY function".into()),
                    });
                }
                let dest = match dst {
                    Some(d) => Some(match reg!(*d) {
                        Rg::N(x) | Rg::S(x) => x,
                        _ => return Err("_FLOAT result of a DECLARE LIBRARY function".into()),
                    }),
                    None => None,
                };
                self.m.xcalls.push(XCall { f, args: regs, dest });
                let c = self.m.xcalls.len() as u32 - 1;
                self.emit(Ins::XCall { c });
                return Ok(true);
            }
            Inst::Call { dst, func, args } => {
                let callee = self.callee(func).ok_or_else(|| format!("unknown function {func}"))?;
                let fi = self.fidx[func.as_str()] as u32;
                let mut out = Vec::new();
                for (r, _p) in args.iter().zip(&callee.params) {
                    match reg!(*r) {
                        Rg::N(x) => out.push(Arg::N(x)),
                        Rg::S(x) => out.push(Arg::S(x)),
                        Rg::X(x) => {
                            out.push(Arg::N(x));
                            out.push(Arg::N(x + 1));
                        }
                        Rg::P => return Err("pointer argument".into()),
                    }
                }
                let dest = match dst {
                    Some(d) => Some(match reg!(*d) {
                        Rg::N(x) | Rg::S(x) | Rg::X(x) => x,
                        Rg::P => return Err("pointer result".into()),
                    }),
                    None => None,
                };
                self.m.calls.push(CallDesc { args: out, dest });
                let desc = self.m.calls.len() as u32 - 1;
                self.emit(Ins::Call { f: fi, desc });
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }

    /// `_FLOAT` instructions (register pairs).
    fn lower_f80(&mut self, k: usize, rg: &mut FnRegs, vars: &HashMap<&'a str, Var>, globals: &HashMap<String, Var>, ins: &Inst) -> Res<()> {
        reg_macros!(self, rg, k);
        macro_rules! var {
            ($n:expr) => {
                lookup_var(vars, globals, k, $n.as_str()).ok_or_else(|| format!("unknown variable {}", $n))?
            };
        }
        match ins {
            Inst::Const { dst, val, .. } => {
                if rg.defs[*dst as usize] != 1 {
                    let d = xreg!(*dst);
                    let x = lit_f80(val);
                    for (j, bits) in [x.mant, u64::from(x.se)].into_iter().enumerate() {
                        let kk = self.m.nconst.len() as u32;
                        self.m.nconst.push(bits);
                        self.emit(Ins::Ldk { d: d + j as R, k: kk });
                    }
                }
            }
            Inst::Mov { dst, src, .. } => {
                let (d, sr) = (xreg!(*dst), xreg!(*src));
                self.emit(Ins::MovX { d, s: sr });
            }
            Inst::Bin { op, dst, a, b, .. } => {
                let (d, x, y) = (xreg!(*dst), xreg!(*a), xreg!(*b));
                let op = match op {
                    BinOp::Add => XOp::Add,
                    BinOp::Sub => XOp::Sub,
                    BinOp::Mul => XOp::Mul,
                    BinOp::Div => XOp::Div,
                    BinOp::Pow => XOp::Pow,
                    _ => return Err("bad _FLOAT op".into()),
                };
                self.emit(Ins::XBin { op, d, a: x, b: y });
            }
            Inst::Un { op: UnOp::Neg, dst, a, .. } => {
                let (d, sr) = (xreg!(*dst), xreg!(*a));
                self.emit(Ins::XNeg { d, s: sr });
            }
            Inst::Cmp { op, dst, a, b, .. } => {
                let (d, x, y) = (nreg!(*dst), xreg!(*a), xreg!(*b));
                self.emit(Ins::XCmp { c: cmp(*op), d, a: x, b: y });
            }
            Inst::Cvt { to, from, dst, src } | Inst::Trunc { to, from, dst, src } => {
                let trunc = matches!(ins, Inst::Trunc { .. });
                if *to == IT::F80 {
                    let d = xreg!(*dst);
                    if *from == IT::F80 {
                        let sr = xreg!(*src);
                        self.emit(Ins::MovX { d, s: sr });
                    } else if from.is_float() {
                        let sr = nreg!(*src);
                        self.emit(Ins::XFromF { d, s: sr });
                    } else {
                        let sr = nreg!(*src);
                        self.emit(Ins::XFromI { d, s: sr, u: *from == IT::U64 });
                    }
                } else {
                    let (d, sr) = (nreg!(*dst), xreg!(*src));
                    if to.is_float() {
                        self.emit(Ins::XToF { d, s: sr, f32: *to == IT::F32 });
                    } else {
                        self.emit(Ins::XToI { d, s: sr, w: width(*to)?, trunc });
                    }
                }
            }
            Inst::Load { dst, var, .. } => match var!(var) {
                Var::N(r, _) => {
                    let d = xreg!(*dst);
                    self.emit(Ins::MovX { d, s: r });
                }
                Var::NG(g, _) => {
                    let d = xreg!(*dst);
                    self.emit(Ins::GetG { d, g });
                    self.emit(Ins::GetG { d: d + 1, g: g + 1 });
                }
                Var::M(p, _) => {
                    let d = xreg!(*dst);
                    self.emit(Ins::LdM { t: MTy::F80, d, p, off: 0 });
                }
                _ => return Err("bad _FLOAT variable".into()),
            },
            Inst::Store { var, src, .. } => match var!(var) {
                Var::N(r, _) => {
                    let sr = xreg!(*src);
                    self.emit(Ins::MovX { d: r, s: sr });
                }
                Var::NG(g, _) => {
                    let sr = xreg!(*src);
                    self.emit(Ins::SetG { g, s: sr });
                    self.emit(Ins::SetG { g: g + 1, s: sr + 1 });
                }
                Var::M(p, _) => {
                    let sr = xreg!(*src);
                    self.emit(Ins::StM { t: MTy::F80, p, off: 0, s: sr });
                }
                _ => return Err("bad _FLOAT variable".into()),
            },
            Inst::Read { dst, .. } => {
                // func_read_float(.., FLOATTYPE): the double value
                let d = xreg!(*dst);
                let t = self.frames[k].new_n(0);
                self.emit(Ins::ReadN { d: t, typ: qb_runtime::numparse::ISFLOAT | 256 });
                self.emit(Ins::XFromF { d, s: t });
            }
            _ => return Err("unsupported _FLOAT instruction".into()),
        }
        Ok(())
    }

    /// Emits a function's entry code. A run of `NewV` of scalar variables becomes one
    /// block for all of them (`NewVA`, then a `NewVO` per further variable): every call
    /// creates the memory-backed variables of its frame, and one allocation instead
    /// of one per variable (BYREF arguments, TYPE locals, string temporaries) is most
    /// of the cost of a call in a program like the compiler.
    fn emit_prologue(&mut self, pro: Vec<Ins>) {
        let scalar = |m: &Module, i: &Ins| matches!(*i, Ins::NewV { v, .. } if !m.mvars[v as usize].array);
        let mut i = 0;
        while i < pro.len() {
            let mut j = i;
            while j < pro.len() && scalar(&self.m, &pro[j]) {
                j += 1;
            }
            if j - i < 2 {
                self.emit(pro[i]);
                i += 1;
                continue;
            }
            let mut off = 0u32;
            let mut place = Vec::new();
            let mut strs = Vec::new();
            for ins in &pro[i..j] {
                let Ins::NewV { d, v } = *ins else { unreachable!("scalar NewV") };
                let mv = self.m.mvars[v as usize];
                let size = mv.size.max(1);
                if mv.strs {
                    strs.push((off, off + size));
                }
                place.push((d, off));
                // 8-byte aligned, as the variables' own allocations were
                off = (off + size).next_multiple_of(8);
            }
            let g = self.m.vgroups.len() as u32;
            self.m.vgroups.push(VGroup { size: off, strs });
            for (n, &(d, o)) in place.iter().enumerate() {
                self.emit(if n == 0 { Ins::NewVA { d, g } } else { Ins::NewVO { d, off: o } });
            }
            i = j;
        }
    }

    /// Value of QIR register `r` when it is a constant register of function `k`.
    fn const_val(&self, k: usize, rg: &FnRegs, r: u32) -> Option<u64> {
        match rg.rmap.get(r as usize).copied().flatten() {
            Some(Rg::N(n)) if self.is_const(k, n) => self.frames[k].num_init.get(n as usize).copied(),
            _ => None,
        }
    }

    /// A TYPE assignment copies field by field: for each field, `padd` of the two
    /// variables' addresses and the field offset, then `copy` of its size. `body[at]`
    /// is such a copy; the copies of the next fields that continue it (the same two
    /// bases, offsets right after it, temporaries used only by their copy) are merged
    /// into it. Returns the bytes they add and sets `copy_skip` past them. String
    /// fields are not copied with `copy`, so a run stops before one (and at any gap).
    fn copy_run(&mut self, k: usize, rg: &FnRegs, body: &[Inst], at: usize) -> u32 {
        let Inst::Copy { dst, src, size } = body[at] else { return 0 };
        let single = |r: u32| rg.defs[r as usize] == 1 && rg.uses[r as usize] == 1;
        // the two `padd` that made this copy's addresses
        let padd = |i: usize, d: u32| match body.get(i) {
            Some(&Inst::PAdd { dst: pd, ptr, off }) if pd == d => Some((ptr, off)),
            _ => None,
        };
        let (Some(i1), Some(i2)) = (at.checked_sub(3), at.checked_sub(1)) else { return 0 };
        let (Some((pa, oa)), Some((pb, ob))) = (padd(i1, dst), padd(i2, src)) else { return 0 };
        if rg.defs[pa as usize] != 1 || rg.defs[pb as usize] != 1 {
            return 0;
        }
        let (Some(oa), Some(ob)) = (self.const_val(k, rg, oa), self.const_val(k, rg, ob)) else { return 0 };
        let mut total = u64::from(size);
        let mut j = at + 1;
        while let [Inst::Const { dst: ca, .. }, Inst::PAdd { dst: a, ptr: p1, off: o1 }, Inst::Const { dst: cb, .. }, Inst::PAdd { dst: b, ptr: p2, off: o2 }, Inst::Copy { dst: cd, src: cs, size: n }, ..] = &body[j..] {
            let ok = o1 == ca
                && o2 == cb
                && p1 == &pa
                && p2 == &pb
                && cd == a
                && cs == b
                && single(*a)
                && single(*b)
                && self.const_val(k, rg, *ca) == Some(oa.wrapping_add(total))
                && self.const_val(k, rg, *cb) == Some(ob.wrapping_add(total));
            if !ok || total + u64::from(*n) > u64::from(u32::MAX) {
                break;
            }
            total += u64::from(*n);
            j += 5;
        }
        if j == at + 1 {
            return 0;
        }
        self.copy_skip = j;
        (total - u64::from(size)) as u32
    }

    fn is_const(&self, k: usize, r: R) -> bool {
        self.frames[k].const_regs.contains(&r)
    }

    fn last_writes(&self, str_bank: bool, r: R) -> bool {
        if self.pc() <= self.barrier {
            return false;
        }
        let mut last = *self.m.code.last().unwrap();
        matches!(last.dest_mut(), Some((b, d)) if b == str_bank && *d == r)
    }

    fn retarget(&mut self, str_bank: bool, t: R, d: R) -> bool {
        if self.pc() <= self.barrier {
            return false;
        }
        if let Some(last) = self.m.code.last_mut()
            && let Some((b, dst)) = last.dest_mut()
                && b == str_bank && *dst == t {
                    *dst = d;
                    return true;
                }
        false
    }

    /// `v = v + (a op b)` (inner op into a temporary, add into `v`) -> `acc`.
    fn fuse_acc(&mut self, k: usize, v: R, once: impl Fn(R) -> bool) {
        let n = self.pc();
        if n < self.barrier + 2 {
            return;
        }
        let (last, prev) = (self.m.code[n as usize - 1], self.m.code[n as usize - 2]);
        let (wo, a, b) = match last {
            Ins::AddI16 { d, a, b } if d == v => (W::W16, a, b),
            Ins::AddI32 { d, a, b } if d == v => (W::W32, a, b),
            Ins::AddI64 { d, a, b } if d == v => (W::W64, a, b),
            _ => return,
        };
        let t = if a == v && b != v {
            b
        } else if b == v && a != v {
            a
        } else {
            return;
        };
        if t < self.frames[k].n_vars || self.is_const(k, t) || !once(t) {
            return;
        }
        let (op, x, y, wi) = match prev {
            Ins::MulI16 { d, a, b } if d == t => (AccOp::Mul, a, b, W::W16),
            Ins::MulI32 { d, a, b } if d == t => (AccOp::Mul, a, b, W::W32),
            Ins::MulI64 { d, a, b } if d == t => (AccOp::Mul, a, b, W::W64),
            Ins::IDiv { d, a, b, w } if d == t => (AccOp::IDiv, a, b, w),
            Ins::Mod { d, a, b, w } if d == t => (AccOp::Mod, a, b, w),
            Ins::And { d, a, b } if d == t => (AccOp::And, a, b, W::W64),
            Ins::Or { d, a, b } if d == t => (AccOp::Or, a, b, W::W64),
            Ins::Xor { d, a, b } if d == t => (AccOp::Xor, a, b, W::W64),
            _ => return,
        };
        // the temporary must not be read again (QIR registers are function-wide)
        let _ = k;
        self.m.code.truncate(n as usize - 2);
        self.emit(Ins::Acc { op, d: v, a: x, b: y, wi, wo });
    }

    /// Emits a jump to `key`; backward jumps become event safe points.
    fn jump(&mut self, ins: Ins, key: &str) {
        match self.labels.get(key).copied() {
            Some(t) if matches!(ins, Ins::Jmp { .. }) => {
                self.emit(Ins::Loop { t });
            }
            Some(t) => {
                let skip = self.pc() + 2;
                let mut inv = invert(ins);
                *inv.target_mut().unwrap() = skip;
                self.emit(inv);
                self.emit(Ins::Loop { t });
            }
            None => {
                let pc = self.emit(ins);
                self.fixups.push((pc, key.to_string()));
            }
        }
    }
}

/// Every reference to label `l` is a jump located strictly between `at` and `here`.
fn local_label(body: &[Inst], l: &str, at: usize, here: usize) -> bool {
    body.iter().enumerate().all(|(k, i)| {
        let refers = match i {
            Inst::Jmp(t) | Inst::Jz { target: t, .. } | Inst::Jnz { target: t, .. } => t == l,
            Inst::Gosub(t) | Inst::GRetL(t) => return t != l,
            Inst::OnError(Some(t)) | Inst::Resume(ResumeKind::Label(t)) | Inst::OnTimer { target: t, .. } | Inst::OnKey { target: t, .. } | Inst::OnPlay { target: t, .. } | Inst::OnStrig { target: t, .. } => return t != l,
            _ => false,
        };
        !refers || (k > at && k < here)
    })
}

/// Every value of integer type `from` has the same normalized bits in `to`.
fn widens(from: IT, to: IT) -> bool {
    use IT::*;
    from == to
        || matches!(
            (from, to),
            (I8, I16 | I32 | I64) | (I16, I32 | I64) | (I32, I64) | (U8, I16 | U16 | I32 | U32 | I64 | U64) | (U16, I32 | U32 | I64 | U64) | (U32, I64 | U64)
        )
}

/// The emitter's FOR test (direction chosen by the sign of STEP each
/// iteration) as one instruction:
/// `br ge step,0 -> P; br lt v,lim -> X; jmp B; P: br gt v,lim -> X; B:`
/// (floats: `br nlt step,0`, the same test for every STEP but NaN, which counts up)
fn fuse_for_tests(code: &mut [Ins], range: std::ops::Range<usize>, zero: &dyn Fn(R) -> bool) {
    for i in range.start..range.end.saturating_sub(4) {
        let w = &code[i..i + 4];
        let fused = match (w[0], w[1], w[2], w[3]) {
            (
                Ins::BrI { c: Cmp::Ge, a: step, b: z, t: p },
                Ins::BrI { c: Cmp::Lt, a: v, b: lim, t: x },
                Ins::Jmp { t: bb },
                Ins::BrI { c: Cmp::Gt, a: v2, b: lim2, t: x2 },
            ) if zero(z) && p as usize == i + 3 && bb as usize == i + 4 && v == v2 && lim == lim2 && x == x2 => {
                Some(Ins::ForTestKI { v, lim, step, t: x, skip: 4 })
            }
            (
                Ins::BrF { c: Cmp::Nlt, a: step, b: z, t: p },
                Ins::BrF { c: Cmp::Lt, a: v, b: lim, t: x },
                Ins::Jmp { t: bb },
                Ins::BrF { c: Cmp::Gt, a: v2, b: lim2, t: x2 },
            ) if zero(z) && p as usize == i + 3 && bb as usize == i + 4 && v == v2 && lim == lim2 && x == x2 => {
                Some(Ins::ForTestKF { v, lim, step, t: x, skip: 4 })
            }
            _ => None,
        };
        if let Some(f) = fused {
            code[i] = f;
        }
    }
    // back edge `loop t` where t: add.i64 c, step, iv; wrap iv, c; for_test.i +k c, lim, step -> exit
    for l in range.clone() {
        let Ins::Loop { t } = code[l] else { continue };
        let t = t as usize;
        if t + 2 >= range.end || l + 1 >= range.end {
            continue;
        }
        let (Ins::AddI64 { d: c, a, b }, Ins::Wrap { d: iv, s: c2, w }, Ins::ForTestKI { v, lim, step, t: exit, skip }) = (code[t], code[t + 1], code[t + 2]) else {
            continue;
        };
        let iv_ok = (a == step && b == iv) || (b == step && a == iv);
        if c == c2 && v == c && iv_ok && step == lim + 1 && exit as usize == l + 1 && c != iv && c != lim && c != step && iv != lim && iv != step {
            code[l] = Ins::LoopForI { c, iv, lim, body: (t + 2) as u32 + u32::from(skip), w };
        }
    }
}

/// A variable as function `k` sees it: its own names (statics, parameters,
/// locals) first, then the module-level variables.
fn lookup_var(own: &HashMap<&str, Var>, globals: &HashMap<String, Var>, k: usize, n: &str) -> Option<Var> {
    own.get(n).copied().or_else(|| globals.get(n).map(|&v| if k == 0 { v } else { to_global(v) }))
}

fn to_global(v: Var) -> Var {
    match v {
        Var::N(r, t) => Var::NG(r, t),
        Var::S(r) => Var::SG(r),
        Var::A(r, t) => Var::A(r | G, t),
        Var::M(r, mk) => Var::MG(r, mk),
        v => v,
    }
}

/// `load %d $x` may reuse `$x`'s register when every use of `%d` comes
/// before anything that could change `$x` (store, call, control flow).
/// The string loaded into `d` at `at` is read once, by LEN, ASC, LEFT$, RIGHT$, MID$ or a
/// string comparison, before anything could change memory: that reader can take the
/// string from memory instead of a copy.
fn str_read_only(body: &[Inst], at: usize, d: u32) -> bool {
    let Some(reader) = body[at + 1..].iter().find(|i| regs(i).1.contains(&d)) else { return false };
    let ok = match reader {
        Inst::Rt { dst: Some(_), name, args } => match name.as_str() {
            "LEN" | "ASC" | "ASC.n" | "LEFT$" | "RIGHT$" | "MID$" | "MID$.n" => args.first() == Some(&d) && !args[1..].contains(&d),
            "INSTR" => args.iter().filter(|&&a| a == d).count() == 1,
            "INSTR.n" => args.first() != Some(&d) && args.iter().filter(|&&a| a == d).count() == 1,
            _ => false,
        },
        Inst::Cmp { ty: IT::Str, a, b, .. } => (*a == d) != (*b == d),
        _ => false,
    };
    ok && stable_until_uses(body, at, d, 1, &|i: &Inst| match i {
        Inst::Store { .. } | Inst::St { .. } | Inst::StFs { .. } | Inst::Copy { .. } | Inst::Read { .. } => true,
        // a builtin that only computes a value cannot change the string (CHR$(13) as
        // INSTR's other argument: INSTR(p, buffer$, CHR$(13)) read the whole buffer)
        Inst::Rt { name, .. } => !rt_writes_nothing(name),
        _ => false,
    })
}

/// `body[at]` loads the module-level string `var` into a register that only starts
/// `var = var + a [+ b]`: up to two `concat`s, each the only use of the value before
/// it, then the store of the last one to `var`. In between only loads of other
/// variables, constants and builtins that write nothing (the parts). Returns the
/// indices of the concats and of the store, and the parts.
fn app_chain(body: &[Inst], at: usize, var: &str, single: &dyn Fn(u32) -> bool) -> Option<(Vec<usize>, u32, Option<u32>)> {
    let Inst::Load { dst, .. } = &body[at] else { return None };
    let mut cur = *dst;
    let mut parts = Vec::new();
    let mut steps = Vec::new();
    for (j, i) in body.iter().enumerate().skip(at + 1) {
        match i {
            Inst::Concat { dst, a, b } if *a == cur && *b != cur && parts.len() < 2 && single(cur) => {
                parts.push(*b);
                steps.push(j);
                cur = *dst;
            }
            Inst::Store { var: v, src, .. } if v == var && *src == cur && !parts.is_empty() && single(cur) => {
                steps.push(j);
                return Some((steps, parts[0], parts.get(1).copied()));
            }
            Inst::Load { var: v, .. } if v != var => {}
            Inst::Const { .. } => {}
            Inst::Rt { name, .. } if rt_writes_nothing(name) => {}
            _ => return None,
        }
    }
    None
}

/// Builtins that write no variable or memory: they only read their arguments and
/// return a value (the VM's own string instructions and the [`crate::rt::builtins::Pure`] ones).
fn rt_writes_nothing(name: &str) -> bool {
    matches!(name, "LEN" | "ASC" | "ASC.n" | "LEFT$" | "RIGHT$" | "MID$" | "MID$.n" | "CHR$" | "INSTR" | "INSTR.n") || crate::rt::builtins::pure(name).is_some()
}

fn alias_ok(body: &[Inst], at: usize, d: u32, var: &str, total: u32) -> bool {
    // (COMMON writes the variable without a store)
    stable_until_uses(body, at, d, total, &|i: &Inst| {
        matches!(i, Inst::Store { var: v, .. } if v == var) || matches!(i, Inst::Rt { name, .. } if is_chain_rt(name) && name.starts_with("CHAINGET"))
    })
}

/// The builtins of COMMON (CHAINGET...) and CHAIN (CHAINPUT...), scalar and array forms.
fn is_chain_rt(name: &str) -> bool {
    matches!(name, "CHAINPUT" | "CHAINPUTS" | "CHAINGET" | "CHAINGETS" | "CHAINPUTA" | "CHAINPUTAS" | "CHAINGETA" | "CHAINGETAS")
}

/// Every use of `d` (defined at `at`) comes, in straight-line order, before
/// any instruction for which `kill` holds and before any merge point
/// (label), call or GOSUB. Jumps *out* of the run are fine: a path that leaves
/// cannot come back without passing a label.
fn stable_until_uses(body: &[Inst], at: usize, d: u32, total: u32, kill: &dyn Fn(&Inst) -> bool) -> bool {
    let mut seen = 0;
    if total == 0 {
        return true;
    }
    for (j, i) in body.iter().enumerate().skip(at + 1) {
        let (_, us) = regs(i);
        seen += us.count(d);
        if seen >= total {
            return true;
        }
        if kill(i) {
            return false;
        }
        match i {
            // a merge point is harmless when every jump to it comes from inside the run
            Inst::Label(l) if local_label(body, l, at, j) => {}
            Inst::Label(_)
            | Inst::Gosub(_)
            | Inst::GRet
            | Inst::GRetL(_)
            | Inst::Call { .. }
            | Inst::Ret(_)
            | Inst::Resume(_)
            | Inst::Error(_)
            | Inst::End
            | Inst::Stop
            | Inst::System(_)
            | Inst::Dim { .. }
            | Inst::St { .. } => return false,
            _ => {}
        }
    }
    false
}

/// Number of reads of register `r` (defined at `at`) before it is defined again.
fn uses_before_redef(body: &[Inst], at: usize, r: u32) -> u32 {
    let mut n = 0;
    for i in &body[at + 1..] {
        let (d, us) = regs(i);
        n += us.count(r);
        if d == Some(r) {
            break;
        }
    }
    n
}

/// Whether register `r` (defined at `at`) is read before it is defined again.
fn used_before_redef(body: &[Inst], at: usize, r: u32) -> bool {
    for i in &body[at + 1..] {
        let (d, us) = regs(i);
        if us.contains(&r) {
            return true;
        }
        if d == Some(r) {
            return false;
        }
    }
    false
}

fn invert(i: Ins) -> Ins {
    match i {
        Ins::BrI { c, a, b, t } => Ins::BrI { c: c.inverse(), a, b, t },
        Ins::BrF { c, a, b, t } => Ins::BrF { c: c.negated(), a, b, t },
        Ins::BrS { c, a, b, t } => Ins::BrS { c: c.inverse(), a, b, t },
        Ins::BrU { c, a, b, t } => Ins::BrU { c: c.inverse(), a, b, t },
        Ins::JzI { s, t } => Ins::JnzI { s, t },
        Ins::JnzI { s, t } => Ins::JzI { s, t },
        other => other,
    }
}

/// `meta extfn <name> <lib|-> <ret> <arg types...>` (emitter: DECLARE LIBRARY).
/// The native call of a DECLARE LIBRARY function (`None` for a `_FLOAT`
/// argument or result, which no platform calling convention here passes).
fn xfn(f: &ir::ExtFn) -> Option<XFn> {
    let ty = |t: IT| ty_map(t).x;
    let ret = match f.ret {
        None => XTy::Void,
        Some(t) => ty(t)?,
    };
    let args = f.args.iter().map(|&t| ty(t)).collect::<Option<Vec<_>>>()?;
    let gl = f.lib == "opengl";
    Some(XFn { name: f.name.clone(), gl, lib: if gl { f.lib.clone() } else { xfn_lib(&f.lib)? }, args, ret })
}

/// Library spec of `meta extfn`: `-` = none (default system libraries), else the
/// `DECLARE LIBRARY "spec"` text as hex (`x` + two hex digits per byte), since
/// a path may contain spaces.
fn xfn_lib(t: &str) -> Option<String> {
    if t == "-" {
        return Some(String::new());
    }
    let h = t.strip_prefix('x')?;
    if h.len() % 2 != 0 || !h.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let bytes = (0..h.len()).step_by(2).map(|i| u8::from_str_radix(h.get(i..i + 2)?, 16).ok()).collect::<Option<Vec<u8>>>()?;
    String::from_utf8(bytes).ok()
}

//! Loader for QIR v1 (`docs/rust-port/qir-v1.md`, crate `qb-ir`) onto the
//! typed register VM.
//!
//! QIR v1 is already typed and register based; what this pass adds:
//!
//! - **Variables become registers.** Scalars (locals, byval params, globals,
//!   statics) live in registers: globals and statics in the main frame (SUBs
//!   reach them with `getg`/`setg`), BYREF params are copied in/out. A
//!   `load` is usually an alias of the variable's register (no instruction),
//!   a `store` usually retargets the instruction that produced the value.
//! - **Addresses are symbolic.** `addr`/`aref`/`aaddr` produce no value: a
//!   pointer register remembers *what* it points at, and the only uses
//!   accepted are a call argument (BYREF / array parameter) and `ld`/`st` at
//!   offset 0 of an array element (`lde1`/`ste1`, ...).
//! - **Constants are registers** of the frame template (no `const` executed).
//! - The same fusions as the v0 loader (compare + branch, `acc`, backward
//!   jumps as event safe points), never across `line` or `label`.
//!
//! Programs that need real memory (UDT, `STRING * n`, pointer arithmetic,
//! GET/PUT of variables) or `_UNSIGNED _INTEGER64` are refused with a reason;
//! the caller then runs the R11a reference interpreter (`v1.rs`).

use std::collections::HashMap;

use crate::bytecode::*;
use qb_ir::{self as ir, BinOp, CmpOp, DimKind, Inst, Lit, ResumeKind, StoreTy, Ty as IT, UnOp, VarKind};

type Res<T> = Result<T, String>;

fn width(t: IT) -> Res<W> {
    Ok(match t {
        IT::I8 => W::W8,
        IT::U8 => W::U8,
        IT::I16 => W::W16,
        IT::U16 => W::U16,
        IT::I32 => W::W32,
        IT::U32 => W::U32,
        IT::I64 => W::W64,
        _ => return Err(format!("no integer width for {}", t.name())),
    })
}

/// VM type used for array storage and builtin arguments.
fn vty(t: IT) -> Res<Ty> {
    Ok(match t {
        IT::I8 | IT::U8 | IT::I16 => Ty::I16,
        IT::U16 | IT::I32 => Ty::I32,
        IT::U32 | IT::I64 => Ty::I64,
        IT::F32 => Ty::F32,
        IT::F64 | IT::F80 => Ty::F64,
        IT::Str => Ty::Str,
        IT::U64 => return Err("_UNSIGNED _INTEGER64".into()),
        IT::Ptr => return Err("pointer value".into()),
    })
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

/// Register defined and registers read by an instruction.
fn regs(i: &Inst) -> (Option<u32>, Vec<u32>) {
    use Inst::*;
    match i {
        Line(_) | Label(_) | Jmp(_) | Gosub(_) | GRet | GRetL(_) | OnError(_) | Resume(_) | Restore(_) | End | Stop => {
            (None, vec![])
        }
        Const { dst, .. } => (Some(*dst), vec![]),
        Mov { dst, src, .. } | Cvt { dst, src, .. } | Trunc { dst, src, .. } => (Some(*dst), vec![*src]),
        Bin { dst, a, b, .. } | Cmp { dst, a, b, .. } | Concat { dst, a, b } => (Some(*dst), vec![*a, *b]),
        Un { dst, a, .. } => (Some(*dst), vec![*a]),
        Load { dst, .. } | Addr { dst, .. } | ARef { dst, .. } => (Some(*dst), vec![]),
        Store { src, .. } => (None, vec![*src]),
        AAddr { dst, idx, .. } => (Some(*dst), idx.clone()),
        Ld { dst, ptr, .. } | LdFs { dst, ptr, .. } => (Some(*dst), vec![*ptr]),
        St { ptr, src, .. } | StFs { ptr, src, .. } => (None, vec![*ptr, *src]),
        PAdd { dst, ptr, off } => (Some(*dst), vec![*ptr, *off]),
        Copy { dst, src, .. } => (None, vec![*dst, *src]),
        Dim { bounds, .. } => (None, bounds.iter().flat_map(|(a, b)| [*a, *b]).collect()),
        Erase { .. } => (None, vec![]),
        LBound { dst, dim, .. } | UBound { dst, dim, .. } => (Some(*dst), vec![*dim]),
        Jz { c, .. } | Jnz { c, .. } | Error(c) => (None, vec![*c]),
        Call { dst, args, .. } => (*dst, args.clone()),
        Ret(v) => (None, v.iter().copied().collect()),
        Rt { dst, args, .. } => (*dst, args.clone()),
        Read { dst, .. } => (Some(*dst), vec![]),
        System(c) => (None, c.iter().copied().collect()),
    }
}

/// Register type written by an instruction (None = no register result).
fn def_ty(m: &ir::Module, i: &Inst) -> Option<IT> {
    use Inst::*;
    Some(match i {
        Const { ty, .. } | Mov { ty, .. } | Bin { ty, .. } | Un { ty, .. } | Load { ty, .. } | Ld { ty, .. } | Read { ty, .. } => *ty,
        Cmp { .. } | LBound { .. } | UBound { .. } => IT::I32,
        Cvt { to, .. } | Trunc { to, .. } => *to,
        Concat { .. } | LdFs { .. } => IT::Str,
        Addr { .. } | ARef { .. } | AAddr { .. } | PAdd { .. } => IT::Ptr,
        Call { dst: Some(_), func, .. } => m.func(func)?.ret?,
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
}

#[derive(Clone, Copy, Debug)]
enum Rg {
    N(R),
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
    /// first numeric register that is not a variable (temporaries, v1 registers, constants)
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
        r
    }
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

fn check_store_ty(t: &StoreTy) -> Res<IT> {
    match t {
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
    /// module-level variables as seen from main (registers of frame 0)
    globals: HashMap<String, Var>,
    frames: Vec<Frame>,
}

/// Lowers a QIR v1 module, or says why it cannot run on the typed VM.
pub fn compile_v1(src: &ir::Module) -> Res<Module> {
    ir::validate(src).map_err(|e| e.to_string())?;
    if !src.types.is_empty() {
        return Err("TYPE (UDT)".into());
    }
    let main_i = src.funcs.iter().position(|f| f.name == "main").ok_or("no main")?;
    // function order: main first (frame 0 holds the globals)
    let mut order = vec![main_i];
    order.extend((0..src.funcs.len()).filter(|&i| i != main_i));
    let fidx: HashMap<&str, usize> = order.iter().enumerate().map(|(k, &i)| (src.funcs[i].name.as_str(), k)).collect();
    let mut lw = Lw {
        ir: src,
        m: Module { data: src.data.clone(), v1: true, ..Module::default() },
        kstr: HashMap::new(),
        labels: HashMap::new(),
        fixups: Vec::new(),
        barrier: 0,
        fidx,
        globals: HashMap::new(),
        frames: (0..order.len()).map(|_| Frame::default()).collect(),
    };
    // module-level storage in frame 0: globals, then every function's statics
    for g in &src.globals {
        let v = lw.alloc_var(0, g)?;
        lw.globals.insert(g.name.clone(), v);
    }
    let mut statics: Vec<HashMap<String, Var>> = vec![HashMap::new(); order.len()];
    for (k, &fi) in order.iter().enumerate() {
        for l in src.funcs[fi].locals.iter().filter(|l| l.kind == VarKind::Static) {
            let v = lw.alloc_var(0, l)?;
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
            is_static: false,
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
    fn alloc_var(&mut self, k: usize, d: &ir::VarDecl) -> Res<Var> {
        let t = check_store_ty(&d.ty)?;
        let fr = &mut self.frames[k];
        Ok(if d.array {
            fr.arr_tys.push(vty(t)?);
            Var::A(fr.arr_tys.len() as R - 1, t)
        } else if t == IT::Str {
            Var::S(fr.new_s())
        } else {
            Var::N(fr.new_n(0), t)
        })
    }

    fn function(&mut self, k: usize, f: &'a ir::Func, statics: &HashMap<String, Var>) -> Res<(Vec<Slot>, Option<(Slot, Ty)>)> {
        let fkey = |l: &str| format!("{k}\u{1}{l}");
        // variables: globals (module view), statics, params, locals
        let mut vars: HashMap<String, Var> = HashMap::new();
        for (n, v) in &self.globals {
            vars.insert(n.clone(), if k == 0 { *v } else { to_global(*v) });
        }
        for (n, v) in statics {
            vars.insert(n.clone(), to_global(*v));
        }
        let mut params = Vec::new();
        for p in &f.params {
            let v = self.alloc_var(k, p)?;
            params.push(match v {
                Var::N(r, _) => Slot::N(r),
                Var::S(r) => Slot::S(r),
                Var::A(r, _) => Slot::A(r),
                _ => unreachable!(),
            });
            vars.insert(p.name.clone(), v);
        }
        for l in f.locals.iter().filter(|l| l.kind != VarKind::Static) {
            let v = self.alloc_var(k, l)?;
            vars.insert(l.name.clone(), v);
        }
        let ret = match f.ret {
            None => None,
            Some(IT::Str) => Some((Slot::S(self.frames[k].new_s()), Ty::Str)),
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
        // register types and use counts
        let mut rty: HashMap<u32, IT> = HashMap::new();
        let mut defs: HashMap<u32, u32> = HashMap::new();
        let mut uses: HashMap<u32, u32> = HashMap::new();
        for i in &f.body {
            let (d, us) = regs(i);
            if let Some(d) = d {
                *defs.entry(d).or_default() += 1;
                if let Some(t) = def_ty(self.ir, i) {
                    rty.insert(d, t);
                }
            }
            for u in us {
                *uses.entry(u).or_default() += 1;
            }
        }
        // v1 registers: constants defined once become constant registers / K strings
        let mut rmap: HashMap<u32, Rg> = HashMap::new();
        for i in &f.body {
            if let Inst::Const { ty, dst, val } = i {
                if defs.get(dst) == Some(&1) && !rmap.contains_key(dst) {
                    let rg = if *ty == IT::Str {
                        let Lit::Str(s) = val else { return Err("bad string const".into()) };
                        Rg::S(self.kstr(s))
                    } else if *ty == IT::Ptr {
                        return Err("pointer constant".into());
                    } else {
                        vty(*ty)?;
                        let bits = lit_num(*ty, val)?;
                        Rg::N(self.frames[k].konst(bits, ty.is_float()))
                    };
                    rmap.insert(*dst, rg);
                }
            }
        }
        let mut ptrs: HashMap<u32, Ptr> = HashMap::new();
        // v1 register -> variable whose register it aliases
        let mut alias_var: HashMap<u32, &str> = HashMap::new();
        self.barrier = self.pc();
        let body = &f.body;
        for (ii, ins) in body.iter().enumerate() {
            // register of a v1 register (allocated on first sight)
            macro_rules! reg {
                ($r:expr) => {{
                    let r: u32 = $r;
                    match rmap.get(&r) {
                        Some(x) => *x,
                        None => {
                            let t = *rty.get(&r).ok_or_else(|| format!("register %{r} has no type"))?;
                            let x = match t {
                                IT::Str => Rg::S(self.frames[k].new_s()),
                                IT::Ptr => Rg::P,
                                _ => {
                                    vty(t)?;
                                    Rg::N(self.frames[k].new_n(0))
                                }
                            };
                            rmap.insert(r, x);
                            x
                        }
                    }
                }};
            }
            macro_rules! nreg {
                ($r:expr) => {
                    match reg!($r) {
                        Rg::N(x) => x,
                        _ => return Err(format!("%{} is not numeric", $r)),
                    }
                };
            }
            macro_rules! sreg {
                ($r:expr) => {
                    match reg!($r) {
                        Rg::S(x) => x,
                        _ => return Err(format!("%{} is not a string", $r)),
                    }
                };
            }
            macro_rules! var {
                ($n:expr) => {
                    *vars.get($n.as_str()).ok_or_else(|| format!("unknown variable {}", $n))?
                };
            }
            let single = |r: u32| defs.get(&r) == Some(&1) && uses.get(&r) == Some(&1);
            match ins {
                Inst::Line(n) => {
                    let pc = self.pc();
                    self.m.stmts.push((pc, *n));
                    self.barrier = pc;
                }
                Inst::Label(l) => {
                    let pc = self.pc();
                    if self.labels.insert(fkey(l), pc).is_some() {
                        return Err(format!("duplicate label {l}"));
                    }
                    self.barrier = pc;
                }
                Inst::Const { ty, dst, val } => {
                    if defs.get(dst) == Some(&1) {
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
                        Rg::P => return Err("pointer constant".into()),
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
                        self.emit(if ty.is_float() { Ins::CmpF { c, d, a: x, b: y } } else { Ins::CmpI { c, d, a: x, b: y } });
                    }
                }
                Inst::Cvt { to, from, dst, src } | Inst::Trunc { to, from, dst, src } => {
                    let trunc = matches!(ins, Inst::Trunc { .. });
                    if to.is_int() && from.is_int() && widens(*from, *to) && defs.get(dst) == Some(&1) && !rmap.contains_key(dst) {
                        // widening an already normalized integer changes nothing: reuse the register
                        let s = nreg!(*src);
                        let n_uses = uses.get(dst).copied().unwrap_or(0);
                        let ok = match alias_var.get(src).copied() {
                            Some(v) => alias_ok(body, ii, *dst, v, n_uses),
                            None => stable_until_uses(body, ii, *dst, n_uses, &|i: &Inst| regs(i).0 == Some(*src)),
                        };
                        if ok {
                            rmap.insert(*dst, Rg::N(s));
                            if let Some(v) = alias_var.get(src).copied() {
                                alias_var.insert(*dst, v);
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
                            self.emit(Ins::IToF { d, s });
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
                            let alias = defs.get(dst) == Some(&1) && !rmap.contains_key(dst) && alias_ok(body, ii, *dst, var, uses.get(dst).copied().unwrap_or(0));
                            let is_s = matches!(v, Var::S(_));
                            if alias {
                                rmap.insert(*dst, if is_s { Rg::S(r) } else { Rg::N(r) });
                                alias_var.insert(*dst, var.as_str());
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
                        Var::SG(g) => {
                            let d = sreg!(*dst);
                            self.emit(Ins::GetGS { d, g });
                        }
                        Var::A(..) => return Err("load of an array".into()),
                    }
                    let _ = ty;
                }
                Inst::Store { ty, var, src } => {
                    let v = var!(var);
                    match v {
                        Var::N(r, t) => {
                            let s = nreg!(*src);
                            let produced = single(*src) && s >= self.frames[k].n_vars && !self.is_const(k, s) && self.last_writes(false, s);
                            if *ty == IT::F32 && !(produced && matches!(self.m.code.last(), Some(Ins::RoundF32 { .. }))) && !self.is_const(k, s) {
                                self.emit(Ins::RoundF32 { d: r, s });
                            } else if s != r && !(produced && self.retarget(false, s, r)) {
                                self.emit(Ins::Mov { d: r, s });
                            }
                            if t.is_int() {
                                let once = |x: R| rmap.iter().any(|(v, g)| matches!(g, Rg::N(y) if *y == x) && single(*v));
                                self.fuse_acc(k, r, once);
                            }
                        }
                        Var::S(r) => {
                            let s = sreg!(*src);
                            if s != r {
                                let produced = single(*src) && s & K == 0 && s >= self.frames[k].n_svars && self.last_writes(true, s);
                                if !(produced && self.retarget(true, s, r)) {
                                    let tmp = s & K == 0 && s >= self.frames[k].n_svars && single(*src);
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
                        Var::A(..) => return Err("store to an array".into()),
                    }
                }
                Inst::Addr { dst, var } => {
                    let p = match var!(var) {
                        Var::N(r, t) => Place::N(r, vty(t)?),
                        Var::NG(g, t) => Place::NG(g, vty(t)?),
                        Var::S(r) => Place::S(r),
                        Var::SG(g) => Place::SG(g),
                        Var::A(..) => return Err("addr of an array".into()),
                    };
                    reg!(*dst);
                    ptrs.insert(*dst, Ptr::Place(p));
                }
                Inst::ARef { dst, var } => {
                    let Var::A(a, _) = var!(var) else { return Err("aref of a scalar".into()) };
                    reg!(*dst);
                    ptrs.insert(*dst, Ptr::Place(Place::A(a)));
                }
                Inst::AAddr { dst, var, idx } => {
                    let Var::A(a, ety) = var!(var) else { return Err("aaddr of a scalar".into()) };
                    let n = idx.len() as u32;
                    // one subscript that stays unchanged until the address is used: read it in place
                    let direct = match idx.as_slice() {
                        [x] => {
                            let x = *x;
                            let cnt = uses_before_redef(body, ii, *dst);
                            let var = alias_var.get(&x).copied();
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
                    ptrs.insert(*dst, Ptr::Elem { a, i: base, n, ety });
                    if !used_before_redef(body, ii, *dst) {
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
                    let Some(Ptr::Elem { a, i, n, ety }) = ptrs.get(ptr).copied() else {
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
                    if single(*c) && self.pc() > self.barrier {
                        let fused = match self.m.code.last().copied() {
                            Some(Ins::CmpI { c: op, d, a, b }) if d == r => Some(Ins::BrI { c: if on_true { op } else { op.not() }, a, b, t: 0 }),
                            Some(Ins::CmpF { c: op, d, a, b }) if d == r => Some(Ins::BrF { c: if on_true { op } else { op.not() }, a, b, t: 0 }),
                            Some(Ins::CmpS { c: op, d, a, b }) if d == r => Some(Ins::BrS { c: if on_true { op } else { op.not() }, a, b, t: 0 }),
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
                Inst::Call { dst, func, args } => {
                    let callee = self.ir.func(func).ok_or_else(|| format!("unknown function {func}"))?;
                    let fi = self.fidx[func.as_str()] as u32;
                    let mut out = Vec::new();
                    for (r, p) in args.iter().zip(&callee.params) {
                        let byref = matches!(p.kind, VarKind::Param { byref: true }) || p.array;
                        if byref {
                            let pl = match ptrs.get(r).copied() {
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
                                Rg::P => return Err("pointer passed by value".into()),
                            }
                        }
                    }
                    let dest = match dst {
                        Some(d) => Some(match reg!(*d) {
                            Rg::N(x) | Rg::S(x) => x,
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
                            _ => return Err("bad return value".into()),
                        }
                    }
                    self.emit(Ins::Ret);
                }
                Inst::Rt { dst: Some(dst), name, args }
                    if matches!(name.as_str(), "LEN" | "LEFT$" | "RIGHT$" | "MID$" | "MID$.n" | "CHR$" | "ASC") =>
                {
                    // hot string builtins: dedicated instructions, same errors as the shared runtime
                    let native = match (name.as_str(), args.as_slice()) {
                        ("LEN", [x]) => Ins::LenS { d: nreg!(*dst), s: sreg!(*x) },
                        ("ASC", [x]) => Ins::Asc { d: nreg!(*dst), s: sreg!(*x) },
                        ("CHR$", [x]) => Ins::Chr { d: sreg!(*dst), s: nreg!(*x) },
                        ("LEFT$", [x, n]) => Ins::Left { d: sreg!(*dst), s: sreg!(*x), n: nreg!(*n) },
                        ("RIGHT$", [x, n]) => Ins::Right { d: sreg!(*dst), s: sreg!(*x), n: nreg!(*n) },
                        ("MID$", [x, st]) => Ins::Mid { d: sreg!(*dst), s: sreg!(*x), start: nreg!(*st), n: NONE },
                        ("MID$.n", [x, st, n]) => Ins::Mid { d: sreg!(*dst), s: sreg!(*x), start: nreg!(*st), n: nreg!(*n) },
                        _ => return Err(format!("bad arguments for {name}")),
                    };
                    self.emit(native);
                }
                Inst::Rt { dst, name, args } => {
                    let b = ir::builtins::lookup(name).ok_or_else(|| format!("unknown builtin {name}"))?;
                    let mut regs_ = Vec::new();
                    for (r, t) in args.iter().zip(b.args) {
                        if *t == IT::Ptr {
                            return Err(format!("builtin {name} takes an address"));
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
                        } else if t.is_float() {
                            vty(t)?
                        } else {
                            vty(t)?;
                            Ty::I64
                        }),
                    };
                    let d = match dst {
                        Some(d) => match reg!(*d) {
                            Rg::N(x) | Rg::S(x) => x,
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
                        self.emit(Ins::Read { d, str: true });
                    } else {
                        let d = nreg!(*dst);
                        self.emit(Ins::ReadN { d });
                        if ty.is_float() {
                            if *ty == IT::F32 {
                                self.emit(Ins::RoundF32 { d, s: d });
                            }
                        } else {
                            self.emit(Ins::CvtFI { d, s: d, w: width(*ty)?, trunc: false });
                        }
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
        Ok((params, ret))
    }

    fn is_const(&self, k: usize, r: R) -> bool {
        self.frames[k].consts.values().any(|&c| c == r)
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
        if let Some(last) = self.m.code.last_mut() {
            if let Some((b, dst)) = last.dest_mut() {
                if b == str_bank && *dst == t {
                    *dst = d;
                    return true;
                }
            }
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
        // the temporary must not be read again (v1 registers are function-wide)
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
            Inst::OnError(Some(t)) | Inst::Resume(ResumeKind::Label(t)) => return t != l,
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
            (I8, I16 | I32 | I64) | (I16, I32 | I64) | (I32, I64) | (U8, I16 | U16 | I32 | U32 | I64) | (U16, I32 | U32 | I64) | (U32, I64)
        )
}

/// The emitter's FOR test (direction chosen by the sign of STEP each
/// iteration) as one instruction:
/// `br ge step,0 -> P; br lt v,lim -> X; jmp B; P: br gt v,lim -> X; B:`
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
                Ins::BrF { c: Cmp::Ge, a: step, b: z, t: p },
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

fn to_global(v: Var) -> Var {
    match v {
        Var::N(r, t) => Var::NG(r, t),
        Var::S(r) => Var::SG(r),
        Var::A(r, t) => Var::A(r | G, t),
        v => v,
    }
}

/// `load %d $x` may reuse `$x`'s register when every use of `%d` comes
/// before anything that could change `$x` (store, call, control flow).
fn alias_ok(body: &[Inst], at: usize, d: u32, var: &str, total: u32) -> bool {
    stable_until_uses(body, at, d, total, &|i: &Inst| matches!(i, Inst::Store { var: v, .. } if v == var))
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
        seen += us.iter().filter(|&&u| u == d).count() as u32;
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
        n += us.iter().filter(|&&u| u == r).count() as u32;
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
        Ins::BrI { c, a, b, t } => Ins::BrI { c: c.not(), a, b, t },
        Ins::BrF { c, a, b, t } => Ins::BrF { c: c.not(), a, b, t },
        Ins::BrS { c, a, b, t } => Ins::BrS { c: c.not(), a, b, t },
        Ins::JzI { s, t } => Ins::JnzI { s, t },
        Ins::JnzI { s, t } => Ins::JzI { s, t },
        other => other,
    }
}

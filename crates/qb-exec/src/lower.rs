//! Loader: QIR text -> typed register bytecode ([`crate::bytecode`]).
//!
//! QIR v0 is a stack machine whose values carry a type tag at run time. Its
//! types are nevertheless static (variables are typed by suffix, temporaries
//! by their first store), so the loader runs the stack *abstractly*: every
//! stack entry becomes a typed register (a variable's own register, a
//! constant register, or a fresh temporary) and every operation becomes one
//! typed instruction. Peephole rules then fuse the hot patterns:
//!
//! - a temporary stored into a variable: the producing instruction writes the
//!   variable directly (`s& = s& + x` is one `add.i32`; `a$ = a$ + x$` appends in place)
//! - compare + `JZ`/`JNZ` -> `br.*`
//! - `FORTEST` + `JZ` -> `for_test.*`; `NEXT` (add + `JMP` back to the test) -> `for_next.*`
//! - backward jumps -> `loop` (event safe point)
//! - hot builtins (LEN, CHR$, ASC, LEFT$, RIGHT$, MID$, math) -> dedicated instructions;
//!   the others are resolved once to a builtin table entry
//!
//! Fusion never crosses a statement (`LINE`) or a label, so error recovery
//! (RESUME / RESUME NEXT) and jumps see the same statement layout as QIR.

use std::collections::HashMap;

use crate::bytecode::*;
use crate::RunError;

fn lerr(msg: impl Into<String>) -> RunError {
    RunError { code: -1, line: 0, msg: msg.into() }
}

type Raw = (String, Vec<String>);

struct Header {
    name: String,
    params: Vec<String>,
    is_func: bool,
    is_static: bool,
}

/// Static type of a name from its suffix (`None` = untyped compiler temporary).
fn suffix_ty(name: &str) -> Option<Ty> {
    let n = name.trim_end_matches("()");
    if n.ends_with("&&") {
        Some(Ty::I64)
    } else if n.ends_with("##") {
        // _FLOAT: carried as DOUBLE until the 80-bit float type is wired in
        Some(Ty::F64)
    } else {
        match n.as_bytes().last()? {
            b'%' => Some(Ty::I16),
            b'&' => Some(Ty::I32),
            b'!' => Some(Ty::F32),
            b'#' => Some(Ty::F64),
            b'$' => Some(Ty::Str),
            _ => None,
        }
    }
}

fn is_array(name: &str) -> bool {
    name.ends_with("()")
}

#[derive(Clone, Copy, Debug)]
enum Var {
    N { r: R, ty: Ty },
    S { r: R },
    /// array operand (may carry G)
    A { r: R, ty: Ty },
    NG { g: R, ty: Ty },
    SG { g: R },
    /// untyped temporary: one register in each bank, type fixed by first use
    U { n: R, s: R, ty: Option<Ty> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kv {
    I(i64),
    F(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Src {
    Temp,
    Var,
    Const,
}

#[derive(Clone, Copy, Debug)]
enum E {
    N { r: R, ty: Ty, src: Src, k: Option<Kv> },
    S { r: R, src: Src },
    Ref { p: u32, ty: Option<Ty> },
    ForT { v: R, lim: R, step: R, int: bool },
}

/// Per-function register allocation.
struct Fx {
    vars: HashMap<String, Var>,
    consts: HashMap<(u64, bool), R>,
    num_init: Vec<u64>,
    n_str: u32,
    arr_tys: Vec<Ty>,
    params: Vec<Slot>,
    param_tys: Vec<Option<Ty>>,
    ret: Option<(Slot, Ty)>,
    is_static: bool,
    name: String,
    // temporaries above the variables/constants
    tbase_n: u32,
    tbase_s: u32,
    tn: u32,
    ts: u32,
    max_tn: u32,
    max_ts: u32,
}

impl Fx {
    fn new(name: &str) -> Fx {
        Fx {
            vars: HashMap::new(),
            consts: HashMap::new(),
            num_init: Vec::new(),
            n_str: 0,
            arr_tys: Vec::new(),
            params: Vec::new(),
            param_tys: Vec::new(),
            ret: None,
            is_static: false,
            name: name.to_string(),
            tbase_n: 0,
            tbase_s: 0,
            tn: 0,
            ts: 0,
            max_tn: 0,
            max_ts: 0,
        }
    }

    fn new_n(&mut self, init: u64) -> R {
        self.num_init.push(init);
        self.num_init.len() as R - 1
    }

    fn new_s(&mut self) -> R {
        self.n_str += 1;
        self.n_str - 1
    }

    /// Local variable (allocated on first sight).
    fn local(&mut self, name: &str) -> Var {
        if let Some(v) = self.vars.get(name) {
            return *v;
        }
        let v = if is_array(name) {
            self.arr_tys.push(suffix_ty(name).unwrap_or(Ty::F32));
            Var::A { r: self.arr_tys.len() as R - 1, ty: suffix_ty(name).unwrap_or(Ty::F32) }
        } else {
            match suffix_ty(name) {
                Some(Ty::Str) => Var::S { r: self.new_s() },
                Some(ty) => Var::N { r: self.new_n(0), ty },
                None => Var::U { n: self.new_n(0), s: self.new_s(), ty: None },
            }
        };
        self.vars.insert(name.to_string(), v);
        v
    }

    fn konst(&mut self, bits: u64, float: bool) -> R {
        if let Some(&r) = self.consts.get(&(bits, float)) {
            return r;
        }
        let r = self.new_n(bits);
        self.consts.insert((bits, float), r);
        r
    }

    fn tmp_n(&mut self) -> R {
        let r = self.tbase_n + self.tn;
        self.tn += 1;
        self.max_tn = self.max_tn.max(self.tn);
        r
    }

    fn tmp_s(&mut self) -> R {
        let r = self.tbase_s + self.ts;
        self.ts += 1;
        self.max_ts = self.max_ts.max(self.ts);
        r
    }
}

fn parse_int(text: &str) -> Result<i64, RunError> {
    text.parse::<i64>().map_err(|_| lerr(format!("bad literal {text}")))
}

fn parse_hex(text: &str) -> Result<Vec<u8>, RunError> {
    let bad = || lerr(format!("bad string literal {text}"));
    if text.len() % 2 != 0 {
        return Err(bad());
    }
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| bad())).collect()
}

/// `PUSH <ty> <text>` -> (type, value)
fn parse_num(ty: &str, text: &str) -> Result<(Ty, Kv), RunError> {
    let bad = || lerr(format!("bad literal {ty} {text}"));
    Ok(match ty {
        "I" => (Ty::I16, Kv::I(i64::from(text.parse::<i16>().map_err(|_| bad())?))),
        "L" => (Ty::I32, Kv::I(i64::from(text.parse::<i32>().map_err(|_| bad())?))),
        "Q" => (Ty::I64, Kv::I(parse_int(text)?)),
        "F" => (Ty::F32, Kv::F(f64::from(text.parse::<f32>().map_err(|_| bad())?))),
        "D" => (Ty::F64, Kv::F(text.parse::<f64>().map_err(|_| bad())?)),
        _ => return Err(bad()),
    })
}

fn kv_bits(k: Kv) -> (u64, bool) {
    match k {
        Kv::I(v) => (v as u64, false),
        Kv::F(v) => (v.to_bits(), true),
    }
}

/// Builtin result type for the generic path.
fn builtin_ret(name: &str, args: &[Ty]) -> Option<Ty> {
    if name.ends_with('$') || name == "MID$SET" {
        return Some(Ty::Str);
    }
    let a0 = args.first().copied();
    Some(match name {
        "LEN" | "INSTR" | "ASC" | "_ROUND" | "CLNG" | "CVL" => Ty::I32,
        "SGN" | "CINT" | "CVI" => Ty::I16,
        "VAL" | "CDBL" | "CVD" | "CVDMBF" => Ty::F64,
        "CSNG" | "CVS" | "CVSMBF" => Ty::F32,
        "ABS" | "INT" | "FIX" => a0.filter(|t| t.is_num())?,
        "SQR" | "LOG" | "EXP" | "SIN" | "COS" | "TAN" | "ATN" => {
            if a0 == Some(Ty::F64) {
                Ty::F64
            } else {
                Ty::F32
            }
        }
        _ => return None,
    })
}

struct Lw {
    m: Module,
    kstr_map: HashMap<Vec<u8>, u32>,
    bfn_map: HashMap<(String, Ty), u32>,
    labels: HashMap<String, u32>,
    /// (pc, label) whose target must be patched when the label is known
    fixups: Vec<(u32, String)>,
    barrier: u32,
    data_labels: HashMap<String, usize>,
    sub_index: HashMap<String, usize>,
    fxs: Vec<Fx>,
    cur: usize,
    stack: Vec<E>,
    /// next raw op (for peeking at the label after NEXT)
    next_label: Option<String>,
}

pub fn compile_v0(src: &str) -> Result<Module, RunError> {
    let mut lines = src.lines().map(str::trim).filter(|l| !l.is_empty());
    match lines.next() {
        Some(h) if h.starts_with("QIR ") => {}
        _ => return Err(lerr("missing QIR header")),
    }
    let mut main: Vec<Raw> = Vec::new();
    let mut subs: Vec<(Header, Vec<Raw>)> = Vec::new();
    let mut shared: Vec<String> = Vec::new();
    let mut in_sub = false;
    for line in lines {
        let mut parts = line.split(' ').filter(|p| !p.is_empty());
        let op = parts.next().unwrap().to_string();
        let args: Vec<String> = parts.map(str::to_string).collect();
        match op.as_str() {
            "SUB" | "FUNCTION" => {
                if in_sub || args.len() < 2 {
                    return Err(lerr(format!("bad SUB: {line}")));
                }
                let n: usize = args[1].parse().map_err(|_| lerr(line.to_string()))?;
                let rest = &args[2..];
                let is_static = rest.len() == n + 1 && rest[n] == "STATIC";
                if rest.len() != n && !is_static {
                    return Err(lerr(format!("bad SUB params: {line}")));
                }
                subs.push((
                    Header { name: args[0].clone(), params: rest[..n].to_vec(), is_func: op == "FUNCTION", is_static },
                    Vec::new(),
                ));
                in_sub = true;
            }
            "ENDSUB" => {
                if !in_sub {
                    return Err(lerr("ENDSUB outside SUB"));
                }
                subs.last_mut().unwrap().1.push((op, args));
                in_sub = false;
            }
            "SHARED" => shared.extend(args),
            _ => {
                if in_sub {
                    subs.last_mut().unwrap().1.push((op, args));
                } else {
                    main.push((op, args));
                }
            }
        }
    }
    if in_sub {
        return Err(lerr("SUB without ENDSUB"));
    }
    main.push(("END".into(), vec![]));

    // DATA items and the data index of every label (RESTORE label)
    let mut data = Vec::new();
    let mut data_labels = HashMap::new();
    for (op, args) in main.iter().chain(subs.iter().flat_map(|(_, b)| b.iter())) {
        match op.as_str() {
            "DATA" => data.push(parse_hex(args.first().map_or("", String::as_str))?),
            "LABEL" => {
                if let Some(n) = args.first() {
                    data_labels.insert(n.clone(), data.len());
                }
            }
            _ => {}
        }
    }

    // ---- register allocation (prescan) for every function
    let mut fxs = Vec::new();
    let mut mfx = Fx::new("main");
    for n in &shared {
        mfx.local(n);
    }
    prescan(&mut mfx, &main)?;
    fxs.push(mfx);
    let mut sub_index = HashMap::new();
    for (h, body) in &subs {
        if sub_index.insert(h.name.clone(), fxs.len()).is_some() {
            return Err(lerr(format!("duplicate SUB {}", h.name)));
        }
        let mut fx = Fx::new(&h.name);
        fx.is_static = h.is_static;
        for p in &h.params {
            let slot = match fx.local(p) {
                Var::N { r, ty } => {
                    fx.param_tys.push(Some(ty));
                    Slot::N(r)
                }
                Var::S { r } => {
                    fx.param_tys.push(Some(Ty::Str));
                    Slot::S(r)
                }
                Var::A { r, .. } => {
                    fx.param_tys.push(None);
                    Slot::A(r)
                }
                Var::U { n, s, .. } => {
                    // untyped parameter: SINGLE (QBasic default type)
                    fx.vars.insert(p.clone(), Var::U { n, s, ty: Some(Ty::F32) });
                    fx.param_tys.push(Some(Ty::F32));
                    Slot::N(n)
                }
                _ => unreachable!(),
            };
            fx.params.push(slot);
        }
        if h.is_func {
            fx.ret = Some(match fx.local(&h.name) {
                Var::N { r, ty } => (Slot::N(r), ty),
                Var::S { r } => (Slot::S(r), Ty::Str),
                Var::U { n, s, .. } => {
                    fx.vars.insert(h.name.clone(), Var::U { n, s, ty: Some(Ty::F32) });
                    (Slot::N(n), Ty::F32)
                }
                _ => return Err(lerr(format!("bad FUNCTION name {}", h.name))),
            });
        }
        // DIM SHARED names refer to module-level storage
        for n in &shared {
            let g = match fxs[0].vars.get(n) {
                Some(Var::N { r, ty }) => Var::NG { g: *r, ty: *ty },
                Some(Var::S { r }) => Var::SG { g: *r },
                Some(Var::A { r, ty }) => Var::A { r: *r | G, ty: *ty },
                _ => return Err(lerr(format!("SHARED {n} needs a type suffix"))),
            };
            if !fx.vars.contains_key(n) {
                fx.vars.insert(n.clone(), g);
            }
        }
        prescan(&mut fx, body)?;
        fxs.push(fx);
    }

    let mut lw = Lw {
        m: Module { data, ..Module::default() },
        kstr_map: HashMap::new(),
        bfn_map: HashMap::new(),
        labels: HashMap::new(),
        fixups: Vec::new(),
        barrier: 0,
        data_labels,
        sub_index,
        fxs,
        cur: 0,
        stack: Vec::new(),
        next_label: None,
    };
    let bodies: Vec<&Vec<Raw>> = std::iter::once(&main).chain(subs.iter().map(|(_, b)| b)).collect();
    let mut ranges = Vec::new();
    for (fi, body) in bodies.iter().enumerate() {
        lw.cur = fi;
        lw.stack.clear();
        let entry = lw.pc();
        lw.barrier = entry;
        {
            let fx = &mut lw.fxs[fi];
            fx.tbase_n = fx.num_init.len() as R;
            fx.tbase_s = fx.n_str;
        }
        for (i, (op, args)) in body.iter().enumerate() {
            lw.next_label = match body.get(i + 1) {
                Some((o, a)) if o == "LABEL" => a.first().cloned(),
                _ => None,
            };
            lw.op(op, args).map_err(|e| lerr(format!("{}: {op} {}: {}", lw.fxs[fi].name, args.join(" "), e.msg)))?;
        }
        if fi > 0 && !matches!(lw.m.code.last(), Some(Ins::Ret)) {
            return Err(lerr(format!("{}: missing ENDSUB", lw.fxs[fi].name)));
        }
        ranges.push((entry, lw.pc()));
    }
    for (pc, name) in std::mem::take(&mut lw.fixups) {
        let t = *lw.labels.get(&name).ok_or_else(|| lerr(format!("unknown label {name}")))?;
        *lw.m.code[pc as usize].target_mut().expect("fixup on non-jump") = t;
    }
    let mut funcs = Vec::new();
    for (fx, (entry, end)) in lw.fxs.into_iter().zip(ranges) {
        let mut num_init = fx.num_init;
        num_init.resize(num_init.len() + fx.max_tn as usize, 0);
        funcs.push(Func {
            name: fx.name,
            entry,
            end,
            num_init,
            n_str: fx.n_str + fx.max_ts,
            arr_tys: fx.arr_tys,
            params: fx.params,
            ret: fx.ret,
            is_static: fx.is_static,
        });
    }
    let mut m = lw.m;
    m.funcs = funcs;
    m.validate().map_err(|e| lerr(format!("internal: {e}")))?;
    Ok(m)
}

/// Allocates every variable and literal of a function body.
fn prescan(fx: &mut Fx, body: &[Raw]) -> Result<(), RunError> {
    for (op, args) in body {
        match op.as_str() {
            "LOAD" | "STORE" | "REF" | "LOADE" | "STOREE" | "REFE" | "DIMA" | "REDIMA" | "BOUND" => {
                let n = args.first().ok_or_else(|| lerr(format!("{op}: missing name")))?;
                fx.local(n);
            }
            "PUSH" => {
                let ty = args.first().ok_or_else(|| lerr("PUSH: missing type"))?;
                if ty != "S" {
                    let (_, k) = parse_num(ty, args.get(1).map_or("", String::as_str))?;
                    let (bits, fl) = kv_bits(k);
                    fx.konst(bits, fl);
                    if let Kv::I(v) = k {
                        // the same literal used in float arithmetic
                        fx.konst((v as f64).to_bits(), true);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn rank(t: Ty) -> u8 {
    match t {
        Ty::I16 => 0,
        Ty::I32 => 1,
        Ty::I64 => 2,
        Ty::F32 => 3,
        Ty::F64 => 4,
        Ty::Str => 9,
    }
}

/// Converts a constant for storage into `to` (None = overflow, error 6).
fn const_store(k: Kv, to: Ty) -> Option<Kv> {
    Some(match (k, to) {
        (Kv::I(v), Ty::I16 | Ty::I32 | Ty::I64) => Kv::I(to.width().wrap(v)),
        (Kv::F(v), Ty::I16 | Ty::I32 | Ty::I64) => {
            let r = qb_runtime::round::qb_qbr_f64(v);
            if to.width().wrap(r) != r {
                return None;
            }
            Kv::I(r)
        }
        (Kv::I(v), Ty::F32) => Kv::F(f64::from(v as f32)),
        (Kv::F(v), Ty::F32) => Kv::F(f64::from(v as f32)),
        (Kv::I(v), Ty::F64) => Kv::F(v as f64),
        (Kv::F(v), Ty::F64) => Kv::F(v),
        (_, Ty::Str) => return None,
    })
}

fn num_text(k: Kv, ty: Ty) -> Vec<u8> {
    match (k, ty) {
        (Kv::I(v), _) => qb_runtime::numfmt::str_i64(v),
        (Kv::F(v), Ty::F32) => qb_runtime::numfmt::str_f32(v as f32),
        (Kv::F(v), _) => qb_runtime::numfmt::str_f64(v),
    }
}

impl Lw {
    fn pc(&self) -> u32 {
        self.m.code.len() as u32
    }

    fn fx(&mut self) -> &mut Fx {
        &mut self.fxs[self.cur]
    }

    fn emit(&mut self, i: Ins) -> u32 {
        self.m.code.push(i);
        self.pc() - 1
    }

    fn emit_to(&mut self, i: Ins, label: &str) {
        let pc = self.emit(i);
        self.fixups.push((pc, label.to_string()));
    }

    fn kstr(&mut self, s: Vec<u8>) -> R {
        if let Some(&k) = self.kstr_map.get(&s) {
            return k | K;
        }
        let k = self.m.kstr.len() as u32;
        self.m.kstr.push(s.clone());
        self.kstr_map.insert(s, k);
        k | K
    }

    fn pop(&mut self) -> Result<E, RunError> {
        self.stack.pop().ok_or_else(|| lerr("stack underflow"))
    }

    fn pop_n(&mut self, n: usize) -> Result<Vec<E>, RunError> {
        if self.stack.len() < n {
            return Err(lerr("stack underflow"));
        }
        Ok(self.stack.split_off(self.stack.len() - n))
    }

    fn need_empty(&self, what: &str) -> Result<(), RunError> {
        if self.stack.is_empty() {
            Ok(())
        } else {
            Err(lerr(format!("values left on the stack at {what}")))
        }
    }

    fn label_pc(&self, name: &str) -> Option<u32> {
        self.labels.get(name).copied()
    }

    /// Variable as seen from the current function.
    fn var(&mut self, name: &str) -> Var {
        self.fx().local(name)
    }

    fn set_untyped(&mut self, name: &str, ty: Ty) -> Var {
        let fx = self.fx();
        if let Some(Var::U { n, s, ty: None }) = fx.vars.get(name).copied() {
            let v = Var::U { n, s, ty: Some(ty) };
            fx.vars.insert(name.to_string(), v);
            return v;
        }
        fx.vars[name]
    }

    // ------------------------------------------------------------ registers

    /// Copies stack entries that read variable register `r` (numeric bank if
    /// `!str`) into temporaries, before the variable is overwritten.
    fn protect(&mut self, str_bank: bool, r: Option<R>) {
        for i in 0..self.stack.len() {
            match self.stack[i] {
                E::N { r: x, ty, src: Src::Var, .. } if !str_bank && r.map_or(true, |r| r == x) => {
                    let t = self.fx().tmp_n();
                    self.emit(Ins::Mov { d: t, s: x });
                    self.stack[i] = E::N { r: t, ty, src: Src::Temp, k: None };
                }
                E::S { r: x, src: Src::Var } if str_bank && r.map_or(true, |r| r == x) => {
                    let t = self.fx().tmp_s();
                    self.emit(Ins::MovS { d: t, s: x });
                    self.stack[i] = E::S { r: t, src: Src::Temp };
                }
                _ => {}
            }
        }
    }

    /// Makes the last instruction (a value producer writing temporary `t`)
    /// write `d` instead. Only within the current statement/basic block.
    fn retarget(&mut self, str_bank: bool, t: R, d: R) -> bool {
        if self.pc() <= self.barrier {
            return false;
        }
        if let Some(last) = self.m.code.last_mut() {
            if let Some((bank, dst)) = last.dest_mut() {
                if bank == str_bank && *dst == t {
                    *dst = d;
                    return true;
                }
            }
        }
        false
    }

    /// Constant register for a float (prescanned), else a temporary loaded with it.
    /// Registers can no longer be added once temporaries are being allocated.
    fn float_const(&mut self, v: f64) -> R {
        if let Some(&r) = self.fx().consts.get(&(v.to_bits(), true)) {
            return r;
        }
        let t = self.fx().tmp_n();
        self.load_bits(t, v.to_bits());
        t
    }

    /// Register holding a numeric entry as an operand of type class `want`
    /// (integer operands may be wider than their value; floats are f64).
    fn operand(&mut self, e: E, want_float: bool) -> Result<R, RunError> {
        let E::N { r, ty, k, .. } = e else { return Err(lerr("numeric operand expected")) };
        Ok(match (ty.is_float(), want_float) {
            (true, true) | (false, false) => r,
            (false, true) => match k {
                Some(Kv::I(v)) => self.float_const(v as f64),
                _ => {
                    let t = self.fx().tmp_n();
                    self.emit(Ins::IToF { d: t, s: r });
                    t
                }
            },
            (true, false) => {
                let t = self.fx().tmp_n();
                match k {
                    Some(Kv::F(v)) => {
                        let i = qb_runtime::round::qb_qbr_f64(v);
                        self.load_int(t, i);
                    }
                    _ => {
                        self.emit(Ins::FToI { d: t, s: r, w: W::W64 });
                    }
                }
                t
            }
        })
    }

    fn load_int(&mut self, d: R, v: i64) {
        if let Ok(x) = i32::try_from(v) {
            self.emit(Ins::Ldi { d, v: x });
        } else {
            let k = self.m.nconst.len() as u32;
            self.m.nconst.push(v as u64);
            self.emit(Ins::Ldk { d, k });
        }
    }

    fn load_bits(&mut self, d: R, bits: u64) {
        let k = self.m.nconst.len() as u32;
        self.m.nconst.push(bits);
        self.emit(Ins::Ldk { d, k });
    }

    /// Integer operand register (index, count, error code, ...).
    fn int_reg(&mut self, e: E) -> Result<R, RunError> {
        self.operand(e, false)
    }

    /// Writes `e` converted to `to` with store semantics into `d` (or a new
    /// temporary when `d` is None). Returns the register holding the result.
    fn store_conv(&mut self, e: E, to: Ty, d: Option<R>) -> Result<R, RunError> {
        let E::N { r, ty, src, k } = e else {
            self.emit(Ins::RaiseK { code: 13 });
            return Ok(d.unwrap_or_else(|| self.fx().tmp_n()));
        };
        if to == Ty::Str {
            self.emit(Ins::RaiseK { code: 13 });
            return Ok(d.unwrap_or(r));
        }
        if let Some(kv) = k {
            let dst = match d {
                Some(d) => d,
                None if ty == to || (ty.is_int() && to.is_int() && rank(ty) <= rank(to)) || (ty == Ty::F32 && to == Ty::F64) => {
                    return Ok(r)
                }
                None => self.fx().tmp_n(),
            };
            match const_store(kv, to) {
                Some(Kv::I(v)) => self.load_int(dst, v),
                Some(Kv::F(v)) => self.load_bits(dst, v.to_bits()),
                None => {
                    self.emit(Ins::RaiseK { code: 6 });
                }
            }
            return Ok(dst);
        }
        // conversion kind
        enum C {
            Same,
            Wrap(W),
            FToI(W),
            IToF,
            IToFRound,
            Round,
        }
        let c = match (ty, to) {
            (a, b) if a == b && b != Ty::F32 => C::Same,
            (Ty::F32, Ty::F32) => {
                if src == Src::Var {
                    C::Same
                } else {
                    C::Round
                }
            }
            (a, b) if a.is_int() && b.is_int() => {
                if rank(a) <= rank(b) {
                    C::Same
                } else {
                    C::Wrap(b.width())
                }
            }
            (a, b) if a.is_float() && b.is_int() => C::FToI(b.width()),
            (Ty::I16, Ty::F32) => C::IToF,
            (_, Ty::F32) if ty.is_int() => C::IToFRound,
            (_, Ty::F32) => C::Round,
            (_, Ty::F64) if ty.is_int() => C::IToF,
            (_, Ty::F64) => C::Same,
            _ => return Err(lerr("bad conversion")),
        };
        let dst = |lw: &mut Lw| d.unwrap_or_else(|| lw.fx().tmp_n());
        Ok(match c {
            C::Same => match d {
                None => r,
                Some(d) if d == r => d,
                Some(d) => {
                    if !(src == Src::Temp && self.retarget(false, r, d)) {
                        self.emit(Ins::Mov { d, s: r });
                    }
                    d
                }
            },
            C::Wrap(w) => {
                let t = dst(self);
                self.emit(Ins::Wrap { d: t, s: r, w });
                t
            }
            C::FToI(w) => {
                let t = dst(self);
                self.emit(Ins::FToI { d: t, s: r, w });
                t
            }
            C::IToF => {
                let t = dst(self);
                self.emit(Ins::IToF { d: t, s: r });
                t
            }
            C::IToFRound => {
                let t = dst(self);
                self.emit(Ins::IToF { d: t, s: r });
                self.emit(Ins::RoundF32 { d: t, s: t });
                t
            }
            C::Round => {
                let t = dst(self);
                self.emit(Ins::RoundF32 { d: t, s: r });
                t
            }
        })
    }

    /// Stores a string entry into string register `d`.
    fn store_str(&mut self, e: E, d: R) {
        match e {
            E::S { r, src } => {
                if r == d {
                    return;
                }
                if src == Src::Temp {
                    if !self.retarget(true, r, d) {
                        self.emit(Ins::TakeS { d, s: r });
                    }
                } else {
                    self.emit(Ins::MovS { d, s: r });
                }
            }
            _ => {
                self.emit(Ins::RaiseK { code: 13 });
            }
        }
    }

    /// Value of a place as a stack entry (BYREF argument whose type differs).
    fn load_place(&mut self, p: u32) -> E {
        match self.m.places[p as usize] {
            Place::N(r, ty) => E::N { r, ty, src: Src::Var, k: None },
            Place::NG(g, ty) => {
                let t = self.fx().tmp_n();
                self.emit(Ins::GetG { d: t, g });
                E::N { r: t, ty, src: Src::Temp, k: None }
            }
            Place::S(r) => E::S { r, src: Src::Var },
            Place::SG(g) => {
                let t = self.fx().tmp_s();
                self.emit(Ins::GetGS { d: t, g });
                E::S { r: t, src: Src::Temp }
            }
            Place::E { a, i, n, ty } => {
                if ty == Ty::Str {
                    let t = self.fx().tmp_s();
                    self.emit(Ins::LdES { d: t, a, i, n });
                    E::S { r: t, src: Src::Temp }
                } else {
                    let t = self.fx().tmp_n();
                    self.emit(Ins::LdE { d: t, a, i, n });
                    E::N { r: t, ty, src: Src::Temp, k: None }
                }
            }
            Place::A(_) => E::Ref { p, ty: None },
        }
    }

    fn place(&mut self, p: Place) -> u32 {
        self.m.places.push(p);
        self.m.places.len() as u32 - 1
    }

    /// Index registers for `n` subscripts: one register, or `n` consecutive temporaries.
    fn indices(&mut self, n: usize, consecutive: bool) -> Result<R, RunError> {
        let idx = self.pop_n(n)?;
        if n == 1 && !consecutive {
            return self.int_reg(idx[0]);
        }
        let base = self.fx().tbase_n + self.fx().tn;
        for _ in 0..n {
            self.fx().tmp_n();
        }
        for (k, e) in idx.into_iter().enumerate() {
            let d = base + k as R;
            match e {
                E::N { ty, .. } if ty.is_int() => {
                    self.store_conv(e, Ty::I64, Some(d))?;
                }
                E::N { .. } => {
                    let r = self.int_reg(e)?;
                    self.emit(Ins::Mov { d, s: r });
                }
                _ => {
                    self.emit(Ins::RaiseK { code: 13 });
                }
            }
        }
        Ok(base)
    }

    fn arr(&mut self, name: &str) -> Result<(R, Ty), RunError> {
        match self.var(name) {
            Var::A { r, ty } => Ok((r, ty)),
            _ => Err(lerr(format!("{name} is not an array"))),
        }
    }

    // ------------------------------------------------------------ ops

    fn op(&mut self, op: &str, a: &[String]) -> Result<(), RunError> {
        let arg = |i: usize| -> Result<&str, RunError> { a.get(i).map(String::as_str).ok_or_else(|| lerr("missing operand")) };
        let num = |i: usize| -> Result<usize, RunError> { arg(i)?.parse().map_err(|_| lerr("bad count")) };
        match op {
            "LINE" => {
                self.need_empty("LINE")?;
                let n: u32 = arg(0)?.parse().map_err(|_| lerr("bad LINE"))?;
                let pc = self.pc();
                self.m.stmts.push((pc, n));
                self.barrier = pc;
                let fx = self.fx();
                fx.tn = 0;
                fx.ts = 0;
            }
            "LABEL" => {
                self.need_empty("LABEL")?;
                let name = arg(0)?;
                let pc = self.pc();
                if self.labels.insert(name.to_string(), pc).is_some() {
                    return Err(lerr(format!("duplicate label {name}")));
                }
                self.barrier = pc;
            }
            "DATA" => {}
            "PUSH" => {
                let ty = arg(0)?;
                if ty == "S" {
                    let s = parse_hex(a.get(1).map_or("", String::as_str))?;
                    let k = self.kstr(s);
                    self.stack.push(E::S { r: k, src: Src::Const });
                } else {
                    let (t, k) = parse_num(ty, a.get(1).map_or("", String::as_str))?;
                    let (bits, fl) = kv_bits(k);
                    let r = self.fx().konst(bits, fl);
                    self.stack.push(E::N { r, ty: t, src: Src::Const, k: Some(k) });
                }
            }
            "LOAD" => {
                let name = arg(0)?;
                let e = match self.var(name) {
                    Var::N { r, ty } => E::N { r, ty, src: Src::Var, k: None },
                    Var::S { r } => E::S { r, src: Src::Var },
                    Var::NG { g, ty } => {
                        let t = self.fx().tmp_n();
                        self.emit(Ins::GetG { d: t, g });
                        E::N { r: t, ty, src: Src::Temp, k: None }
                    }
                    Var::SG { g } => {
                        let t = self.fx().tmp_s();
                        self.emit(Ins::GetGS { d: t, g });
                        E::S { r: t, src: Src::Temp }
                    }
                    Var::U { .. } => match self.set_untyped(name, Ty::F32) {
                        Var::U { s, ty: Some(Ty::Str), .. } => E::S { r: s, src: Src::Var },
                        Var::U { n, ty: Some(ty), .. } => E::N { r: n, ty, src: Src::Var, k: None },
                        _ => unreachable!(),
                    },
                    Var::A { .. } => return Err(lerr(format!("LOAD of array {name}"))),
                };
                self.stack.push(e);
            }
            "STORE" => {
                let name = arg(0)?;
                let e = self.pop()?;
                let mut v = self.var(name);
                if let Var::U { ty: None, .. } = v {
                    let ty = match e {
                        E::N { ty, .. } => ty,
                        E::S { .. } => Ty::Str,
                        _ => return Err(lerr("bad STORE operand")),
                    };
                    v = self.set_untyped(name, ty);
                }
                match v {
                    Var::N { r, ty } | Var::U { n: r, ty: Some(ty @ (Ty::I16 | Ty::I32 | Ty::I64 | Ty::F32 | Ty::F64)), .. } => {
                        self.protect(false, Some(r));
                        self.store_conv(e, ty, Some(r))?;
                    }
                    Var::S { r } | Var::U { s: r, ty: Some(Ty::Str), .. } => {
                        self.protect(true, Some(r));
                        self.store_str(e, r);
                    }
                    Var::NG { g, ty } => {
                        let r = self.store_conv(e, ty, None)?;
                        self.emit(Ins::SetG { g, s: r });
                    }
                    Var::SG { g } => match e {
                        E::S { r, .. } => {
                            self.emit(Ins::SetGS { g, s: r });
                        }
                        _ => {
                            self.emit(Ins::RaiseK { code: 13 });
                        }
                    },
                    _ => return Err(lerr(format!("bad STORE target {name}"))),
                }
            }
            "REF" => {
                let name = arg(0)?;
                let v = match self.var(name) {
                    Var::U { ty: None, .. } => self.set_untyped(name, Ty::F32),
                    v => v,
                };
                let (p, ty) = match v {
                    Var::N { r, ty } | Var::U { n: r, ty: Some(ty @ (Ty::I16 | Ty::I32 | Ty::I64 | Ty::F32 | Ty::F64)), .. } => {
                        (Place::N(r, ty), Some(ty))
                    }
                    Var::S { r } | Var::U { s: r, ty: Some(Ty::Str), .. } => (Place::S(r), Some(Ty::Str)),
                    Var::NG { g, ty } => (Place::NG(g, ty), Some(ty)),
                    Var::SG { g } => (Place::SG(g), Some(Ty::Str)),
                    Var::A { r, .. } => (Place::A(r), None),
                    Var::U { .. } => unreachable!(),
                };
                let p = self.place(p);
                self.stack.push(E::Ref { p, ty });
            }
            "LOADE" => {
                let (a_, ty) = self.arr(arg(0)?)?;
                let n = num(1)?;
                let i = self.indices(n, n > 1)?;
                if ty == Ty::Str {
                    let d = self.fx().tmp_s();
                    self.emit(if n == 1 { Ins::LdE1S { d, a: a_, i } } else { Ins::LdES { d, a: a_, i, n: n as u32 } });
                    self.stack.push(E::S { r: d, src: Src::Temp });
                } else {
                    let d = self.fx().tmp_n();
                    self.emit(if n == 1 { Ins::LdE1 { d, a: a_, i } } else { Ins::LdE { d, a: a_, i, n: n as u32 } });
                    self.stack.push(E::N { r: d, ty, src: Src::Temp, k: None });
                }
            }
            "STOREE" => {
                let (a_, ty) = self.arr(arg(0)?)?;
                let n = num(1)?;
                let v = self.pop()?;
                let i = self.indices(n, n > 1)?;
                if ty == Ty::Str {
                    let s = match v {
                        E::S { r, .. } => r,
                        _ => {
                            self.emit(Ins::RaiseK { code: 13 });
                            return Ok(());
                        }
                    };
                    self.emit(if n == 1 { Ins::StE1S { a: a_, i, s } } else { Ins::StES { a: a_, i, n: n as u32, s } });
                } else {
                    let s = self.store_conv(v, ty, None)?;
                    self.emit(if n == 1 { Ins::StE1 { a: a_, i, s } } else { Ins::StE { a: a_, i, n: n as u32, s } });
                }
            }
            "REFE" => {
                let (a_, ty) = self.arr(arg(0)?)?;
                let n = num(1)?;
                let i = self.indices(n, true)?;
                let p = self.place(Place::E { a: a_, i, n: n as u32, ty });
                self.stack.push(E::Ref { p, ty: Some(ty) });
            }
            "DIMA" | "REDIMA" => {
                let (a_, _) = self.arr(arg(0)?)?;
                let n = num(1)?;
                let mode = if op == "DIMA" {
                    DimMode::Dim
                } else if num(2)? != 0 {
                    DimMode::RedimPreserve
                } else {
                    DimMode::Redim
                };
                let dims = self.indices(2 * n, true)?;
                self.emit(Ins::Dim { a: a_, dims, n: n as u32, mode });
            }
            "BOUND" => {
                let (a_, _) = self.arr(arg(0)?)?;
                let upper = arg(1)? == "U";
                let e = self.pop()?;
                let dim = self.int_reg(e)?;
                let d = self.fx().tmp_n();
                self.emit(Ins::Bound { d, a: a_, dim, upper });
                self.stack.push(E::N { r: d, ty: Ty::I32, src: Src::Temp, k: None });
            }
            "SWAP" => {
                let (q, p) = (self.pop()?, self.pop()?);
                let (E::Ref { p, ty: tp }, E::Ref { p: q, ty: tq }) = (p, q) else {
                    return Err(lerr("SWAP needs two references"));
                };
                self.protect(false, None);
                self.protect(true, None);
                if tp != tq {
                    self.emit(Ins::RaiseK { code: 13 });
                } else {
                    self.emit(Ins::Swap { p, q });
                }
            }
            "OP" => self.binop(arg(0)?)?,
            "UN" => self.unop(arg(0)?)?,
            "FN" => self.builtin(arg(0)?, num(1)?)?,
            "PRINT" => match self.pop()? {
                E::S { r, .. } => {
                    self.emit(Ins::PrintS { s: r });
                }
                E::N { k: Some(k), ty, .. } => {
                    let mut t = num_text(k, ty);
                    t.push(b' ');
                    let s = self.kstr(t);
                    self.emit(Ins::PrintS { s });
                }
                E::N { r, ty, .. } => {
                    self.emit(Ins::PrintN { s: r, ty });
                }
                _ => return Err(lerr("PRINT of a reference")),
            },
            "PRINTZONE" => {
                self.emit(Ins::PrintZone);
            }
            "PRINTNL" => {
                self.emit(Ins::PrintNl);
            }
            "JMP" => self.jmp(arg(0)?)?,
            "JZ" | "JNZ" => self.cond_jump(arg(0)?, op == "JNZ")?,
            "GOSUB" => {
                self.need_empty("GOSUB")?;
                self.emit_to(Ins::Gosub { t: 0 }, arg(0)?);
            }
            "RETURN" => {
                self.emit(Ins::Return);
            }
            "FORTEST" => {
                let (step, lim, v) = (self.pop()?, self.pop()?, self.pop()?);
                let int = [step, lim, v].iter().all(|e| matches!(e, E::N { ty, .. } if ty.is_int()));
                let v = self.operand(v, !int)?;
                let lim = self.operand(lim, !int)?;
                let step = self.operand(step, !int)?;
                self.stack.push(E::ForT { v, lim, step, int });
            }
            "CALL" => self.call(arg(0)?, num(1)?)?,
            "ENDSUB" => {
                self.need_empty("ENDSUB")?;
                self.emit(Ins::Ret);
            }
            "ONERROR" => {
                if arg(0)? == "0" {
                    self.emit(Ins::OnError { t: NONE });
                } else {
                    self.emit_to(Ins::OnError { t: 0 }, arg(0)?);
                }
            }
            "RESUME" => match arg(0)? {
                "0" => {
                    self.emit(Ins::Resume { k: ResumeK::Same, t: 0 });
                }
                "NEXT" => {
                    self.emit(Ins::Resume { k: ResumeK::Next, t: 0 });
                }
                l => self.emit_to(Ins::Resume { k: ResumeK::Label, t: 0 }, l),
            },
            "ERROR" => {
                let e = self.pop()?;
                let s = self.int_reg(e)?;
                self.emit(Ins::Raise { s });
            }
            "READ" => {
                if num(0)? != 0 {
                    let d = self.fx().tmp_s();
                    self.emit(Ins::Read { d, str: true });
                    self.stack.push(E::S { r: d, src: Src::Temp });
                } else {
                    let d = self.fx().tmp_n();
                    self.emit(Ins::Read { d, str: false });
                    self.stack.push(E::N { r: d, ty: Ty::F64, src: Src::Temp, k: None });
                }
            }
            "RESTORE" => {
                let i = match a.first() {
                    None => 0,
                    Some(n) => *self.data_labels.get(n).ok_or_else(|| lerr(format!("unknown label {n}")))?,
                };
                self.emit(Ins::Restore { i: i as u32 });
            }
            "RANDOMIZE" => {
                let e = self.pop()?;
                let s = self.operand(e, true)?;
                self.emit(Ins::Randomize { s, using: num(0)? != 0 });
            }
            "END" => {
                self.emit(Ins::End);
            }
            // events (QIR v0 extension): ON <event> GOSUB, EVENT ON/OFF/STOP, ON TIMER(n) GOSUB
            "ONEVENT" => {
                let id: u32 = arg(0)?.parse().map_err(|_| lerr("bad event id"))?;
                if id == 0 || id >= 32 {
                    return Err(lerr("event id must be 1..31 (0 = TIMER)"));
                }
                self.emit_to(Ins::OnEvent { id, t: 0 }, arg(1)?);
            }
            "EVENTON" | "EVENTOFF" | "EVENTSTOP" => {
                let id: u32 = arg(0)?.parse().map_err(|_| lerr("bad event id"))?;
                if id >= 32 {
                    return Err(lerr("bad event id"));
                }
                let mode = match op {
                    "EVENTON" => EvMode::On,
                    "EVENTOFF" => EvMode::Off,
                    _ => EvMode::Stop,
                };
                self.emit(Ins::EventCtl { id, mode });
            }
            "ONTIMER" => {
                let e = self.pop()?;
                let s = self.operand(e, true)?;
                self.emit_to(Ins::OnTimer { s, t: 0 }, arg(0)?);
            }
            other => return Err(lerr(format!("unknown op {other}"))),
        }
        Ok(())
    }

    fn jmp(&mut self, label: &str) -> Result<(), RunError> {
        self.need_empty("JMP")?;
        let Some(t) = self.label_pc(label) else {
            self.emit_to(Ins::Jmp { t: 0 }, label);
            return Ok(());
        };
        // NEXT: `v = v + step` followed by a jump back to the loop's for_test
        if let Ins::ForTestI { v, lim, step, .. } = self.m.code[t as usize] {
            let last = self.pc().checked_sub(1).filter(|&p| p >= self.barrier);
            if let Some(lp) = last {
                let w = match self.m.code[lp as usize] {
                    Ins::AddI16 { d, a, b } if d == v && ((a == v && b == step) || (b == v && a == step)) => Some(W::W16),
                    Ins::AddI32 { d, a, b } if d == v && ((a == v && b == step) || (b == v && a == step)) => Some(W::W32),
                    Ins::AddI64 { d, a, b } if d == v && ((a == v && b == step) || (b == v && a == step)) => Some(W::W64),
                    _ => None,
                };
                if let Some(w) = w {
                    self.m.code[lp as usize] = Ins::ForNextI { v, step, lim, body: t + 1, w };
                    // fall through to the loop exit (emit a jump unless it is the next label)
                    let exit = self.fixups.iter().find(|(pc, _)| *pc == t).map(|(_, l)| l.clone());
                    if let Some(exit) = exit {
                        if self.next_label.as_deref() != Some(exit.as_str()) {
                            self.emit_to(Ins::Jmp { t: 0 }, &exit);
                        }
                        return Ok(());
                    }
                    let Ins::ForTestI { t: x, .. } = self.m.code[t as usize] else { unreachable!() };
                    self.emit(Ins::Jmp { t: x });
                    return Ok(());
                }
            }
        }
        if t < self.pc() {
            self.emit(Ins::Loop { t });
        } else {
            self.emit(Ins::Jmp { t });
        }
        Ok(())
    }

    fn cond_jump(&mut self, label: &str, on_true: bool) -> Result<(), RunError> {
        let e = self.pop()?;
        self.need_empty("conditional jump")?;
        let ins = match e {
            E::ForT { v, lim, step, int } => {
                if on_true {
                    return Err(lerr("FORTEST must be followed by JZ"));
                }
                if int {
                    Ins::ForTestI { v, lim, step, t: 0 }
                } else {
                    Ins::ForTestF { v, lim, step, t: 0 }
                }
            }
            E::N { k: Some(k), .. } => {
                let truth = match k {
                    Kv::I(v) => v != 0,
                    Kv::F(v) => v != 0.0,
                };
                if truth == on_true {
                    return self.jmp(label);
                }
                return Ok(());
            }
            E::N { r, ty, src, .. } => {
                let fused = if src == Src::Temp && self.pc() > self.barrier {
                    match self.m.code.last().copied() {
                        Some(Ins::CmpI { c, d, a, b }) if d == r => Some(Ins::BrI { c: if on_true { c } else { c.not() }, a, b, t: 0 }),
                        Some(Ins::CmpF { c, d, a, b }) if d == r => Some(Ins::BrF { c: if on_true { c } else { c.not() }, a, b, t: 0 }),
                        Some(Ins::CmpS { c, d, a, b }) if d == r => Some(Ins::BrS { c: if on_true { c } else { c.not() }, a, b, t: 0 }),
                        _ => None,
                    }
                } else {
                    None
                };
                match fused {
                    Some(b) => {
                        self.m.code.pop();
                        b
                    }
                    None => match (ty.is_float(), on_true) {
                        (false, false) => Ins::JzI { s: r, t: 0 },
                        (false, true) => Ins::JnzI { s: r, t: 0 },
                        (true, false) => Ins::JzF { s: r, t: 0 },
                        (true, true) => Ins::JnzF { s: r, t: 0 },
                    },
                }
            }
            _ => {
                self.emit(Ins::RaiseK { code: 13 });
                return Ok(());
            }
        };
        match self.label_pc(label) {
            // backward conditional jump: inverted branch over a `loop` (safe point)
            Some(t) if t <= self.pc() && !matches!(ins, Ins::ForTestI { .. } | Ins::ForTestF { .. }) => {
                let skip = self.pc() + 2;
                let mut inv = invert(ins);
                *inv.target_mut().unwrap() = skip;
                self.emit(inv);
                self.emit(Ins::Loop { t });
            }
            Some(t) => {
                let mut i = ins;
                *i.target_mut().unwrap() = t;
                self.emit(i);
            }
            None => self.emit_to(ins, label),
        }
        Ok(())
    }

    fn binop(&mut self, op: &str) -> Result<(), RunError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let cmp = match op {
            "EQ" => Some(Cmp::Eq),
            "NE" => Some(Cmp::Ne),
            "LT" => Some(Cmp::Lt),
            "LE" => Some(Cmp::Le),
            "GT" => Some(Cmp::Gt),
            "GE" => Some(Cmp::Ge),
            _ => None,
        };
        match (a, b) {
            (E::S { r: ra, src: sa }, E::S { r: rb, .. }) => {
                if let Some(c) = cmp {
                    let d = self.fx().tmp_n();
                    self.emit(Ins::CmpS { c, d, a: ra, b: rb });
                    self.stack.push(E::N { r: d, ty: Ty::I16, src: Src::Temp, k: None });
                } else if op == "ADD" {
                    // a temporary left operand is extended in place
                    let d = if sa == Src::Temp { ra } else { self.fx().tmp_s() };
                    self.emit(Ins::Concat { d, a: ra, b: rb });
                    self.stack.push(E::S { r: d, src: Src::Temp });
                } else {
                    return self.mismatch(Ty::I16);
                }
            }
            (E::N { ty: ta, .. }, E::N { ty: tb, .. }) => {
                let ints = ta.is_int() && tb.is_int();
                let any64 = ta == Ty::I64 || tb == Ty::I64;
                let fres = if ta == Ty::F64 || tb == Ty::F64 { Ty::F64 } else { Ty::F32 };
                if let Some(c) = cmp {
                    let (x, y) = (self.operand(a, !ints)?, self.operand(b, !ints)?);
                    let d = self.fx().tmp_n();
                    self.emit(if ints { Ins::CmpI { c, d, a: x, b: y } } else { Ins::CmpF { c, d, a: x, b: y } });
                    self.stack.push(E::N { r: d, ty: Ty::I16, src: Src::Temp, k: None });
                    return Ok(());
                }
                match op {
                    "ADD" | "SUB" | "MUL" if ints => {
                        let rt = if rank(ta) >= rank(tb) { ta } else { tb };
                        let (x, y) = (self.operand(a, false)?, self.operand(b, false)?);
                        let d = self.fx().tmp_n();
                        self.emit(match (op, rt) {
                            ("ADD", Ty::I16) => Ins::AddI16 { d, a: x, b: y },
                            ("ADD", Ty::I32) => Ins::AddI32 { d, a: x, b: y },
                            ("ADD", _) => Ins::AddI64 { d, a: x, b: y },
                            ("SUB", Ty::I16) => Ins::SubI16 { d, a: x, b: y },
                            ("SUB", Ty::I32) => Ins::SubI32 { d, a: x, b: y },
                            ("SUB", _) => Ins::SubI64 { d, a: x, b: y },
                            (_, Ty::I16) => Ins::MulI16 { d, a: x, b: y },
                            (_, Ty::I32) => Ins::MulI32 { d, a: x, b: y },
                            _ => Ins::MulI64 { d, a: x, b: y },
                        });
                        self.stack.push(E::N { r: d, ty: rt, src: Src::Temp, k: None });
                    }
                    "ADD" | "SUB" | "MUL" | "DIV" | "POW" => {
                        let (x, y) = (self.operand(a, true)?, self.operand(b, true)?);
                        let d = self.fx().tmp_n();
                        self.emit(match op {
                            "ADD" => Ins::AddF { d, a: x, b: y },
                            "SUB" => Ins::SubF { d, a: x, b: y },
                            "MUL" => Ins::MulF { d, a: x, b: y },
                            "DIV" => Ins::DivF { d, a: x, b: y },
                            _ => Ins::PowF { d, a: x, b: y },
                        });
                        self.stack.push(E::N { r: d, ty: fres, src: Src::Temp, k: None });
                    }
                    "IDIV" | "MOD" | "AND" | "OR" | "XOR" | "EQV" | "IMP" => {
                        let rt = if any64 {
                            Ty::I64
                        } else if ta == Ty::I16 && tb == Ty::I16 {
                            Ty::I16
                        } else {
                            Ty::I32
                        };
                        let (x, y) = (self.operand(a, false)?, self.operand(b, false)?);
                        let d = self.fx().tmp_n();
                        let w = rt.width();
                        self.emit(match op {
                            "IDIV" => Ins::IDiv { d, a: x, b: y, w },
                            "MOD" => Ins::Mod { d, a: x, b: y, w },
                            "AND" => Ins::And { d, a: x, b: y },
                            "OR" => Ins::Or { d, a: x, b: y },
                            "XOR" => Ins::Xor { d, a: x, b: y },
                            "EQV" => Ins::Eqv { d, a: x, b: y },
                            _ => Ins::Imp { d, a: x, b: y },
                        });
                        if !ints && rt != Ty::I64 && !matches!(op, "IDIV" | "MOD") {
                            self.emit(Ins::Wrap { d, s: d, w });
                        }
                        self.stack.push(E::N { r: d, ty: rt, src: Src::Temp, k: None });
                    }
                    _ => return Err(lerr(format!("unknown OP {op}"))),
                }
            }
            _ => return self.mismatch(Ty::I16),
        }
        Ok(())
    }

    /// Type mismatch found while loading: raise error 13 when reached.
    fn mismatch(&mut self, ty: Ty) -> Result<(), RunError> {
        self.emit(Ins::RaiseK { code: 13 });
        if ty == Ty::Str {
            let d = self.fx().tmp_s();
            self.stack.push(E::S { r: d, src: Src::Temp });
        } else {
            let d = self.fx().tmp_n();
            self.stack.push(E::N { r: d, ty, src: Src::Temp, k: None });
        }
        Ok(())
    }

    fn unop(&mut self, op: &str) -> Result<(), RunError> {
        let e = self.pop()?;
        let E::N { r, ty, .. } = e else { return self.mismatch(Ty::I16) };
        let d = self.fx().tmp_n();
        let rt = match op {
            "NEG" => {
                let u = if ty.is_int() { Un::NegI(ty.width()) } else { Un::NegF };
                self.emit(Ins::Un { op: u, d, s: r });
                ty
            }
            "NOT" => {
                let s = self.operand(e, false)?;
                self.emit(Ins::Un { op: Un::Not, d, s });
                if ty.is_float() {
                    self.emit(Ins::Wrap { d, s: d, w: W::W32 });
                    Ty::I32
                } else {
                    ty
                }
            }
            _ => return Err(lerr(format!("unknown UN {op}"))),
        };
        self.stack.push(E::N { r: d, ty: rt, src: Src::Temp, k: None });
        Ok(())
    }

    fn sreg(&mut self, e: E) -> Option<R> {
        match e {
            E::S { r, .. } => Some(r),
            _ => None,
        }
    }

    fn builtin(&mut self, name: &str, argc: usize) -> Result<(), RunError> {
        let args = self.pop_n(argc)?;
        let push_n = |lw: &mut Lw, d: R, ty: Ty| lw.stack.push(E::N { r: d, ty, src: Src::Temp, k: None });
        let push_s = |lw: &mut Lw, d: R| lw.stack.push(E::S { r: d, src: Src::Temp });
        let num_ty = |e: &E| match e {
            E::N { ty, .. } => Some(*ty),
            _ => None,
        };
        match (name, argc) {
            ("ERR", 0) => {
                let d = self.fx().tmp_n();
                self.emit(Ins::Err { d });
                push_n(self, d, Ty::I32);
            }
            ("ERL", 0) => {
                let d = self.fx().tmp_n();
                self.emit(Ins::Erl { d });
                push_n(self, d, Ty::I32);
            }
            ("RND", 0 | 1) => {
                let s = match args.first() {
                    Some(e) => self.operand(*e, true)?,
                    None => NONE,
                };
                let d = self.fx().tmp_n();
                self.emit(Ins::Rnd { d, s });
                push_n(self, d, Ty::F32);
            }
            ("LEN", 1) if matches!(args[0], E::S { .. }) => {
                let s = self.sreg(args[0]).unwrap();
                let d = self.fx().tmp_n();
                self.emit(Ins::LenS { d, s });
                push_n(self, d, Ty::I32);
            }
            ("ASC", 1) if matches!(args[0], E::S { .. }) => {
                let s = self.sreg(args[0]).unwrap();
                let d = self.fx().tmp_n();
                self.emit(Ins::Asc { d, s });
                push_n(self, d, Ty::I32);
            }
            ("CHR$", 1) if num_ty(&args[0]).is_some() => {
                let s = self.int_reg(args[0])?;
                let d = self.fx().tmp_s();
                self.emit(Ins::Chr { d, s });
                push_s(self, d);
            }
            ("LEFT$" | "RIGHT$", 2) if matches!(args[0], E::S { .. }) && num_ty(&args[1]).is_some() => {
                let s = self.sreg(args[0]).unwrap();
                let n = self.int_reg(args[1])?;
                let d = self.fx().tmp_s();
                self.emit(if name == "LEFT$" { Ins::Left { d, s, n } } else { Ins::Right { d, s, n } });
                push_s(self, d);
            }
            ("MID$", 2 | 3) if matches!(args[0], E::S { .. }) && args[1..].iter().all(|e| num_ty(e).is_some()) => {
                let s = self.sreg(args[0]).unwrap();
                let start = self.int_reg(args[1])?;
                let n = if argc == 3 { self.int_reg(args[2])? } else { NONE };
                let d = self.fx().tmp_s();
                self.emit(Ins::Mid { d, s, start, n });
                push_s(self, d);
            }
            ("INT" | "FIX", 1) if num_ty(&args[0]).is_some_and(Ty::is_int) => self.stack.push(args[0]),
            ("ABS" | "SGN" | "INT" | "FIX" | "SQR" | "LOG" | "EXP" | "SIN" | "COS" | "TAN" | "ATN", 1) if num_ty(&args[0]).is_some() => {
                let ty = num_ty(&args[0]).unwrap();
                let fl = if ty == Ty::F64 { Ty::F64 } else { Ty::F32 };
                let (op, rt, float_arg) = match name {
                    "ABS" if ty.is_int() => (Un::AbsI(ty.width()), ty, false),
                    "ABS" => (Un::AbsF, ty, true),
                    "SGN" if ty.is_int() => (Un::SgnI, Ty::I16, false),
                    "SGN" => (Un::SgnF, Ty::I16, true),
                    "INT" => (Un::Int, ty, true),
                    "FIX" => (Un::Fix, ty, true),
                    "SQR" => (Un::Sqr, fl, true),
                    "LOG" => (Un::Log, fl, true),
                    "EXP" => (Un::Exp, fl, true),
                    "SIN" => (Un::Sin, fl, true),
                    "COS" => (Un::Cos, fl, true),
                    "TAN" => (Un::Tan, fl, true),
                    _ => (Un::Atn, fl, true),
                };
                let s = self.operand(args[0], float_arg)?;
                let d = self.fx().tmp_n();
                self.emit(Ins::Un { op, d, s });
                push_n(self, d, rt);
            }
            _ => {
                let tys: Vec<Ty> = args
                    .iter()
                    .map(|e| match e {
                        E::N { ty, .. } => Ok(*ty),
                        E::S { .. } => Ok(Ty::Str),
                        _ => Err(lerr("builtin argument is a reference")),
                    })
                    .collect::<Result<_, _>>()?;
                let Some(rt) = builtin_ret(name, &tys) else {
                    if builtin_ret(name, &[Ty::F64]).is_some() || name.ends_with('$') {
                        return self.mismatch(Ty::I16);
                    }
                    return Err(lerr(format!("unknown builtin {name}")));
                };
                let regs: Vec<(R, Ty)> = args
                    .iter()
                    .zip(&tys)
                    .map(|(e, t)| match e {
                        E::N { r, .. } | E::S { r, .. } => (*r, *t),
                        _ => unreachable!(),
                    })
                    .collect();
                let key = (name.to_string(), rt);
                let f = match self.bfn_map.get(&key) {
                    Some(&f) => f,
                    None => {
                        let f = self.m.bfns.len() as u32;
                        self.m.bfns.push(key.clone());
                        self.bfn_map.insert(key, f);
                        f
                    }
                };
                self.m.bargs.push(regs);
                let args_i = self.m.bargs.len() as u32 - 1;
                if rt == Ty::Str {
                    let d = self.fx().tmp_s();
                    self.emit(Ins::Bi { f, d, args: args_i });
                    push_s(self, d);
                } else {
                    let d = self.fx().tmp_n();
                    self.emit(Ins::Bi { f, d, args: args_i });
                    push_n(self, d, rt);
                }
            }
        }
        Ok(())
    }

    fn call(&mut self, name: &str, argc: usize) -> Result<(), RunError> {
        let fi = *self.sub_index.get(name).ok_or_else(|| lerr(format!("unknown SUB {name}")))?;
        let (params, ptys, ret) = {
            let f = &self.fxs[fi];
            (f.params.clone(), f.param_tys.clone(), f.ret)
        };
        if params.len() != argc {
            // QBasic: wrong number of arguments (450)
            self.pop_n(argc)?;
            self.emit(Ins::RaiseK { code: 450 });
            if let Some((_, ty)) = ret {
                return self.mismatch(ty);
            }
            return Ok(());
        }
        let args = self.pop_n(argc)?;
        // the callee may change variables through BYREF / SHARED
        self.protect(false, None);
        self.protect(true, None);
        let mut out = Vec::with_capacity(argc);
        for (e, (slot, pty)) in args.into_iter().zip(params.iter().zip(&ptys)) {
            let e = match e {
                E::Ref { p, ty } if ty == *pty => {
                    out.push(Arg::Ref(p));
                    continue;
                }
                E::Ref { p, .. } => self.load_place(p),
                e => e,
            };
            match (slot, e) {
                (Slot::N(_), E::N { .. }) => {
                    let r = self.store_conv(e, pty.unwrap(), None)?;
                    out.push(Arg::N(r));
                }
                (Slot::S(_), E::S { r, src }) => out.push(if src == Src::Temp { Arg::SMove(r) } else { Arg::S(r) }),
                _ => {
                    // QBasic: parameter type mismatch
                    self.emit(Ins::RaiseK { code: 13 });
                    if let Some((_, ty)) = ret {
                        return self.mismatch(ty);
                    }
                    return Ok(());
                }
            }
        }
        let dest = match ret {
            Some((Slot::S(_), _)) => {
                let d = self.fx().tmp_s();
                self.stack.push(E::S { r: d, src: Src::Temp });
                Some(d)
            }
            Some((_, ty)) => {
                let d = self.fx().tmp_n();
                self.stack.push(E::N { r: d, ty, src: Src::Temp, k: None });
                Some(d)
            }
            None => None,
        };
        self.m.calls.push(CallDesc { args: out, dest });
        let desc = self.m.calls.len() as u32 - 1;
        self.emit(Ins::Call { f: fi as u32, desc });
        Ok(())
    }
}

/// Branch with the opposite condition (same operands, target unchanged).
fn invert(i: Ins) -> Ins {
    match i {
        Ins::BrI { c, a, b, t } => Ins::BrI { c: c.not(), a, b, t },
        Ins::BrF { c, a, b, t } => Ins::BrF { c: c.not(), a, b, t },
        Ins::BrS { c, a, b, t } => Ins::BrS { c: c.not(), a, b, t },
        Ins::JzI { s, t } => Ins::JnzI { s, t },
        Ins::JnzI { s, t } => Ins::JzI { s, t },
        Ins::JzF { s, t } => Ins::JnzF { s, t },
        Ins::JnzF { s, t } => Ins::JzF { s, t },
        other => other,
    }
}

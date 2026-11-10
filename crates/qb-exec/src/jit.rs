//! JIT tier (R11.6): VM bytecode -> `qb-jit` MIR -> native code.
//!
//! Native code runs on the VM's own numeric register window (`nums[nb..]`),
//! so entering and leaving it needs no translation:
//!
//! - numeric instructions, branches and FOR loops are compiled natively;
//! - instructions that touch strings, arrays or runtime state (PRINT, MID$,
//!   array elements, builtins, ...) become `VmOp`: native code calls back into
//!   the VM, which executes exactly that one instruction ([`Vm::step_one`]);
//! - control transfers the VM owns (CALL, RET, GOSUB/RETURN, END, ON ERROR,
//!   RESUME, events setup, string branches) end native code with "continue
//!   at pc" (deopt) and the VM carries on.
//!
//! Errors leave native code with (code, pc of the failing instruction), the
//! same pair the interpreter produces, so ON ERROR / RESUME are unchanged.
//! Events are polled at the same back-edges as the VM (`loop`, `for_next`).

use crate::bytecode::{self as bc, Cmp, Ins, Opnd, W};
use qb_jit::mir::{self, BinOp, Block, Cc, Imm, Inst, MathOp, Term, Ty};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Class {
    Int,
    Float,
}

fn cc(c: Cmp) -> Cc {
    match c {
        Cmp::Eq => Cc::Eq,
        Cmp::Ne => Cc::Ne,
        Cmp::Lt => Cc::Lt,
        Cmp::Le => Cc::Le,
        Cmp::Gt => Cc::Gt,
        Cmp::Ge => Cc::Ge,
    }
}

/// Compiled to native code (no VM callback).
fn light(ins: &Ins) -> bool {
    use Ins::*;
    match ins {
        // native code wraps to INTEGER / LONG / _INTEGER64 only
        FToI { w, .. } | Wrap { w, .. } | IDiv { w, .. } | Mod { w, .. } if !matches!(w, W::W16 | W::W32 | W::W64) => false,
        Ldi { .. } | Ldk { .. } | Mov { .. } | GetG { .. } | SetG { .. } | IToF { .. } | FToI { .. } | Wrap { .. }
        | RoundF32 { .. } | AddI16 { .. } | AddI32 { .. } | AddI64 { .. } | SubI16 { .. } | SubI32 { .. }
        | SubI64 { .. } | MulI16 { .. } | MulI32 { .. } | MulI64 { .. } | IDiv { .. } | Mod { .. } | And { .. }
        | Or { .. } | Xor { .. } | Eqv { .. } | Imp { .. } | AddF { .. } | SubF { .. } | MulF { .. } | DivF { .. }
        | PowF { .. } | CmpI { .. } | CmpF { .. } | Jmp { .. } | Loop { .. } | JzI { .. } | JnzI { .. }
        | JzF { .. } | JnzF { .. } | BrI { .. } | BrF { .. } | ForTestI { .. } | ForTestF { .. }
        | ForNextI { .. } | LdE1 { .. } | StE1 { .. } | Gosub { .. } | Return => true,
        LdE { n, .. } | StE { n, .. } => *n <= 2,
        Un { op, .. } => !matches!(op, bc::Un::Log | bc::Un::Exp | bc::Un::Sin | bc::Un::Cos | bc::Un::Tan | bc::Un::Atn),
        _ => false,
    }
}

/// Loop headers where native code would mostly call back into the VM
/// (string-heavy loops): entering there is slower than interpreting.
fn vm_bound_headers(m: &bc::Module) -> Vec<bool> {
    let mut v = vec![false; m.code.len()];
    for (pc, ins) in m.code.iter().enumerate() {
        let t = match *ins {
            Ins::Loop { t } => t,
            Ins::ForNextI { body, .. } => body,
            _ => continue,
        };
        if t as usize > pc {
            continue;
        }
        let body = &m.code[t as usize..=pc];
        let heavy = body.iter().filter(|i| !light(i) && !exits(i) && !matches!(i, Ins::Call { .. })).count();
        let lite = body.iter().filter(|i| light(i)).count();
        // Each VM callback costs about as much as interpreting several numeric instructions.
        if heavy * 4 > lite {
            v[t as usize] = true;
        }
    }
    v
}

/// Straight-line VM instructions native code may hand to the VM one at a
/// time (`VmOp`): they only touch the current frame's banks and runtime
/// state, never frames, the GOSUB stack or the pc. Anything not listed
/// (including instructions added to the VM later) makes native code stop
/// and the VM continue at that pc.
fn vm_safe(ins: &Ins) -> bool {
    use Ins::*;
    matches!(
        ins,
        MovS { .. }
            | TakeS { .. }
            | GetGS { .. }
            | SetGS { .. }
            | Concat { .. }
            | LenS { .. }
            | Chr { .. }
            | Asc { .. }
            | Left { .. }
            | Right { .. }
            | Mid { .. }
            | Bi { .. }
            | PrintS { .. }
            | PrintN { .. }
            | PrintZone
            | PrintNl
            | CmpS { .. }
            | Dim { .. }
            | LdE { .. }
            | StE { .. }
            | LdES { .. }
            | StES { .. }
            | LdE1 { .. }
            | StE1 { .. }
            | LdE1S { .. }
            | StE1S { .. }
            | Bound { .. }
            | Swap { .. }
            | Read { .. }
            | Restore { .. }
            | Randomize { .. }
            | Rnd { .. }
            | Err { .. }
            | Erl { .. }
            | Raise { .. }
            | Un { .. }
            | Call { .. }
    )
}

/// Control the VM owns: native code stops here and the VM continues at this pc.
fn exits(ins: &Ins) -> bool {
    use Ins::*;
    matches!(
        ins,
        Ret
            | End
            | OnError { .. }
            | Resume { .. }
            | OnEvent { .. }
            | EventCtl { .. }
            | OnTimer { .. }
            | BrS { .. }
    )
}

/// Jump targets inside the function (None for instructions without one).
fn targets(ins: &Ins) -> Vec<u32> {
    let mut v = Vec::new();
    ins.operands(&mut |o| {
        if let Opnd::T(t) = o {
            v.push(t)
        }
    });
    v
}

fn ends_block(ins: &Ins) -> bool {
    use Ins::*;
    exits(ins)
        || matches!(
            ins,
            Jmp { .. }
                | Loop { .. }
                | JzI { .. }
                | JnzI { .. }
                | JzF { .. }
                | JnzF { .. }
                | BrI { .. }
                | BrF { .. }
                | ForTestI { .. }
                | ForTestF { .. }
                | ForNextI { .. }
                | RaiseK { .. }
                | Gosub { .. }
                | Return
        )
}

/// Numeric registers of the current frame an instruction may read or write
/// (for VmOp register flushing). `is_main`: module-level absolute operands
/// alias frame registers.
fn frame_regs(m: &bc::Module, ins: &Ins, n: u32, is_main: bool) -> Vec<u32> {
    let mut v = BTreeSet::new();
    let mut add = |r: u32| {
        if r < n {
            v.insert(r);
        }
    };
    if let Ins::Call { f, desc } = ins {
        let c = &m.calls[*desc as usize];
        for a in &c.args {
            match *a {
                bc::Arg::N(r) => add(r),
                bc::Arg::Ref(p) => match m.places[p as usize] {
                    bc::Place::N(r, _) => add(r),
                    bc::Place::NG(g, _) if is_main => add(g),
                    bc::Place::E { i, n: k, .. } => (i..i + k).for_each(&mut add),
                    _ => {}
                },
                _ => {}
            }
        }
        if let (Some(d), Some((bc::Slot::N(_), _))) = (c.dest, m.funcs[*f as usize].ret) {
            add(d);
        }
    }
    if let Ins::Bi { f, d, .. } = ins {
        if m.bfns[*f as usize].1 != bc::Ty::Str {
            add(*d);
        }
    }
    ins.operands(&mut |o| match o {
        Opnd::N(r) => add(r),
        Opnd::NRun(r, k) => (r..r.saturating_add(k)).for_each(&mut add),
        Opnd::NG(g) if is_main => add(g),
        Opnd::BArgs(k) => {
            for (r, t) in &m.bargs[k as usize] {
                if *t != bc::Ty::Str {
                    add(*r)
                }
            }
        }
        Opnd::Place(p) => match m.places[p as usize] {
            bc::Place::N(r, _) => add(r),
            bc::Place::NG(g, _) if is_main => add(g),
            bc::Place::E { i, n: k, .. } => (i..i + k).for_each(&mut add),
            _ => {}
        },
        _ => {}
    });
    v.into_iter().collect()
}

/// Per-register class from the instructions that use it with a type.
fn classes(m: &bc::Module, f: &bc::Func) -> Vec<Class> {
    let n = f.n_num() as usize;
    let mut int = vec![false; n];
    let mut flt = vec![false; n];
    let code = &m.code[f.entry as usize..f.end as usize];
    let mark = |r: u32, v: &mut Vec<bool>| {
        if (r as usize) < n {
            v[r as usize] = true
        }
    };
    for ins in code {
        use Ins::*;
        let (iv, fv): (Vec<u32>, Vec<u32>) = match *ins {
            IToF { d, s } => (vec![s], vec![d]),
            FToI { d, s, .. } => (vec![d], vec![s]),
            Wrap { d, s, .. } => (vec![d, s], vec![]),
            RoundF32 { d, s } => (vec![], vec![d, s]),
            AddI16 { d, a, b }
            | AddI32 { d, a, b }
            | AddI64 { d, a, b }
            | SubI16 { d, a, b }
            | SubI32 { d, a, b }
            | SubI64 { d, a, b }
            | MulI16 { d, a, b }
            | MulI32 { d, a, b }
            | MulI64 { d, a, b }
            | IDiv { d, a, b, .. }
            | Mod { d, a, b, .. }
            | And { d, a, b }
            | Or { d, a, b }
            | Xor { d, a, b }
            | Eqv { d, a, b }
            | Imp { d, a, b } => (vec![d, a, b], vec![]),
            AddF { d, a, b } | SubF { d, a, b } | MulF { d, a, b } | DivF { d, a, b } | PowF { d, a, b } => {
                (vec![], vec![d, a, b])
            }
            CmpI { d, a, b, .. } => (vec![d, a, b], vec![]),
            CmpF { d, a, b, .. } => (vec![d], vec![a, b]),
            JzI { s, .. } | JnzI { s, .. } => (vec![s], vec![]),
            JzF { s, .. } | JnzF { s, .. } => (vec![], vec![s]),
            BrI { a, b, .. } => (vec![a, b], vec![]),
            BrF { a, b, .. } => (vec![], vec![a, b]),
            ForTestI { v, lim, step, .. } => (vec![v, lim, step], vec![]),
            ForTestF { v, lim, step, .. } => (vec![], vec![v, lim, step]),
            ForNextI { v, step, lim, .. } => (vec![v, step, lim], vec![]),
            Un { op, d, s } => match op {
                bc::Un::NegI(_) | bc::Un::Not | bc::Un::AbsI(_) | bc::Un::SgnI => (vec![d, s], vec![]),
                bc::Un::SgnF => (vec![d], vec![s]),
                _ => (vec![], vec![d, s]),
            },
            _ => (vec![], vec![]),
        };
        for r in iv {
            mark(r, &mut int);
        }
        for r in fv {
            mark(r, &mut flt);
        }
    }
    // Registers only moved around take the class of what they meet.
    let mut cls: Vec<Option<Class>> = (0..n)
        .map(|r| match (int[r], flt[r]) {
            (_, false) if int[r] => Some(Class::Int),
            (false, true) => Some(Class::Float),
            (true, true) => Some(Class::Int), // conflicting: raw bits, bitcast at float uses
            _ => None,
        })
        .collect();
    for _ in 0..4 {
        for ins in code {
            if let Ins::Mov { d, s } = *ins {
                let (d, s) = (d as usize, s as usize);
                if d < n && s < n {
                    match (cls[d], cls[s]) {
                        (None, Some(c)) => cls[d] = Some(c),
                        (Some(c), None) => cls[s] = Some(c),
                        _ => {}
                    }
                }
            }
        }
    }
    cls.into_iter().map(|c| c.unwrap_or(Class::Int)).collect()
}

struct Lower<'a> {
    m: &'a bc::Module,
    fi: usize,
    cls: Vec<Class>,
    /// main module classes (types of global slots)
    gcls: &'a [Class],
    slots: Vec<Ty>,
    insts: Vec<Inst>,
    view_off: u32,
    /// VM function -> MIR index of its native call version (None: VM calls it)
    natcall: &'a [Option<u32>],
}

fn ty_of(c: Class) -> Ty {
    match c {
        Class::Int => Ty::I64,
        Class::Float => Ty::F64,
    }
}

impl Lower<'_> {
    fn tmp(&mut self, c: Class) -> u32 {
        self.cls.push(c);
        self.slots.push(ty_of(c));
        (self.slots.len() - 1) as u32
    }

    fn is_main(&self) -> bool {
        self.fi == 0
    }

    /// Register `r` viewed as class `c` (bitcast into a temporary when it differs).
    fn get(&mut self, r: u32, c: Class) -> u32 {
        if self.cls[r as usize] == c {
            r
        } else {
            let t = self.tmp(c);
            self.insts.push(Inst::Bitcast { dst: t, src: r });
            t
        }
    }

    /// Destination of class `c` for register `r`; call `put` after writing it.
    fn dst(&mut self, r: u32, c: Class) -> u32 {
        if self.cls[r as usize] == c {
            r
        } else {
            self.tmp(c)
        }
    }

    fn put(&mut self, r: u32, d: u32) {
        if d != r {
            self.insts.push(Inst::Bitcast { dst: r, src: d });
        }
    }

    fn konst_i(&mut self, v: i64) -> u32 {
        let t = self.tmp(Class::Int);
        self.insts.push(Inst::Const { dst: t, imm: Imm::I(v) });
        t
    }

    fn konst_f(&mut self, v: f64) -> u32 {
        let t = self.tmp(Class::Float);
        self.insts.push(Inst::Const { dst: t, imm: Imm::F(v) });
        t
    }

    fn raw_const(&mut self, d: u32, bits: u64) {
        let imm = match self.cls[d as usize] {
            Class::Int => Imm::I(bits as i64),
            Class::Float => Imm::F(f64::from_bits(bits)),
        };
        self.insts.push(Inst::Const { dst: d, imm });
    }

    fn wrap(&mut self, d: u32, w: W) {
        let op = match w {
            W::W16 => MathOp::Wrap16,
            W::W32 => MathOp::Wrap32,
            _ => return, // W64; other widths never reach native code (see `light`)
        };
        self.insts.push(Inst::Math { op, dst: d, src: d });
    }

    fn ibin(&mut self, op: BinOp, d: u32, a: u32, b: u32, w: W) {
        let (a, b) = (self.get(a, Class::Int), self.get(b, Class::Int));
        let dd = self.dst(d, Class::Int);
        self.insts.push(Inst::Bin { op, dst: dd, a, b });
        self.wrap(dd, w);
        self.put(d, dd);
    }

    fn fbin(&mut self, op: BinOp, d: u32, a: u32, b: u32) {
        let (a, b) = (self.get(a, Class::Float), self.get(b, Class::Float));
        let dd = self.dst(d, Class::Float);
        self.insts.push(Inst::Bin { op, dst: dd, a, b });
        self.put(d, dd);
    }

    /// Condition register (int, non-zero = true) for `a c b`.
    fn cmp(&mut self, c: Cmp, a: u32, b: u32, cls: Class) -> u32 {
        let (a, b) = (self.get(a, cls), self.get(b, cls));
        let t = self.tmp(Class::Int);
        self.insts.push(Inst::Cmp { cc: cc(c), dst: t, a, b });
        t
    }

    /// FOR exit condition: step < 0 ? v < lim : v > lim
    fn for_exit(&mut self, v: u32, lim: u32, step: u32, cls: Class) -> u32 {
        let zero = match cls {
            Class::Int => self.konst_i(0),
            Class::Float => self.konst_f(0.0),
        };
        let neg = self.cmp(Cmp::Lt, step, zero, cls);
        let below = self.cmp(Cmp::Lt, v, lim, cls);
        let above = self.cmp(Cmp::Gt, v, lim, cls);
        let t1 = self.tmp(Class::Int);
        self.insts.push(Inst::Bin { op: BinOp::And, dst: t1, a: neg, b: below });
        let nn = self.tmp(Class::Int);
        self.insts.push(Inst::Not { dst: nn, src: neg });
        let t2 = self.tmp(Class::Int);
        self.insts.push(Inst::Bin { op: BinOp::And, dst: t2, a: nn, b: above });
        let r = self.tmp(Class::Int);
        self.insts.push(Inst::Bin { op: BinOp::Or, dst: r, a: t1, b: t2 });
        r
    }

    /// CALL of a numeric-only function: arguments copied into the callee
    /// frame (BYREF copied back after a normal return), like `Vm::call`/`ret`.
    fn native_call(&mut self, pc: u32, f: u32, desc: u32) {
        let callee = &self.m.funcs[f as usize];
        let ccls = classes(self.m, callee);
        let cd = &self.m.calls[desc as usize];
        let mut args = Vec::new();
        let mut back = Vec::new();
        let mut after = Vec::new();
        for (a, slot) in cd.args.iter().zip(&callee.params) {
            let bc::Slot::N(ps) = *slot else { unreachable!("numeric callee") };
            let pc_cls = ccls[ps as usize];
            match *a {
                bc::Arg::N(r) => args.push(mir::Arg::Val(self.get(r, pc_cls))),
                bc::Arg::Ref(p) => match self.m.places[p as usize] {
                    bc::Place::N(r, _) => {
                        args.push(mir::Arg::Val(self.get(r, pc_cls)));
                        let d = self.dst(r, pc_cls);
                        back.push((ps, d));
                        if d != r {
                            after.push(Inst::Bitcast { dst: r, src: d });
                        }
                    }
                    bc::Place::NG(g, _) if self.is_main() => {
                        args.push(mir::Arg::Val(self.get(g, pc_cls)));
                        let d = self.dst(g, pc_cls);
                        back.push((ps, d));
                        if d != g {
                            after.push(Inst::Bitcast { dst: g, src: d });
                        }
                    }
                    bc::Place::NG(g, _) => {
                        let gc = self.global(g);
                        let t = self.tmp(gc);
                        self.insts.push(Inst::GLoad { dst: t, slot: g });
                        args.push(mir::Arg::Val(self.get(t, pc_cls)));
                        let d = self.tmp(pc_cls);
                        back.push((ps, d));
                        let v = self.get(d, gc);
                        // `get` emitted its bitcast now; it must run after the call.
                        if v != d {
                            let bc_inst = self.insts.pop().expect("bitcast");
                            after.push(bc_inst);
                        }
                        after.push(Inst::GStore { slot: g, src: v });
                    }
                    _ => unreachable!("checked by call_args_native"),
                },
                _ => unreachable!("checked by call_args_native"),
            }
        }
        let dst = match (cd.dest, callee.ret) {
            (Some(d), Some((bc::Slot::N(r), _))) => {
                let c = ccls[r as usize];
                let dd = self.dst(d, c);
                if dd != d {
                    after.push(Inst::Bitcast { dst: d, src: dd });
                }
                Some(dd)
            }
            _ => None,
        };
        let func = self.natcall[f as usize].expect("native callee");
        self.insts.push(Inst::Call { func, args, dst, pc, back });
        self.insts.extend(after);
    }

    /// Element type of numeric array operand `a` (None for string arrays).
    fn elem_ty(&self, a: u32) -> Option<Ty> {
        let (f, k) = if a & bc::G != 0 { (0, a & !bc::G) } else { (self.fi, a) };
        match self.m.funcs[f].arr_tys.get(k as usize)? {
            bc::Ty::I16 => Some(Ty::I16),
            bc::Ty::I32 => Some(Ty::I32),
            bc::Ty::I64 => Some(Ty::I64),
            bc::Ty::F32 => Some(Ty::F32),
            bc::Ty::F64 => Some(Ty::F64),
            bc::Ty::Str | bc::Ty::F80 => None,
        }
    }

    fn global(&mut self, g: u32) -> Class {
        self.gcls.get(g as usize).copied().unwrap_or(Class::Int)
    }

    /// One straight-line instruction at `pc`.
    fn inst(&mut self, pc: u32, ins: &Ins) {
        use Ins::*;
        match *ins {
            Ldi { d, v } => self.raw_const(d, v as i64 as u64),
            Ldk { d, k } => self.raw_const(d, self.m.nconst[k as usize]),
            Mov { d, s } => {
                if self.cls[d as usize] == self.cls[s as usize] {
                    self.insts.push(Inst::Mov { dst: d, src: s })
                } else {
                    self.insts.push(Inst::Bitcast { dst: d, src: s })
                }
            }
            GetG { d, g } => {
                if self.is_main() {
                    self.inst(pc, &Mov { d, s: g });
                } else {
                    let gc = self.global(g);
                    let t = self.dst(d, gc);
                    self.insts.push(Inst::GLoad { dst: t, slot: g });
                    if self.cls[d as usize] != gc {
                        self.insts.push(Inst::Bitcast { dst: d, src: t });
                    }
                }
            }
            SetG { g, s } => {
                if self.is_main() {
                    self.inst(pc, &Mov { d: g, s });
                } else {
                    let gc = self.global(g);
                    let v = self.get(s, gc);
                    self.insts.push(Inst::GStore { slot: g, src: v });
                }
            }
            IToF { d, s } => {
                let s = self.get(s, Class::Int);
                let dd = self.dst(d, Class::Float);
                self.insts.push(Inst::Conv { dst: dd, src: s });
                self.put(d, dd);
            }
            FToI { d, s, w } => {
                self.insts.push(Inst::At { pc });
                let s = self.get(s, Class::Float);
                let dd = self.dst(d, Class::Int);
                match w {
                    W::W64 => self.insts.push(Inst::Conv { dst: dd, src: s }),
                    _ => {
                        self.cls.push(Class::Int);
                        self.slots.push(if w == W::W16 { Ty::I16 } else { Ty::I32 });
                        let n = (self.slots.len() - 1) as u32;
                        self.insts.push(Inst::Conv { dst: n, src: s });
                        self.insts.push(Inst::Conv { dst: dd, src: n });
                    }
                }
                self.put(d, dd);
            }
            Wrap { d, s, w } => {
                let s = self.get(s, Class::Int);
                let dd = self.dst(d, Class::Int);
                self.insts.push(Inst::Mov { dst: dd, src: s });
                self.wrap(dd, w);
                self.put(d, dd);
            }
            RoundF32 { d, s } => {
                let s = self.get(s, Class::Float);
                let dd = self.dst(d, Class::Float);
                self.insts.push(Inst::Math { op: MathOp::RoundF32, dst: dd, src: s });
                self.put(d, dd);
            }
            AddI16 { d, a, b } => self.ibin(BinOp::Add, d, a, b, W::W16),
            AddI32 { d, a, b } => self.ibin(BinOp::Add, d, a, b, W::W32),
            AddI64 { d, a, b } => self.ibin(BinOp::Add, d, a, b, W::W64),
            SubI16 { d, a, b } => self.ibin(BinOp::Sub, d, a, b, W::W16),
            SubI32 { d, a, b } => self.ibin(BinOp::Sub, d, a, b, W::W32),
            SubI64 { d, a, b } => self.ibin(BinOp::Sub, d, a, b, W::W64),
            MulI16 { d, a, b } => self.ibin(BinOp::Mul, d, a, b, W::W16),
            MulI32 { d, a, b } => self.ibin(BinOp::Mul, d, a, b, W::W32),
            MulI64 { d, a, b } => self.ibin(BinOp::Mul, d, a, b, W::W64),
            IDiv { d, a, b, w } => {
                self.insts.push(Inst::At { pc });
                self.ibin(BinOp::IDiv, d, a, b, w)
            }
            Mod { d, a, b, w } => {
                self.insts.push(Inst::At { pc });
                self.ibin(BinOp::Mod, d, a, b, w)
            }
            And { d, a, b } => self.ibin(BinOp::And, d, a, b, W::W64),
            Or { d, a, b } => self.ibin(BinOp::Or, d, a, b, W::W64),
            Xor { d, a, b } => self.ibin(BinOp::Xor, d, a, b, W::W64),
            Eqv { d, a, b } => self.ibin(BinOp::Eqv, d, a, b, W::W64),
            Imp { d, a, b } => self.ibin(BinOp::Imp, d, a, b, W::W64),
            AddF { d, a, b } => self.fbin(BinOp::Add, d, a, b),
            SubF { d, a, b } => self.fbin(BinOp::Sub, d, a, b),
            MulF { d, a, b } => self.fbin(BinOp::Mul, d, a, b),
            DivF { d, a, b } => {
                self.insts.push(Inst::At { pc });
                self.fbin(BinOp::Div, d, a, b)
            }
            PowF { d, a, b } => self.fbin(BinOp::Pow, d, a, b),
            CmpI { c, d, a, b } | CmpF { c, d, a, b } => {
                let cls = if matches!(ins, CmpI { .. }) { Class::Int } else { Class::Float };
                let t = self.cmp(c, a, b, cls);
                let dd = self.dst(d, Class::Int);
                self.insts.push(Inst::Mov { dst: dd, src: t });
                self.put(d, dd);
            }
            Un { op, d, s } => {
                let (sc, dc) = match op {
                    bc::Un::NegI(_) | bc::Un::Not | bc::Un::AbsI(_) | bc::Un::SgnI => (Class::Int, Class::Int),
                    bc::Un::SgnF => (Class::Float, Class::Int),
                    _ => (Class::Float, Class::Float),
                };
                let s = self.get(s, sc);
                let dd = self.dst(d, dc);
                match op {
                    bc::Un::NegI(w) => {
                        self.insts.push(Inst::Neg { dst: dd, src: s });
                        self.wrap(dd, w);
                    }
                    bc::Un::NegF => self.insts.push(Inst::Neg { dst: dd, src: s }),
                    bc::Un::Not => self.insts.push(Inst::Not { dst: dd, src: s }),
                    bc::Un::AbsI(w) => {
                        self.insts.push(Inst::Math { op: MathOp::Abs, dst: dd, src: s });
                        self.wrap(dd, w);
                    }
                    bc::Un::AbsF => self.insts.push(Inst::Math { op: MathOp::Abs, dst: dd, src: s }),
                    bc::Un::SgnI | bc::Un::SgnF => self.insts.push(Inst::Math { op: MathOp::Sgn, dst: dd, src: s }),
                    bc::Un::Int => self.insts.push(Inst::Math { op: MathOp::Floor, dst: dd, src: s }),
                    bc::Un::Fix => self.insts.push(Inst::Math { op: MathOp::Fix, dst: dd, src: s }),
                    bc::Un::Sqr => {
                        self.insts.push(Inst::At { pc });
                        self.insts.push(Inst::Math { op: MathOp::Sqrt, dst: dd, src: s })
                    }
                    _ => unreachable!("not native"),
                }
                self.put(d, dd);
            }
            LdE1 { d, a, i } | StE1 { a, i, s: d } if self.elem_ty(a).is_some() => {
                let load = matches!(ins, LdE1 { .. });
                let et = self.elem_ty(a).expect("numeric");
                let c = if et.is_float() { Class::Float } else { Class::Int };
                self.insts.push(Inst::At { pc });
                let ir = self.get(i, Class::Int);
                let global = a & bc::G != 0;
                let slow = match (load, global) {
                    (true, false) => H_LDE1,
                    (true, true) => H_LDE1G,
                    (false, false) => H_STE1,
                    (false, true) => H_STE1G,
                };
                let (table, index, view_off) = (u32::from(global), a & !bc::G, self.view_off);
                if load {
                    let dd = self.dst(d, c);
                    self.insts.push(Inst::VAElem { load, reg: dd, table, index, view_off, elem: et, idx: ir, slow });
                    self.put(d, dd);
                } else {
                    let sr = self.get(d, c);
                    self.insts.push(Inst::VAElem { load, reg: sr, table, index, view_off, elem: et, idx: ir, slow });
                }
            }
            LdE1 { d, a, i } | LdE { d, a, i, n: 1 } => {
                self.insts.push(Inst::At { pc });
                let (ar, ir) = (self.konst_i(i64::from(a)), self.get(i, Class::Int));
                let dd = self.dst(d, Class::Int);
                self.insts.push(Inst::Host { f: H_LDE1, args: vec![ar, ir], dst: Some(dd) });
                self.put(d, dd);
            }
            StE1 { a, i, s } | StE { a, i, n: 1, s } => {
                self.insts.push(Inst::At { pc });
                let (ar, ir, sr) = (self.konst_i(i64::from(a)), self.get(i, Class::Int), self.get(s, Class::Int));
                self.insts.push(Inst::Host { f: H_STE1, args: vec![ar, ir, sr], dst: None });
            }
            LdE { d, a, i, n: 2 } => {
                self.insts.push(Inst::At { pc });
                let ar = self.konst_i(i64::from(a));
                let (i1, i2) = (self.get(i, Class::Int), self.get(i + 1, Class::Int));
                let dd = self.dst(d, Class::Int);
                self.insts.push(Inst::Host { f: H_LDE2, args: vec![ar, i1, i2], dst: Some(dd) });
                self.put(d, dd);
            }
            StE { a, i, n: 2, s } => {
                self.insts.push(Inst::At { pc });
                let ar = self.konst_i(i64::from(a));
                let (i1, i2, sr) = (self.get(i, Class::Int), self.get(i + 1, Class::Int), self.get(s, Class::Int));
                self.insts.push(Inst::Host { f: H_STE2, args: vec![ar, i1, i2, sr], dst: None });
            }
            Call { f, desc } if self.natcall[f as usize].is_some() && call_args_native(self.m, desc, self.is_main()) => {
                self.native_call(pc, f, desc)
            }
            _ => {
                let regs = frame_regs(self.m, ins, self.m.funcs[self.fi].n_num(), self.is_main());
                self.insts.push(Inst::VmOp { op: pc, reads: regs.clone(), writes: regs });
            }
        }
    }
}

/// Main-module registers that code outside the main module's own native
/// code can change while main is in a VM call: SHARED variables (absolute
/// operands of other functions) and, when the program uses events, all of
/// them (handlers run main code at any safe point).
fn main_shared(m: &bc::Module) -> Vec<u32> {
    let n = m.funcs[0].n_num();
    let events = m.code.iter().any(|i| matches!(i, Ins::OnEvent { .. } | Ins::OnTimer { .. }));
    if events {
        return (0..n).collect();
    }
    let mut v = BTreeSet::new();
    for f in &m.funcs[1..] {
        for ins in &m.code[f.entry as usize..f.end as usize] {
            ins.operands(&mut |o| match o {
                Opnd::NG(g) if g < n => {
                    v.insert(g);
                }
                Opnd::Place(p) => {
                    if let bc::Place::NG(g, _) = m.places[p as usize] {
                        v.insert(g);
                    }
                }
                _ => {}
            });
            if let Ins::Call { desc, .. } = ins {
                for a in &m.calls[*desc as usize].args {
                    if let bc::Arg::Ref(p) = *a {
                        if let bc::Place::NG(g, _) = m.places[p as usize] {
                            v.insert(g);
                        }
                    }
                }
            }
        }
    }
    v.into_iter().filter(|&g| g < n).collect()
}

/// MIR of the whole program with the VM's host table.
pub(crate) fn lower_full(m: &bc::Module, hosts: &Hosts) -> mir::Module {
    let mut mm = lower(m, hosts.vm_op as *const () as usize, hosts.view_off);
    mm.hosts = host_table(hosts);
    mm
}

/// Native image of program `m` for this executable (AOT, D15).
pub(crate) fn build_image(m: &bc::Module, hosts: &Hosts) -> Result<Vec<u8>, String> {
    let mir = lower_full(m, hosts);
    let obj = qb_jit::aot::compile_object(&mir, qb_jit::Target::Host).map_err(|e| e.0)?;
    let img = qb_link::link(&obj).map_err(|e| e.to_string())?;
    let mut out = fingerprint(m, hosts).to_le_bytes().to_vec();
    out.extend_from_slice(&img.to_bytes());
    Ok(out)
}

/// Identifies the program and the runner layout an image was built for
/// (addresses are not part of it: they are resolved by name at load time).
fn fingerprint(m: &bc::Module, hosts: &Hosts) -> u64 {
    let text = format!("{} {} {:?}", env!("CARGO_PKG_VERSION"), hosts.view_off, m);
    text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

/// Lowers every function of `m`. `vm_op` = address of the VM step callback.
/// Arguments a native call can pass: numbers and BYREF numeric variables.
fn call_args_native(m: &bc::Module, desc: u32, _is_main: bool) -> bool {
    m.calls[desc as usize].args.iter().all(|a| match *a {
        bc::Arg::N(_) => true,
        bc::Arg::Ref(p) => matches!(m.places[p as usize], bc::Place::N(..) | bc::Place::NG(..)),
        _ => false,
    })
}

/// Functions that can run entirely as native code with their frame in the
/// JIT arena: numbers only (no string/array locals, not STATIC), and every
/// instruction is native, a VM exit, or a call of another such function.
fn native_callees(m: &bc::Module) -> Vec<bool> {
    let mut ok: Vec<bool> = m
        .funcs
        .iter()
        .enumerate()
        .map(|(i, f)| {
            i != 0
                && f.n_str == 0
                && f.n_arr() == 0
                && !f.is_static
                && f.params.iter().all(|p| matches!(p, bc::Slot::N(_)))
                && matches!(f.ret, None | Some((bc::Slot::N(_), _)))
        })
        .collect();
    loop {
        let mut changed = false;
        for (i, f) in m.funcs.iter().enumerate() {
            if !ok[i] {
                continue;
            }
            let good = m.code[f.entry as usize..f.end as usize].iter().all(|ins| match *ins {
                Ins::Call { f: g, desc } => ok[g as usize] && call_args_native(m, desc, false),
                Ins::RaiseK { .. } | Ins::Gosub { .. } | Ins::Return => true,
                ref x => light(x) || exits(x) || !vm_safe(x),
            });
            if !good {
                ok[i] = false;
                changed = true;
            }
        }
        if !changed {
            return ok;
        }
    }
}

pub(crate) fn lower(m: &bc::Module, vm_op: usize, view_off: u32) -> mir::Module {
    let gcls = classes(m, &m.funcs[0]);
    let simple = native_callees(m);
    let n = m.funcs.len();
    let mut natcall = vec![None; n];
    let mut k = n as u32;
    for (i, s) in simple.iter().enumerate() {
        if *s {
            natcall[i] = Some(k);
            k += 1;
        }
    }
    let mut funcs = Vec::new();
    let order: Vec<(usize, bool)> =
        (0..n).map(|i| (i, false)).chain((0..n).filter(|&i| simple[i]).map(|i| (i, true))).collect();
    for (fi, callver) in order {
        let f = &m.funcs[fi];
        let cls = if fi == 0 { gcls.clone() } else { classes(m, f) };
        let (lo, hi) = (f.entry, f.end);
        let code = &m.code[lo as usize..hi as usize];
        // Block leaders.
        let mut leaders = BTreeSet::new();
        leaders.insert(lo);
        for (k, ins) in code.iter().enumerate() {
            let pc = lo + k as u32;
            for t in targets(ins) {
                if (lo..hi).contains(&t) {
                    leaders.insert(t);
                }
            }
            if ends_block(ins) && pc + 1 < hi {
                leaders.insert(pc + 1);
            }
        }
        let leaders: Vec<u32> = leaders.into_iter().collect();
        // A call version starts with a prologue block (the frame's constants).
        let base = u32::from(callver);
        let bidx = |pc: u32| leaders.binary_search(&pc).ok().map(|i| i as u32 + base);
        let mut lw = Lower {
            m,
            fi,
            slots: cls.iter().map(|c| ty_of(*c)).collect(),
            cls,
            gcls: &gcls,
            insts: vec![],
            view_off,
            natcall: &natcall,
        };
        let mut blocks = Vec::new();
        if callver {
            let params: Vec<u32> = f.params.iter().map(|p| if let bc::Slot::N(r) = *p { r } else { unreachable!() }).collect();
            for (r, &bits) in f.num_init.iter().enumerate() {
                if bits != 0 && !params.contains(&(r as u32)) {
                    lw.raw_const(r as u32, bits);
                }
            }
            blocks.push(Block { pc: None, insts: std::mem::take(&mut lw.insts), term: Term::Jmp(1) });
        }
        if fi == 0 && !callver {
            // Keep these registers in memory (an address-taken slot is never cached).
            for g in main_shared(m) {
                lw.slots.push(Ty::Ptr);
                lw.cls.push(Class::Int);
                let t = (lw.slots.len() - 1) as u32;
                lw.insts.push(Inst::AddrOf { dst: t, slot: g });
            }
        }
        for (bi, &start) in leaders.iter().enumerate() {
            let end = leaders.get(bi + 1).copied().unwrap_or(hi);
            let mut term = None;
            for pc in start..end {
                let ins = &m.code[pc as usize];
                let next = pc + 1;
                // Target block or an exit when the target is outside the function.
                let blk = |t: u32| bidx(t);
                let branch = |_: &mut Lower, cond: u32, then_pc: u32, else_pc: u32| -> Term {
                    match (blk(then_pc), blk(else_pc)) {
                        (Some(t), Some(e)) => Term::Br { cond, then: t, els: e },
                        _ => Term::Exit { pc },
                    }
                };
                use Ins::*;
                let t: Option<Term> = if callver && matches!(ins, Ret) {
                    Some(Term::Ret)
                } else if callver && matches!(ins, Gosub { .. } | Return) {
                    // the VM's GOSUB stack belongs to VM frames
                    Some(Term::Exit { pc })
                } else if exits(ins) || !(light(ins) || vm_safe(ins) || matches!(ins, RaiseK { .. } | Gosub { .. } | Return)) {
                    Some(Term::Exit { pc })
                } else {
                    match *ins {
                        Jmp { t } | Loop { t } => Some(match blk(t) {
                            Some(b) => Term::Jmp(b),
                            None => Term::Exit { pc: t },
                        }),
                        JzI { s, t } | JnzI { s, t } => {
                            let c = lw.get(s, Class::Int);
                            let (a, b) = if matches!(ins, JnzI { .. }) { (t, next) } else { (next, t) };
                            Some(branch(&mut lw, c, a, b))
                        }
                        JzF { s, t } | JnzF { s, t } => {
                            let z = lw.konst_f(0.0);
                            let c = lw.cmp(Cmp::Ne, s, z, Class::Float);
                            let (a, b) = if matches!(ins, JnzF { .. }) { (t, next) } else { (next, t) };
                            Some(branch(&mut lw, c, a, b))
                        }
                        BrI { c, a, b, t } | BrF { c, a, b, t } => {
                            let cls = if matches!(ins, BrI { .. }) { Class::Int } else { Class::Float };
                            let r = lw.cmp(c, a, b, cls);
                            Some(branch(&mut lw, r, t, next))
                        }
                        ForTestI { v, lim, step, t } | ForTestF { v, lim, step, t } => {
                            let cls = if matches!(ins, ForTestI { .. }) { Class::Int } else { Class::Float };
                            let r = lw.for_exit(v, lim, step, cls);
                            Some(branch(&mut lw, r, t, next))
                        }
                        ForNextI { v, step, lim, body, w } => {
                            lw.ibin(BinOp::Add, v, v, step, w);
                            // continue while !(step < 0 ? v < lim : v > lim)
                            let ex = lw.for_exit(v, lim, step, Class::Int);
                            Some(branch(&mut lw, ex, next, body))
                        }
                        Gosub { t } => {
                            let r = lw.konst_i(i64::from(next));
                            lw.insts.push(Inst::Host { f: H_GOSUB, args: vec![r], dst: None });
                            Some(match blk(t) {
                                Some(b) => Term::Jmp(b),
                                None => Term::Exit { pc: t },
                            })
                        }
                        Return => {
                            lw.insts.push(Inst::At { pc });
                            let at = lw.konst_i(i64::from(pc));
                            let r = lw.tmp(Class::Int);
                            lw.insts.push(Inst::Host { f: H_RETURN, args: vec![at], dst: Some(r) });
                            let cases = code
                                .iter()
                                .enumerate()
                                .filter(|(_, x)| matches!(x, Gosub { .. }))
                                .filter_map(|(k, _)| {
                                    let rp = lo + k as u32 + 1;
                                    blk(rp).map(|b| (rp, b))
                                })
                                .collect();
                            Some(Term::Dispatch { val: r, cases })
                        }
                        RaiseK { code } => {
                            lw.insts.push(Inst::At { pc });
                            let r = lw.konst_i(i64::from(code));
                            lw.insts.push(Inst::Raise { code: r });
                            Some(Term::Exit { pc })
                        }
                        _ => {
                            lw.inst(pc, ins);
                            None
                        }
                    }
                };
                if let Some(t) = t {
                    term = Some(t);
                    break;
                }
            }
            let term = term.unwrap_or_else(|| match bidx(end) {
                Some(b) => Term::Jmp(b),
                None => Term::Exit { pc: end },
            });
            blocks.push(Block { pc: Some(start), insts: std::mem::take(&mut lw.insts), term });
        }
        let (params, ret) = if callver {
            (
                f.params.iter().map(|p| if let bc::Slot::N(r) = *p { r } else { unreachable!() }).collect(),
                match f.ret {
                    Some((bc::Slot::N(r), _)) => Some(r),
                    _ => None,
                },
            )
        } else {
            (vec![], None)
        };
        funcs.push(mir::Func {
            name: if callver { format!("{}.call", f.name) } else { f.name.clone() },
            slots: lw.slots,
            params,
            ret,
            blocks,
            frame_len: Some(f.n_num()),
        });
    }
    let globals = gcls.iter().map(|c| ty_of(*c)).collect();
    mir::Module { funcs, hosts: vec![], globals, vm_op }
}

// ---------------------------------------------------------------- tier state

use qb_jit::tier::{Mode, Tiering};
use qb_jit::{status, Ctx};
use core::sync::atomic::AtomicU32;

pub(crate) type VmOpFn = extern "C" fn(*mut Ctx, *mut u64, u32) -> u32;

/// VM entry points native code calls.
pub(crate) struct Hosts {
    pub vm_op: VmOpFn,
    pub lde1: usize,
    pub ste1: usize,
    pub lde2: usize,
    pub ste2: usize,
    pub lde1g: usize,
    pub gosub: usize,
    pub greturn: usize,
    pub ste1g: usize,
    /// offset of the `ArrView` inside the VM's array object
    pub view_off: u32,
}

const H_LDE1: u32 = 0;
const H_STE1: u32 = 1;
const H_LDE2: u32 = 2;
const H_STE2: u32 = 3;
const H_LDE1G: u32 = 4;
const H_STE1G: u32 = 5;
const H_GOSUB: u32 = 6;
const H_RETURN: u32 = 7;

fn host_table(h: &Hosts) -> Vec<mir::HostFn> {
    let f = |name: &str, n: usize, ret: bool, addr: usize| mir::HostFn {
        sig: mir::HostSig { name: name.into(), params: vec![Ty::I64; n], ret: ret.then_some(Ty::I64), may_error: true },
        addr,
    };
    vec![
        f("lde1", 2, true, h.lde1),
        f("ste1", 3, false, h.ste1),
        f("lde2", 3, true, h.lde2),
        f("ste2", 4, false, h.ste2),
        f("lde1g", 2, true, h.lde1g),
        f("ste1g", 3, false, h.ste1g),
        mir::HostFn {
            sig: mir::HostSig { name: "gosub".into(), params: vec![Ty::I64], ret: None, may_error: false },
            addr: h.gosub,
        },
        f("return", 1, true, h.greturn),
    ]
}

/// JIT state owned by a VM.
pub struct JitState {
    tier: Tiering,
    /// Lowered on first use (needs the VM callback address).
    mir: Option<mir::Module>,
    ctx: Box<Ctx>,
    /// function index of every pc
    func_at: Vec<u32>,
    pub entries: u64,
    bc: *const bc::Module,
    /// loop headers left to the interpreter (see [`vm_bound_headers`])
    skip: Vec<bool>,
}

impl JitState {
    pub(crate) fn new(m: &bc::Module, mode: Mode, threshold: u32) -> Option<Box<JitState>> {
        if mode == Mode::Off {
            return None;
        }
        Some(Box::new(JitState {
            tier: Tiering::new(mode, threshold),
            mir: None,
            ctx: Ctx::with_capacity(core::ptr::null_mut(), 1, 1 << 18),
            func_at: {
                let mut v = vec![0u32; m.code.len()];
                for (i, f) in m.funcs.iter().enumerate() {
                    v[f.entry as usize..f.end as usize].fill(i as u32);
                }
                v
            },
            entries: 0,
            bc: m,
            skip: if mode == Mode::Eager { vec![false; m.code.len()] } else { vm_bound_headers(m) },
        }))
    }

    /// State running code compiled ahead of time (see [`build_image`]).
    pub(crate) fn from_image(m: &bc::Module, image: &[u8], hosts: &Hosts) -> Result<Box<JitState>, String> {
        let (fp, image) = image.split_at_checked(8).ok_or("truncated native image")?;
        if u64::from_le_bytes(fp.try_into().expect("8 bytes")) != fingerprint(m, hosts) {
            return Err("native image was built for another program or runner".into());
        }
        let mir = lower_full(m, hosts);
        let img = qb_link::Image::from_bytes(image).map_err(|e| e.to_string())?;
        let syms: std::collections::HashMap<String, usize> = qb_jit::symbols(&mir).into_iter().collect();
        let loaded = qb_link::load(&img, &|n| syms.get(n).copied()).map_err(|e| e.to_string())?;
        let mut fns = Vec::with_capacity(mir.funcs.len());
        for i in 0..mir.funcs.len() {
            let p = loaded.symbol(&qb_jit::func_symbol(i)).ok_or("native image does not match this program")?;
            // SAFETY: compiled from this program's MIR with the native entry signature.
            fns.push(unsafe { core::mem::transmute::<*const u8, qb_jit::NativeFn>(p) });
        }
        // SAFETY: as above; `loaded` owns the code memory.
        let code = unsafe { qb_jit::Compiled::from_loaded(fns, Box::new(loaded)) };
        let mut st = JitState::new(m, Mode::Hot, 1).expect("hot mode");
        st.tier.preload(code);
        st.mir = Some(mir);
        Ok(st)
    }

    pub(crate) fn from_env(m: &bc::Module) -> Option<Box<JitState>> {
        let t = Tiering::from_env();
        JitState::new(m, t.mode, t.threshold)
    }

    pub(crate) fn failure(&self) -> Option<String> {
        self.tier.failure().map(str::to_string)
    }

    /// Cheap pre-check at every back-edge: is this loop header ever entered natively?
    #[inline(always)]
    pub(crate) fn wants(&self, pc: u32) -> bool {
        !self.skip[pc as usize]
    }

    fn func_of(&self, pc: u32) -> u32 {
        self.func_at[pc as usize]
    }

    /// Enters native code at `pc` once it is compiled.
    ///
    /// # Safety
    /// `np` is the VM's current window of the function containing `pc`,
    /// `globals` its module frame, `user` the VM that `cb` expects.
    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn enter(
        &mut self,
        pc: u32,
        np: *mut u64,
        globals: *mut u64,
        ev: *const AtomicU32,
        mask: u32,
        user: *mut core::ffi::c_void,
        hosts: &Hosts,
        tables: [*mut u64; 2],
    ) -> Option<Result<(u32, bool), (i32, u32)>> {
        if self.mir.is_none() {
            // SAFETY: the bytecode module outlives the VM that owns this state.
            self.mir = Some(lower_full(unsafe { &*self.bc }, hosts));
        }
        if self.skip[pc as usize] {
            return None;
        }
        let func = self.func_of(pc);
        let mir = self.mir.as_ref().expect("lowered");
        let code = self.tier.tick(mir, func, pc)?;
        let ctx = &mut *self.ctx;
        ctx.reset_exit();
        ctx.globals = globals;
        ctx.ev_ptr = ev;
        ctx.ev_mask = mask;
        ctx.user = user;
        ctx.tables = tables;
        ctx.call_budget = qb_jit::ctx::DEFAULT_CALL_BUDGET;
        self.entries += 1;
        // SAFETY: caller contract; the frame layout is the VM window.
        let st = unsafe { code.run(ctx, func, np, pc) };
        Some(match st {
            status::DEOPT => Ok((ctx.exit_pc, false)),
            status::EVENT => Ok((ctx.exit_pc, true)),
            status::ERROR => Err((ctx.err, ctx.exit_pc)),
            status::END => Ok((u32::MAX, false)),
            other => unreachable!("native status {other} from VM-lowered code"),
        })
    }

    /// Native call frames left behind by the last exit, outermost first:
    /// (VM function, its registers, pc of the CALL in the parent).
    pub(crate) fn take_frames(&mut self, vm_funcs: usize) -> Vec<(u32, Vec<u64>, u32)> {
        let mir = self.mir.as_ref().expect("lowered");
        let ctx = &mut *self.ctx;
        let mut v: Vec<(u32, Vec<u64>, u32)> = ctx
            .chain
            .drain(..)
            .map(|c| {
                let name = &mir.funcs[c.func as usize].name;
                let vf = mir.funcs[..vm_funcs].iter().position(|f| format!("{}.call", f.name) == *name).expect("call version") as u32;
                let n = mir.funcs[c.func as usize].frame_len();
                // SAFETY: the frame is in the context arena, untouched since the exit.
                let regs = unsafe { core::slice::from_raw_parts(c.frame, n) }.to_vec();
                (vf, regs, c.ret_pc)
            })
            .collect();
        v.reverse();
        v
    }
}

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

/// Control the VM owns: native code stops here and the VM continues at this pc.
fn exits(ins: &Ins) -> bool {
    use Ins::*;
    matches!(
        ins,
        Call { .. }
            | Ret
            | End
            | Gosub { .. }
            | Return
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
            W::W64 => return,
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
            _ => {
                let regs = frame_regs(self.m, ins, self.m.funcs[self.fi].n_num(), self.is_main());
                self.insts.push(Inst::VmOp { op: pc, reads: regs.clone(), writes: regs });
            }
        }
    }
}

/// Lowers every function of `m`. `vm_op` = address of the VM step callback.
pub(crate) fn lower(m: &bc::Module, vm_op: usize) -> mir::Module {
    let gcls = classes(m, &m.funcs[0]);
    let mut funcs = Vec::new();
    for (fi, f) in m.funcs.iter().enumerate() {
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
        let bidx = |pc: u32| leaders.binary_search(&pc).ok().map(|i| i as u32);
        let mut lw = Lower { m, fi, slots: cls.iter().map(|c| ty_of(*c)).collect(), cls, gcls: &gcls, insts: vec![] };
        let mut blocks = Vec::new();
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
                let t: Option<Term> = if exits(ins) {
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
        funcs.push(mir::Func {
            name: f.name.clone(),
            slots: lw.slots,
            params: vec![],
            ret: None,
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

/// JIT state owned by a VM.
pub struct JitState {
    tier: Tiering,
    /// Lowered on first use (needs the VM callback address).
    mir: Option<mir::Module>,
    ctx: Box<Ctx>,
    /// (entry, end) of every function, by function index.
    ranges: Vec<(u32, u32)>,
    pub entries: u64,
    bc: *const bc::Module,
}

impl JitState {
    pub(crate) fn new(m: &bc::Module, mode: Mode, threshold: u32) -> Option<Box<JitState>> {
        if mode == Mode::Off {
            return None;
        }
        Some(Box::new(JitState {
            tier: Tiering::new(mode, threshold),
            mir: None,
            ctx: Ctx::with_capacity(core::ptr::null_mut(), 1, 1),
            ranges: m.funcs.iter().map(|f| (f.entry, f.end)).collect(),
            entries: 0,
            bc: m,
        }))
    }

    pub(crate) fn from_env(m: &bc::Module) -> Option<Box<JitState>> {
        let t = Tiering::from_env();
        JitState::new(m, t.mode, t.threshold)
    }

    pub(crate) fn failure(&self) -> Option<String> {
        self.tier.failure().map(str::to_string)
    }

    fn func_of(&self, pc: u32) -> u32 {
        self.ranges.iter().position(|&(lo, hi)| (lo..hi).contains(&pc)).unwrap_or(0) as u32
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
        cb: VmOpFn,
    ) -> Option<Result<(u32, bool), (i32, u32)>> {
        if self.mir.is_none() {
            // SAFETY: the bytecode module outlives the VM that owns this state.
            self.mir = Some(lower(unsafe { &*self.bc }, cb as *const () as usize));
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
        self.entries += 1;
        // SAFETY: caller contract; the frame layout is the VM window.
        let st = unsafe { code.run(ctx, func, np, pc) };
        Some(match st {
            status::DEOPT => Ok((ctx.exit_pc, false)),
            status::EVENT => Ok((ctx.exit_pc, true)),
            status::ERROR => Err((ctx.err, ctx.exit_pc)),
            other => unreachable!("native status {other} from VM-lowered code"),
        })
    }
}

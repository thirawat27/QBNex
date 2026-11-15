//! Which BYREF parameters must alias their argument (register mode).
//!
//! In register mode a BYREF scalar or array element is copied into the
//! callee's register at the call and copied back at the return: the callee
//! then works on registers, which is what makes calls cheap and lets numeric
//! callees run as native code. A copy can only be told apart from the
//! argument itself when the same storage is reached another way while the
//! call runs:
//!
//! 1. the callee, or anything it calls, uses the argument's module-level
//!    variable (DIM SHARED, a variable of the main module, a STATIC) by name,
//!    and one of the two sides writes it;
//! 2. two arguments of the same call share storage (`CALL p(x, x)`, `a(i)`
//!    and `a(j)`, `a(i)` and `a()`), and the callee writes one of them.
//!
//! Such a parameter becomes a reference ([`crate::bytecode::Slot::R`]): its
//! loads and stores go to the argument's storage. Whole arrays are always
//! shared with the callee (`Vm::call`), and ON ERROR / event handlers and
//! `SUB _GL`, which run while procedures are suspended, see the copies
//! written back first (`Vm::sync_out`); neither needs this analysis.
//!
//! The analysis is conservative (a variable passed on BYREF counts as
//! written, a parameter's possible arguments are those of every call site),
//! so a parameter may become a reference that never needed to be one; that
//! costs speed, never correctness.

use std::collections::{HashMap, HashSet};

use qb_ir::{self as ir, Inst, VarKind};

/// Fixed-size set of module-level storage ids.
#[derive(Clone, PartialEq)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(n: usize) -> Bits {
        Bits(vec![0; n.div_ceil(64)])
    }
    fn set(&mut self, i: u32) {
        self.0[i as usize / 64] |= 1 << (i % 64);
    }
    fn get(&self, i: u32) -> bool {
        self.0[i as usize / 64] & (1 << (i % 64)) != 0
    }
    /// `self |= o`; true when something was added.
    fn or(&mut self, o: &Bits) -> bool {
        let mut changed = false;
        for (a, b) in self.0.iter_mut().zip(&o.0) {
            let n = *a | *b;
            changed |= n != *a;
            *a = n;
        }
        changed
    }
}

/// What a variable name means inside one function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Res {
    /// a BYREF scalar or array parameter (index)
    Param(usize),
    /// storage: module-level ids are `< n_module`, locals come after
    Key(u32),
}

struct Site {
    callee: usize,
    /// per argument: the variable whose address is passed, if any
    args: Vec<Option<Res>>,
}

struct FnInfo {
    /// module-level storage used by name (read or written)
    touch: Bits,
    /// module-level storage possibly written by name
    written: Bits,
    /// per parameter: written (stored, or passed on BYREF / by address)
    pwritten: Vec<bool>,
    sites: Vec<Site>,
}

/// `(function, parameter)` pairs (lowered function order: `order[k]` is the
/// index in `src.funcs`) whose BYREF parameter must refer to its argument.
pub(crate) fn reference_params(src: &ir::Module, order: &[usize]) -> HashSet<(usize, usize)> {
    let fidx: HashMap<&str, usize> = order.iter().enumerate().map(|(k, &i)| (src.funcs[i].name.as_str(), k)).collect();
    // module-level storage: globals, then every function's statics
    let mut module: HashMap<(usize, &str), u32> = HashMap::new();
    for g in &src.globals {
        let id = module.len() as u32;
        module.entry((usize::MAX, g.name.as_str())).or_insert(id);
    }
    for (k, &fi) in order.iter().enumerate() {
        for l in src.funcs[fi].locals.iter().filter(|l| l.kind == VarKind::Static) {
            let id = module.len() as u32;
            module.entry((k, l.name.as_str())).or_insert(id);
        }
    }
    let n_module = module.len() as u32;
    let mut next_local = n_module;
    let mut infos = Vec::with_capacity(order.len());
    for (k, &fi) in order.iter().enumerate() {
        let f = &src.funcs[fi];
        let mut own: HashMap<&str, Res> = HashMap::new();
        for (j, p) in f.params.iter().enumerate() {
            let byref = matches!(p.kind, VarKind::Param { byref: true }) || p.array;
            let r = if byref {
                Res::Param(j)
            } else {
                next_local += 1;
                Res::Key(next_local - 1)
            };
            own.insert(p.name.as_str(), r);
        }
        for l in &f.locals {
            let r = if l.kind == VarKind::Static {
                Res::Key(module[&(k, l.name.as_str())])
            } else {
                next_local += 1;
                Res::Key(next_local - 1)
            };
            own.insert(l.name.as_str(), r);
        }
        let resolve = |n: &str| -> Option<Res> { own.get(n).copied().or_else(|| module.get(&(usize::MAX, n)).map(|&g| Res::Key(g))) };
        let mut info = FnInfo { touch: Bits::new(n_module as usize), written: Bits::new(n_module as usize), pwritten: vec![false; f.params.len()], sites: Vec::new() };
        // register -> variable whose address it holds (addr / aref / aaddr)
        let mut addr_of: HashMap<u32, Res> = HashMap::new();
        for i in &f.body {
            let (var, write) = match i {
                Inst::Load { var, .. } | Inst::LBound { var, .. } | Inst::UBound { var, .. } => (Some(var), false),
                Inst::Store { var, .. } | Inst::Dim { var, .. } | Inst::Erase { var } => (Some(var), true),
                Inst::Addr { dst, var } | Inst::ARef { dst, var } | Inst::AAddr { dst, var, .. } => {
                    if let Some(r) = resolve(var) {
                        addr_of.insert(*dst, r);
                    }
                    (Some(var), true)
                }
                _ => (None, false),
            };
            if let Some(r) = var.and_then(|v| resolve(v)) {
                match r {
                    Res::Key(g) if g < n_module => {
                        info.touch.set(g);
                        if write {
                            info.written.set(g);
                        }
                    }
                    Res::Param(j) if write => info.pwritten[j] = true,
                    _ => {}
                }
            }
            if let Inst::Call { func, args, .. } = i
                && let Some(&callee) = fidx.get(func.as_str())
            {
                info.sites.push(Site { callee, args: args.iter().map(|a| addr_of.get(a).copied()).collect() });
            }
            // a register redefined by anything else no longer holds that address
            if !matches!(i, Inst::Addr { .. } | Inst::ARef { .. } | Inst::AAddr { .. })
                && let Some(d) = def_reg(i)
            {
                addr_of.remove(&d);
            }
        }
        infos.push(info);
    }
    // what each function uses through its callees too
    loop {
        let mut changed = false;
        for k in 0..infos.len() {
            let callees: Vec<usize> = infos[k].sites.iter().map(|s| s.callee).collect();
            for g in callees {
                if g == k {
                    continue;
                }
                let (t, w) = (infos[g].touch.clone(), infos[g].written.clone());
                changed |= infos[k].touch.or(&t);
                changed |= infos[k].written.or(&w);
            }
        }
        if !changed {
            break;
        }
    }
    // storage each BYREF parameter may stand for, over all call sites
    let mut alias: Vec<Vec<HashSet<u32>>> = order.iter().map(|&fi| vec![HashSet::new(); src.funcs[fi].params.len()]).collect();
    loop {
        let mut changed = false;
        for (c, info) in infos.iter().enumerate() {
            for s in &info.sites {
                for (j, a) in s.args.iter().enumerate() {
                    let Some(r) = a else { continue };
                    let keys = keys_of(&alias, c, *r);
                    if j < alias[s.callee].len() {
                        for key in keys {
                            changed |= alias[s.callee][j].insert(key);
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut out = HashSet::new();
    for (f, info) in infos.iter().enumerate() {
        let params = &src.funcs[order[f]].params;
        for (j, p) in params.iter().enumerate() {
            if p.array || !matches!(p.kind, VarKind::Param { byref: true }) {
                continue;
            }
            // 1. the callee reaches the argument by its module-level name
            let named = alias[f][j].iter().any(|&g| g < n_module && info.touch.get(g) && (info.pwritten[j] || info.written.get(g)));
            if named {
                out.insert((f, j));
            }
        }
    }
    // 2. arguments of one call that share storage
    for (c, info) in infos.iter().enumerate() {
        for s in &info.sites {
            let keys: Vec<HashSet<u32>> = s.args.iter().map(|a| a.map(|r| keys_of(&alias, c, r)).unwrap_or_default()).collect();
            let cp = &src.funcs[order[s.callee]].params;
            let written = |j: usize| infos[s.callee].pwritten.get(j).copied().unwrap_or(false);
            for i in 0..keys.len() {
                for j in i + 1..keys.len() {
                    if keys[i].is_disjoint(&keys[j]) || !(written(i) || written(j)) {
                        continue;
                    }
                    for x in [i, j] {
                        if cp.get(x).is_some_and(|p| !p.array && matches!(p.kind, VarKind::Param { byref: true })) {
                            out.insert((s.callee, x));
                        }
                    }
                }
            }
        }
    }
    out
}

/// Storage a variable of function `c` stands for.
fn keys_of(alias: &[Vec<HashSet<u32>>], c: usize, r: Res) -> HashSet<u32> {
    match r {
        Res::Key(k) => HashSet::from([k]),
        Res::Param(j) => alias[c].get(j).cloned().unwrap_or_default(),
    }
}

/// Register an instruction defines.
fn def_reg(i: &Inst) -> Option<u32> {
    use Inst::*;
    match i {
        Const { dst, .. }
        | Mov { dst, .. }
        | Cvt { dst, .. }
        | Trunc { dst, .. }
        | Bin { dst, .. }
        | Cmp { dst, .. }
        | Concat { dst, .. }
        | Un { dst, .. }
        | Load { dst, .. }
        | Addr { dst, .. }
        | ARef { dst, .. }
        | AAddr { dst, .. }
        | Ld { dst, .. }
        | LdFs { dst, .. }
        | PAdd { dst, .. }
        | LBound { dst, .. }
        | UBound { dst, .. }
        | Read { dst, .. } => Some(*dst),
        Call { dst, .. } | Rt { dst, .. } => *dst,
        _ => None,
    }
}

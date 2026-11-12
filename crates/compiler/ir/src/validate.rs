//! Static checks every executor may rely on after [`validate`] succeeds.

use crate::print::inst_text;
use crate::*;
use std::collections::{HashMap, HashSet};

fn bad<T>(f: &str, msg: impl Into<String>) -> Result<T, IrError> {
    Err(IrError { line: 0, msg: format!("{f}: {}", msg.into()) })
}

struct Scope<'m> {
    m: &'m Module,
    f: &'m Func,
    vars: HashMap<&'m str, &'m VarDecl>,
    regs: HashMap<Reg, Ty>,
}

impl<'m> Scope<'m> {
    fn var(&self, n: &str, ctx: &str) -> Result<&'m VarDecl, IrError> {
        match self.vars.get(n) {
            Some(d) => Ok(d),
            None => bad(&self.f.name, format!("`{ctx}`: unknown variable {n}")),
        }
    }
    fn reg(&self, r: Reg, want: Option<Ty>, ctx: &str) -> Result<Ty, IrError> {
        let t = match self.regs.get(&r) {
            Some(t) => *t,
            None => return bad(&self.f.name, format!("`{ctx}`: register %{r} is never defined")),
        };
        if let Some(w) = want
            && t != w {
                return bad(
                    &self.f.name,
                    format!("`{ctx}`: %{r} is {} but {} is required", t.name(), w.name()),
                );
            }
        Ok(t)
    }
    fn int(&self, r: Reg, ctx: &str) -> Result<(), IrError> {
        let t = self.reg(r, None, ctx)?;
        if !t.is_int() {
            return bad(&self.f.name, format!("`{ctx}`: %{r} must be an integer, is {}", t.name()));
        }
        Ok(())
    }
}

fn check_store_ty(m: &Module, t: &StoreTy, ctx: &str) -> Result<(), IrError> {
    if let StoreTy::Udt(n) = t
        && m.udt(n).is_none() {
            return bad(ctx, format!("unknown type udt:{n}"));
        }
    Ok(())
}

/// Destination register and its type, if the instruction defines one.
fn def(i: &Inst) -> Option<(Reg, Ty)> {
    Some(match i {
        Inst::Const { ty, dst, .. } | Inst::Mov { ty, dst, .. } => (*dst, *ty),
        Inst::Bin { ty, dst, .. } | Inst::Un { ty, dst, .. } => (*dst, *ty),
        Inst::Cmp { dst, .. } => (*dst, Ty::I32),
        Inst::Cvt { to, dst, .. } | Inst::Trunc { to, dst, .. } => (*dst, *to),
        Inst::Concat { dst, .. } | Inst::LdFs { dst, .. } => (*dst, Ty::Str),
        Inst::Load { ty, dst, .. } | Inst::Ld { ty, dst, .. } | Inst::Read { ty, dst } => {
            (*dst, *ty)
        }
        Inst::Addr { dst, .. } | Inst::ARef { dst, .. } | Inst::AAddr { dst, .. } => {
            (*dst, Ty::Ptr)
        }
        Inst::PAdd { dst, .. } => (*dst, Ty::Ptr),
        Inst::LBound { dst, .. } | Inst::UBound { dst, .. } => (*dst, Ty::I32),
        _ => return None,
    })
}

fn func_def_ty(m: &Module, i: &Inst) -> Option<(Reg, Ty)> {
    match i {
        Inst::Call { dst: Some(d), func, .. } => match m.func(func) {
            Some(f) => f.ret.map(|t| (*d, t)),
            // DECLARE LIBRARY: the result type of `meta extfn`
            None => m.meta.iter().filter(|(k, _)| k == "extfn").find_map(|(_, v)| {
                let mut w = v.split_whitespace();
                if w.next() != Some(func.as_str()) {
                    return None;
                }
                let ret = w.nth(1)?;
                ALL_TYS.iter().copied().find(|t| t.name() == ret).map(|t| (*d, t))
            }),
        },
        Inst::Rt { dst: Some(d), name, .. } => {
            builtins::lookup(name).and_then(|b| b.ret).map(|t| (*d, t))
        }
        _ => None,
    }
}

fn labels(f: &Func) -> Result<HashSet<&str>, IrError> {
    let mut s = HashSet::new();
    for i in &f.body {
        if let Inst::Label(l) = i
            && !s.insert(l.as_str()) {
                return bad(&f.name, format!("label {l} defined twice"));
            }
    }
    Ok(s)
}

fn check_func<'m>(m: &'m Module, f: &'m Func, main_labels: &HashSet<&str>) -> Result<(), IrError> {
    let mut vars: HashMap<&str, &VarDecl> = m.globals.iter().map(|g| (g.name.as_str(), g)).collect();
    let mut own = HashSet::new();
    for d in f.params.iter().chain(&f.locals) {
        if !own.insert(d.name.as_str()) {
            return bad(&f.name, format!("variable {} declared twice", d.name));
        }
        check_store_ty(m, &d.ty, &f.name)?;
        if let VarKind::Param { byref: false } = d.kind
            && (d.array || !matches!(d.ty, StoreTy::Scalar(_))) {
                return bad(&f.name, format!("byval parameter {} must be a scalar", d.name));
            }
        vars.insert(&d.name, d);
    }
    let lbl = labels(f)?;
    let mut regs: HashMap<Reg, Ty> = HashMap::new();
    for i in &f.body {
        if let Some((r, t)) = def(i).or_else(|| func_def_ty(m, i))
            && let Some(old) = regs.insert(r, t)
                && old != t {
                    return bad(
                        &f.name,
                        format!("`{}`: %{r} redefined as {} (was {})", inst_text(i), t.name(), old.name()),
                    );
                }
    }
    let s = Scope { m, f, vars, regs };
    for i in &f.body {
        check_inst(&s, i, &lbl, main_labels)?;
    }
    Ok(())
}

fn check_inst(s: &Scope, i: &Inst, lbl: &HashSet<&str>, main_labels: &HashSet<&str>) -> Result<(), IrError> {
    let ctx = inst_text(i);
    let ctx = ctx.as_str();
    let fname = s.f.name.as_str();
    let label = |l: &str, set: &HashSet<&str>| -> Result<(), IrError> {
        if set.contains(l) {
            Ok(())
        } else {
            bad(fname, format!("`{ctx}`: undefined label {l}"))
        }
    };
    let op_err = |what: &str| bad::<()>(fname, format!("`{ctx}`: {what}"));
    let scalar_var = |n: &str| -> Result<&VarDecl, IrError> {
        let d = s.var(n, ctx)?;
        if d.array {
            return bad(fname, format!("`{ctx}`: {n} is an array"));
        }
        Ok(d)
    };
    let array_var = |n: &str| -> Result<&VarDecl, IrError> {
        let d = s.var(n, ctx)?;
        if !d.array {
            return bad(fname, format!("`{ctx}`: {n} is not an array"));
        }
        Ok(d)
    };
    match i {
        Inst::Line(_) | Inst::Label(_) | Inst::End | Inst::Stop | Inst::GRet => {}
        Inst::IncLine { file, .. } => {
            if *file as usize >= s.m.srcfiles.len() {
                op_err("unknown srcfile")?;
            }
        }
        Inst::Const { ty, val, .. } => {
            let ok = match val {
                Lit::Int(_) => ty.is_int() || *ty == Ty::Ptr,
                Lit::Float(_) => ty.is_float(),
                Lit::Str(_) => *ty == Ty::Str,
            };
            if !ok {
                op_err("literal does not match type")?;
            }
        }
        Inst::Mov { ty, src, .. } => {
            s.reg(*src, Some(*ty), ctx)?;
        }
        Inst::Bin { op, ty, a, b, .. } => {
            let ok = match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul => ty.is_num(),
                BinOp::Div | BinOp::Pow => ty.is_float(),
                _ => ty.is_int(),
            };
            if !ok {
                op_err(&format!("{:?} is not defined for {}", op, ty.name()).to_lowercase())?;
            }
            s.reg(*a, Some(*ty), ctx)?;
            s.reg(*b, Some(*ty), ctx)?;
        }
        Inst::Un { op, ty, a, .. } => {
            let ok = if *op == UnOp::Neg { ty.is_num() } else { ty.is_int() };
            if !ok {
                op_err("operator not defined for this type")?;
            }
            s.reg(*a, Some(*ty), ctx)?;
        }
        Inst::Cmp { ty, a, b, .. } => {
            if !(ty.is_num() || *ty == Ty::Str) {
                op_err("compare needs a number or str type")?;
            }
            s.reg(*a, Some(*ty), ctx)?;
            s.reg(*b, Some(*ty), ctx)?;
        }
        Inst::Cvt { to, from, src, .. } | Inst::Trunc { to, from, src, .. } => {
            if !to.is_num() || !from.is_num() {
                op_err("conversion needs number types")?;
            }
            s.reg(*src, Some(*from), ctx)?;
        }
        Inst::Concat { a, b, .. } => {
            s.reg(*a, Some(Ty::Str), ctx)?;
            s.reg(*b, Some(Ty::Str), ctx)?;
        }
        Inst::Load { ty, var, .. } | Inst::Store { ty, var, .. } => {
            let d = scalar_var(var)?;
            if d.ty.reg_ty() != Some(*ty) {
                op_err(&format!("{var} cannot be accessed as {}", ty.name()))?;
            }
            if let Inst::Store { src, .. } = i {
                s.reg(*src, Some(*ty), ctx)?;
            }
        }
        Inst::Addr { var, .. } => {
            scalar_var(var)?;
        }
        Inst::ARef { var, .. } => {
            array_var(var)?;
        }
        Inst::AAddr { var, idx, .. } => {
            array_var(var)?;
            if idx.is_empty() {
                op_err("needs at least one index")?;
            }
            for r in idx {
                s.int(*r, ctx)?;
            }
        }
        Inst::Ld { ptr, .. } => {
            s.reg(*ptr, Some(Ty::Ptr), ctx)?;
        }
        Inst::St { ty, ptr, src, .. } => {
            s.reg(*ptr, Some(Ty::Ptr), ctx)?;
            s.reg(*src, Some(*ty), ctx)?;
        }
        Inst::LdFs { ptr, .. } => {
            s.reg(*ptr, Some(Ty::Ptr), ctx)?;
        }
        Inst::StFs { ptr, src, .. } => {
            s.reg(*ptr, Some(Ty::Ptr), ctx)?;
            s.reg(*src, Some(Ty::Str), ctx)?;
        }
        Inst::PAdd { ptr, off, .. } => {
            s.reg(*ptr, Some(Ty::Ptr), ctx)?;
            s.int(*off, ctx)?;
        }
        Inst::Copy { dst, src, .. } => {
            s.reg(*dst, Some(Ty::Ptr), ctx)?;
            s.reg(*src, Some(Ty::Ptr), ctx)?;
        }
        Inst::Dim { var, bounds, .. } => {
            array_var(var)?;
            if bounds.is_empty() {
                op_err("needs at least one dimension")?;
            }
            for (l, u) in bounds {
                s.int(*l, ctx)?;
                s.int(*u, ctx)?;
            }
        }
        Inst::Erase { var } => {
            array_var(var)?;
        }
        Inst::LBound { var, dim, .. } | Inst::UBound { var, dim, .. } => {
            array_var(var)?;
            s.int(*dim, ctx)?;
        }
        Inst::Jmp(l) | Inst::Gosub(l) | Inst::GRetL(l) => label(l, lbl)?,
        Inst::Jz { c, target } | Inst::Jnz { c, target } => {
            s.int(*c, ctx)?;
            label(target, lbl)?;
        }
        Inst::Call { dst, func, args } => {
            let Some(g) = s.m.func(func) else {
                // DECLARE LIBRARY: `meta extfn <name> <lib|-> <ret> <arg types...>`
                let decl = s.m.meta.iter().filter(|(k, _)| k == "extfn").find_map(|(_, v)| {
                    let mut w = v.split_whitespace();
                    (w.next() == Some(func.as_str())).then(|| w.skip(1).map(str::to_string).collect::<Vec<_>>())
                });
                let Some(decl) = decl else {
                    return bad(fname, format!("`{ctx}`: unknown function {func}"));
                };
                let Some((ret, tys)) = decl.split_first() else {
                    return bad(fname, format!("`{ctx}`: bad extfn {func}"));
                };
                if args.len() != tys.len() {
                    op_err(&format!("{func} takes {} arguments", tys.len()))?;
                }
                for (a, t) in args.iter().zip(tys) {
                    if s.reg(*a, None, ctx)?.name() != t.as_str() {
                        op_err(&format!("{func}: argument %{a} must be {t}"))?;
                    }
                }
                if dst.is_some() != (ret != "void") {
                    op_err(&format!("{func}: result register must match its return type"))?;
                }
                return Ok(());
            };
            if func == "main" {
                op_err("main cannot be called")?;
            }
            if args.len() != g.params.len() {
                op_err(&format!("{func} takes {} arguments", g.params.len()))?;
            }
            for (a, p) in args.iter().zip(&g.params) {
                let byref = matches!(p.kind, VarKind::Param { byref: true }) || p.array;
                let want = if byref { Ty::Ptr } else { p.ty.reg_ty().unwrap_or(Ty::Ptr) };
                s.reg(*a, Some(want), ctx)?;
            }
            if dst.is_some() != g.ret.is_some() {
                op_err(&format!("{func}: result register must match its return type"))?;
            }
        }
        Inst::Ret(r) => match (r, s.f.ret) {
            (None, None) => {}
            (Some(r), Some(t)) => {
                s.reg(*r, Some(t), ctx)?;
            }
            _ => op_err("`ret` must carry a value exactly when the function returns one")?,
        },
        Inst::Rt { dst, name, args } => {
            let Some(b) = builtins::lookup(name) else {
                return bad(fname, format!("`{ctx}`: unknown builtin {name}"));
            };
            if args.len() != b.args.len() {
                op_err(&format!("{name} takes {} arguments", b.args.len()))?;
            }
            for (a, t) in args.iter().zip(b.args) {
                if s.reg(*a, None, ctx)? != *t {
                    op_err(&format!("{name}: argument %{a} must be {}", t.name()))?;
                }
            }
            if dst.is_some() != b.ret.is_some() {
                op_err(&format!("{name}: result register must match its return type"))?;
            }
        }
        Inst::OnError(l) => {
            if let Some(l) = l {
                label(l, main_labels)?;
            }
        }
        Inst::Resume(ResumeKind::Label(l)) => label(l, main_labels)?,
        Inst::Resume(_) => {}
        Inst::Error(r) => s.int(*r, ctx)?,
        Inst::OnTimer { slot, secs, target } => {
            s.int(*slot, ctx)?;
            s.reg(*secs, Some(Ty::F64), ctx)?;
            label(target, main_labels)?;
        }
        Inst::OnKey { key: n, target } | Inst::OnPlay { n, target } => {
            s.int(*n, ctx)?;
            label(target, main_labels)?;
        }
        Inst::Read { ty, .. } => {
            if *ty == Ty::Ptr {
                op_err("cannot READ a ptr")?;
            }
        }
        Inst::Restore(Some(l)) => {
            if !s.m.data_labels.iter().any(|(n, _)| n == l) {
                op_err(&format!("undefined DATA label {l}"))?;
            }
        }
        Inst::Restore(None) => {}
        Inst::System(r) => {
            if let Some(r) = r {
                s.int(*r, ctx)?;
            }
        }
    }
    Ok(())
}

/// Checks names, register types, operand types, labels, call and builtin signatures.
pub fn validate(m: &Module) -> Result<(), IrError> {
    let Some(main) = m.func("main") else {
        return bad("module", "no `main` function");
    };
    if !main.params.is_empty() || main.ret.is_some() {
        return bad("main", "main takes no parameters and returns nothing");
    }
    let mut names = HashSet::new();
    for f in &m.funcs {
        if !names.insert(f.name.as_str()) {
            return bad("module", format!("function {} defined twice", f.name));
        }
    }
    let mut tnames = HashSet::new();
    for t in &m.types {
        if !tnames.insert(t.name.as_str()) {
            return bad("module", format!("type {} defined twice", t.name));
        }
        for fl in &t.fields {
            check_store_ty(m, &fl.ty, &t.name)?;
            let size = match &fl.ty {
                StoreTy::Scalar(ty) => ty.size(),
                StoreTy::FStr(n) => *n,
                StoreTy::Udt(n) => m.udt(n).map_or(0, |u| u.size),
            };
            if u64::from(fl.offset) + u64::from(size) > u64::from(t.size) {
                return bad(&t.name, format!("field {} outside the type", fl.name));
            }
        }
    }
    let mut gnames = HashSet::new();
    for g in &m.globals {
        if !gnames.insert(g.name.as_str()) {
            return bad("module", format!("global {} declared twice", g.name));
        }
        check_store_ty(m, &g.ty, "module")?;
    }
    let main_labels = labels(main)?;
    for f in &m.funcs {
        check_func(m, f, &main_labels)?;
    }
    Ok(())
}

const ALL_TYS: [Ty; 13] = [Ty::I8, Ty::U8, Ty::I16, Ty::U16, Ty::I32, Ty::U32, Ty::I64, Ty::U64, Ty::F32, Ty::F64, Ty::F80, Ty::Str, Ty::Ptr];

//! [`Module`] -> canonical QIR text (parses back to an equal module).

use crate::*;
use std::fmt::Write;

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(1 + b.len() * 2);
    s.push('x');
    for c in b {
        let _ = write!(s, "{c:02X}");
    }
    s
}

fn store_ty(t: &StoreTy) -> String {
    match t {
        StoreTy::Scalar(t) => t.name().to_string(),
        StoreTy::FStr(n) => format!("fstr:{n}"),
        StoreTy::Udt(n) => format!("udt:{n}"),
    }
}

fn decl(kw: &str, d: &VarDecl) -> String {
    let arr = if d.array { " array" } else { "" };
    let tail = match d.kind {
        VarKind::Param { byref: true } => " byref",
        VarKind::Param { byref: false } => " byval",
        _ => "",
    };
    format!("{kw} {}{arr} {}{tail}", d.name, store_ty(&d.ty))
}

fn regs(v: &[Reg]) -> String {
    v.iter().map(|r| format!(" %{r}")).collect()
}

fn opt(r: &Option<Reg>) -> String {
    r.map(|r| format!(" %{r}")).unwrap_or_default()
}

fn bin_name(o: BinOp) -> &'static str {
    match o {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        BinOp::Mul => "mul",
        BinOp::Div => "div",
        BinOp::IDiv => "idiv",
        BinOp::Mod => "mod",
        BinOp::Pow => "pow",
        BinOp::And => "and",
        BinOp::Or => "or",
        BinOp::Xor => "xor",
        BinOp::Shl => "shl",
        BinOp::Shr => "shr",
    }
}

fn cmp_name(o: CmpOp) -> &'static str {
    match o {
        CmpOp::Eq => "eq",
        CmpOp::Ne => "ne",
        CmpOp::Lt => "lt",
        CmpOp::Le => "le",
        CmpOp::Gt => "gt",
        CmpOp::Ge => "ge",
    }
}

/// One instruction in canonical text form (no newline).
pub fn inst_text(i: &Inst) -> String {
    match i {
        Inst::Line(n) => format!("line {n}"),
        Inst::Label(l) => format!("label {l}"),
        Inst::Const { ty, dst, val } => {
            let v = match val {
                Lit::Int(n) => n.to_string(),
                Lit::Float(t) => t.clone(),
                Lit::Str(b) => hex(b),
            };
            format!("const.{} %{dst} {v}", ty.name())
        }
        Inst::Mov { ty, dst, src } => format!("mov.{} %{dst} %{src}", ty.name()),
        Inst::Bin { op, ty, dst, a, b } => format!("{}.{} %{dst} %{a} %{b}", bin_name(*op), ty.name()),
        Inst::Un { op, ty, dst, a } => {
            let n = if *op == UnOp::Neg { "neg" } else { "not" };
            format!("{n}.{} %{dst} %{a}", ty.name())
        }
        Inst::Cmp { op, ty, dst, a, b } => format!("{}.{} %{dst} %{a} %{b}", cmp_name(*op), ty.name()),
        Inst::Cvt { to, from, dst, src } => format!("cvt.{}.{} %{dst} %{src}", to.name(), from.name()),
        Inst::Trunc { to, from, dst, src } => {
            format!("trunc.{}.{} %{dst} %{src}", to.name(), from.name())
        }
        Inst::Concat { dst, a, b } => format!("concat %{dst} %{a} %{b}"),
        Inst::Load { ty, dst, var } => format!("load.{} %{dst} {var}", ty.name()),
        Inst::Store { ty, var, src } => format!("store.{} {var} %{src}", ty.name()),
        Inst::Addr { dst, var } => format!("addr %{dst} {var}"),
        Inst::ARef { dst, var } => format!("aref %{dst} {var}"),
        Inst::AAddr { dst, var, idx } => format!("aaddr %{dst} {var}{}", regs(idx)),
        Inst::Ld { ty, dst, ptr, off } => format!("ld.{} %{dst} %{ptr} {off}", ty.name()),
        Inst::St { ty, ptr, off, src } => format!("st.{} %{ptr} {off} %{src}", ty.name()),
        Inst::LdFs { dst, ptr, off, len } => format!("ldfs %{dst} %{ptr} {off} {len}"),
        Inst::StFs { ptr, off, len, src } => format!("stfs %{ptr} {off} {len} %{src}"),
        Inst::PAdd { dst, ptr, off } => format!("padd %{dst} %{ptr} %{off}"),
        Inst::Copy { dst, src, size } => format!("copy %{dst} %{src} {size}"),
        Inst::Dim { kind, var, bounds } => {
            let k = match kind {
                DimKind::Dim => "dim",
                DimKind::Redim => "redim",
                DimKind::RedimPreserve => "redimp",
            };
            let b: String = bounds.iter().map(|(l, u)| format!(" %{l} %{u}")).collect();
            format!("{k} {var} {}{b}", bounds.len())
        }
        Inst::Erase { var } => format!("erase {var}"),
        Inst::LBound { dst, var, dim } => format!("lbound %{dst} {var} %{dim}"),
        Inst::UBound { dst, var, dim } => format!("ubound %{dst} {var} %{dim}"),
        Inst::Jmp(l) => format!("jmp {l}"),
        Inst::Jz { c, target } => format!("jz %{c} {target}"),
        Inst::Jnz { c, target } => format!("jnz %{c} {target}"),
        Inst::Gosub(l) => format!("gosub {l}"),
        Inst::GRet => "gret".into(),
        Inst::GRetL(l) => format!("gretl {l}"),
        Inst::Call { dst, func, args } => format!("call{} {func}{}", opt(dst), regs(args)),
        Inst::Ret(r) => format!("ret{}", opt(r)),
        Inst::Rt { dst, name, args } => format!("rt{} {name}{}", opt(dst), regs(args)),
        Inst::OnError(None) => "onerror 0".into(),
        Inst::OnError(Some(l)) => format!("onerror {l}"),
        Inst::Resume(ResumeKind::Same) => "resume".into(),
        Inst::Resume(ResumeKind::Next) => "resume next".into(),
        Inst::Resume(ResumeKind::Label(l)) => format!("resume {l}"),
        Inst::Error(r) => format!("error %{r}"),
        Inst::OnTimer { slot, secs, target } => format!("ontimer %{slot} %{secs} {target}"),
        Inst::Read { ty, dst } => format!("read.{} %{dst}", ty.name()),
        Inst::Restore(None) => "restore".into(),
        Inst::Restore(Some(l)) => format!("restore {l}"),
        Inst::End => "end".into(),
        Inst::Stop => "stop".into(),
        Inst::System(r) => format!("system{}", opt(r)),
    }
}

/// Canonical text: header, meta, types, globals, data, functions.
pub fn print(m: &Module) -> String {
    let mut s = format!("QIR {VERSION}\n");
    for (k, v) in &m.meta {
        if v.is_empty() {
            let _ = writeln!(s, "meta {k}");
        } else {
            let _ = writeln!(s, "meta {k} {v}");
        }
    }
    for t in &m.types {
        let _ = writeln!(s, "type {} {}", t.name, t.size);
        for f in &t.fields {
            let _ = writeln!(s, "field {} {} {}", f.name, store_ty(&f.ty), f.offset);
        }
        s.push_str("endtype\n");
    }
    for g in &m.globals {
        let _ = writeln!(s, "{}", decl("global", g));
    }
    for i in 0..=m.data.len() {
        for (l, _) in m.data_labels.iter().filter(|(_, k)| *k == i) {
            let _ = writeln!(s, "datalabel {l}");
        }
        if let Some(d) = m.data.get(i) {
            let _ = writeln!(s, "data {}", hex(d));
        }
    }
    for f in &m.funcs {
        match f.ret {
            Some(t) => {
                let _ = writeln!(s, "func {} -> {}", f.name, t.name());
            }
            None => {
                let _ = writeln!(s, "func {}", f.name);
            }
        }
        for p in &f.params {
            let _ = writeln!(s, "{}", decl("param", p));
        }
        for l in &f.locals {
            let kw = if l.kind == VarKind::Static { "static" } else { "local" };
            let _ = writeln!(s, "{}", decl(kw, l));
        }
        for i in &f.body {
            s.push_str(&inst_text(i));
            s.push('\n');
        }
        s.push_str("endfunc\n");
    }
    s
}

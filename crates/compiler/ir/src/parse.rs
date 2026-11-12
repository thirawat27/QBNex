//! QIR text -> [`Module`].

use crate::*;

struct P<'a> {
    toks: Vec<&'a str>,
    pos: usize,
    line: u32,
}

fn err<T>(line: u32, msg: impl Into<String>) -> Result<T, IrError> {
    Err(IrError { line, msg: msg.into() })
}

impl<'a> P<'a> {
    fn e<T>(&self, msg: impl Into<String>) -> Result<T, IrError> {
        err(self.line, msg)
    }
    fn next(&mut self) -> Result<&'a str, IrError> {
        let t = self.toks.get(self.pos).copied();
        self.pos += 1;
        match t {
            Some(t) => Ok(t),
            None => self.e(format!("missing operand in `{}`", self.toks.join(" "))),
        }
    }
    fn peek(&self) -> Option<&'a str> {
        self.toks.get(self.pos).copied()
    }
    fn done(&self) -> Result<(), IrError> {
        if self.pos < self.toks.len() {
            return self.e(format!("unexpected `{}`", self.toks[self.pos]));
        }
        Ok(())
    }
    fn reg(&mut self) -> Result<Reg, IrError> {
        let t = self.next()?;
        match t.strip_prefix('%').and_then(|n| n.parse::<Reg>().ok()) {
            Some(r) => Ok(r),
            None => self.e(format!("expected register, found `{t}`")),
        }
    }
    fn regs(&mut self) -> Result<Vec<Reg>, IrError> {
        let mut v = Vec::new();
        while self.peek().is_some() {
            v.push(self.reg()?);
        }
        Ok(v)
    }
    fn opt_dst(&mut self) -> Result<Option<Reg>, IrError> {
        if self.peek().is_some_and(|t| t.starts_with('%')) {
            Ok(Some(self.reg()?))
        } else {
            Ok(None)
        }
    }
    fn var(&mut self) -> Result<String, IrError> {
        let t = self.next()?;
        if t.len() < 2 || !t.starts_with('$') {
            return self.e(format!("expected variable, found `{t}`"));
        }
        Ok(t.to_string())
    }
    fn name(&mut self) -> Result<String, IrError> {
        let t = self.next()?;
        if t.starts_with('%') || t.starts_with('$') {
            return self.e(format!("expected name, found `{t}`"));
        }
        Ok(t.to_string())
    }
    fn u32(&mut self) -> Result<u32, IrError> {
        let t = self.next()?;
        match t.parse::<u32>() {
            Ok(n) => Ok(n),
            Err(_) => self.e(format!("expected number, found `{t}`")),
        }
    }
    fn ty(&mut self) -> Result<Ty, IrError> {
        let t = self.next()?;
        match Ty::from_name(t) {
            Some(t) => Ok(t),
            None => self.e(format!("unknown type `{t}`")),
        }
    }
    fn store_ty(&mut self) -> Result<StoreTy, IrError> {
        let t = self.next()?;
        store_ty(t).ok_or(()).or_else(|_| self.e(format!("unknown type `{t}`")))
    }
}

fn store_ty(t: &str) -> Option<StoreTy> {
    if let Some(n) = t.strip_prefix("fstr:") {
        return n.parse().ok().map(StoreTy::FStr);
    }
    if let Some(n) = t.strip_prefix("udt:") {
        return (!n.is_empty()).then(|| StoreTy::Udt(n.to_string()));
    }
    Ty::from_name(t).map(StoreTy::Scalar)
}

pub(crate) fn hex_bytes(h: &str) -> Option<Vec<u8>> {
    if !h.len().is_multiple_of(2) {
        return None;
    }
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(h.get(i..i + 2)?, 16).ok())
        .collect()
}

fn lit(p: &mut P, ty: Ty) -> Result<Lit, IrError> {
    let t = p.next()?;
    if ty == Ty::Str {
        return match t.strip_prefix('x').and_then(hex_bytes) {
            Some(b) => Ok(Lit::Str(b)),
            None => p.e(format!("bad string literal `{t}`")),
        };
    }
    if ty.is_float() {
        let ok = if ty == Ty::F80 && t.starts_with("0x") {
            f80_bits(t).is_some()
        } else {
            float_value(t).is_some()
        };
        if !ok {
            return p.e(format!("bad float literal `{t}`"));
        }
        return Ok(Lit::Float(t.to_string()));
    }
    let v = match t.parse::<i128>() {
        Ok(v) => v,
        Err(_) => return p.e(format!("bad integer literal `{t}`")),
    };
    let (lo, hi): (i128, i128) = match ty {
        Ty::I8 => (i8::MIN.into(), i8::MAX.into()),
        Ty::U8 => (0, u8::MAX.into()),
        Ty::I16 => (i16::MIN.into(), i16::MAX.into()),
        Ty::U16 => (0, u16::MAX.into()),
        Ty::I32 => (i32::MIN.into(), i32::MAX.into()),
        Ty::U32 => (0, u32::MAX.into()),
        Ty::I64 => (i64::MIN.into(), i64::MAX.into()),
        _ => (0, u64::MAX.into()),
    };
    if v < lo || v > hi {
        return p.e(format!("literal {v} out of range for {}", ty.name()));
    }
    Ok(Lit::Int(v))
}

fn decl(p: &mut P, kind: VarKind) -> Result<VarDecl, IrError> {
    let name = p.var()?;
    let mut array = false;
    if p.peek() == Some("array") {
        p.pos += 1;
        array = true;
    }
    let ty = p.store_ty()?;
    let kind = match kind {
        VarKind::Param { .. } => match p.next()? {
            "byref" => VarKind::Param { byref: true },
            "byval" => VarKind::Param { byref: false },
            t => return p.e(format!("expected byref/byval, found `{t}`")),
        },
        k => k,
    };
    p.done()?;
    Ok(VarDecl { name, kind, ty, array })
}

fn bin_op(s: &str) -> Option<BinOp> {
    Some(match s {
        "add" => BinOp::Add,
        "sub" => BinOp::Sub,
        "mul" => BinOp::Mul,
        "div" => BinOp::Div,
        "idiv" => BinOp::IDiv,
        "mod" => BinOp::Mod,
        "pow" => BinOp::Pow,
        "and" => BinOp::And,
        "or" => BinOp::Or,
        "xor" => BinOp::Xor,
        "shl" => BinOp::Shl,
        "shr" => BinOp::Shr,
        _ => return None,
    })
}

fn cmp_op(s: &str) -> Option<CmpOp> {
    Some(match s {
        "eq" => CmpOp::Eq,
        "ne" => CmpOp::Ne,
        "lt" => CmpOp::Lt,
        "le" => CmpOp::Le,
        "gt" => CmpOp::Gt,
        "ge" => CmpOp::Ge,
        _ => return None,
    })
}

fn inst(p: &mut P) -> Result<Inst, IrError> {
    let op = p.next()?;
    let mut parts = op.split('.');
    let m = parts.next().unwrap_or("");
    let tys: Vec<&str> = parts.collect();
    let ty_at = |p: &P, i: usize| -> Result<Ty, IrError> {
        match tys.get(i).and_then(|t| Ty::from_name(t)) {
            Some(t) => Ok(t),
            None => p.e(format!("`{op}` needs a type suffix")),
        }
    };
    let want_tys = |p: &P, n: usize| -> Result<(), IrError> {
        if tys.len() != n {
            return p.e(format!("`{op}`: wrong number of type suffixes"));
        }
        Ok(())
    };
    let i = if let Some(b) = bin_op(m) {
        want_tys(p, 1)?;
        let ty = ty_at(p, 0)?;
        Inst::Bin { op: b, ty, dst: p.reg()?, a: p.reg()?, b: p.reg()? }
    } else if let Some(c) = cmp_op(m) {
        want_tys(p, 1)?;
        let ty = ty_at(p, 0)?;
        Inst::Cmp { op: c, ty, dst: p.reg()?, a: p.reg()?, b: p.reg()? }
    } else {
        let typed = matches!(
            m,
            "const" | "mov" | "neg" | "not" | "load" | "store" | "ld" | "st" | "read"
        );
        let two = matches!(m, "cvt" | "trunc");
        want_tys(p, if two { 2 } else if typed { 1 } else { 0 })?;
        match m {
            "line" => Inst::Line(p.u32()?),
            "incline" => Inst::IncLine { line: p.u32()?, file: p.u32()? },
            "label" => Inst::Label(p.name()?),
            "const" => {
                let ty = ty_at(p, 0)?;
                let dst = p.reg()?;
                Inst::Const { ty, dst, val: lit(p, ty)? }
            }
            "mov" => Inst::Mov { ty: ty_at(p, 0)?, dst: p.reg()?, src: p.reg()? },
            "neg" => Inst::Un { op: UnOp::Neg, ty: ty_at(p, 0)?, dst: p.reg()?, a: p.reg()? },
            "not" => Inst::Un { op: UnOp::Not, ty: ty_at(p, 0)?, dst: p.reg()?, a: p.reg()? },
            "cvt" => Inst::Cvt { to: ty_at(p, 0)?, from: ty_at(p, 1)?, dst: p.reg()?, src: p.reg()? },
            "trunc" => {
                Inst::Trunc { to: ty_at(p, 0)?, from: ty_at(p, 1)?, dst: p.reg()?, src: p.reg()? }
            }
            "concat" => Inst::Concat { dst: p.reg()?, a: p.reg()?, b: p.reg()? },
            "load" => Inst::Load { ty: ty_at(p, 0)?, dst: p.reg()?, var: p.var()? },
            "store" => Inst::Store { ty: ty_at(p, 0)?, var: p.var()?, src: p.reg()? },
            "addr" => Inst::Addr { dst: p.reg()?, var: p.var()? },
            "aref" => Inst::ARef { dst: p.reg()?, var: p.var()? },
            "aaddr" => Inst::AAddr { dst: p.reg()?, var: p.var()?, idx: p.regs()? },
            "ld" => Inst::Ld { ty: ty_at(p, 0)?, dst: p.reg()?, ptr: p.reg()?, off: p.u32()? },
            "st" => Inst::St { ty: ty_at(p, 0)?, ptr: p.reg()?, off: p.u32()?, src: p.reg()? },
            "ldfs" => Inst::LdFs { dst: p.reg()?, ptr: p.reg()?, off: p.u32()?, len: p.u32()? },
            "stfs" => Inst::StFs { ptr: p.reg()?, off: p.u32()?, len: p.u32()?, src: p.reg()? },
            "padd" => Inst::PAdd { dst: p.reg()?, ptr: p.reg()?, off: p.reg()? },
            "copy" => Inst::Copy { dst: p.reg()?, src: p.reg()?, size: p.u32()? },
            "dim" | "dims" | "redim" | "redimp" => {
                let kind = match m {
                    "dim" => DimKind::Dim,
                    "dims" => DimKind::Static,
                    "redim" => DimKind::Redim,
                    _ => DimKind::RedimPreserve,
                };
                let var = p.var()?;
                let n = p.u32()?;
                let mut bounds = Vec::new();
                for _ in 0..n {
                    bounds.push((p.reg()?, p.reg()?));
                }
                Inst::Dim { kind, var, bounds }
            }
            "erase" => Inst::Erase { var: p.var()? },
            "lbound" => Inst::LBound { dst: p.reg()?, var: p.var()?, dim: p.reg()? },
            "ubound" => Inst::UBound { dst: p.reg()?, var: p.var()?, dim: p.reg()? },
            "jmp" => Inst::Jmp(p.name()?),
            "jz" => Inst::Jz { c: p.reg()?, target: p.name()? },
            "jnz" => Inst::Jnz { c: p.reg()?, target: p.name()? },
            "gosub" => Inst::Gosub(p.name()?),
            "gret" => Inst::GRet,
            "gretl" => Inst::GRetL(p.name()?),
            "call" => Inst::Call { dst: p.opt_dst()?, func: p.name()?, args: p.regs()? },
            "ret" => Inst::Ret(p.opt_dst()?),
            "rt" => Inst::Rt { dst: p.opt_dst()?, name: p.name()?, args: p.regs()? },
            "onerror" => match p.name()? {
                n if n == "0" => Inst::OnError(None),
                n => Inst::OnError(Some(n)),
            },
            "ontimer" => Inst::OnTimer { slot: p.reg()?, secs: p.reg()?, target: p.name()? },
            "onkey" => Inst::OnKey { key: p.reg()?, target: p.name()? },
            "onplay" => Inst::OnPlay { n: p.reg()?, target: p.name()? },
            "resume" => match p.peek() {
                None => Inst::Resume(ResumeKind::Same),
                Some("next") => {
                    p.pos += 1;
                    Inst::Resume(ResumeKind::Next)
                }
                Some(_) => Inst::Resume(ResumeKind::Label(p.name()?)),
            },
            "error" => Inst::Error(p.reg()?),
            "read" => Inst::Read { ty: ty_at(p, 0)?, dst: p.reg()? },
            "restore" => Inst::Restore(if p.peek().is_some() { Some(p.name()?) } else { None }),
            "end" => Inst::End,
            "stop" => Inst::Stop,
            "system" => Inst::System(p.opt_dst()?),
            _ => return p.e(format!("unknown instruction `{op}`")),
        }
    };
    p.done()?;
    Ok(i)
}

/// Parses QIR text. Rejects a header version newer than [`VERSION`].
pub fn parse(src: &str) -> Result<Module, IrError> {
    let mut m = Module::default();
    let mut func: Option<Func> = None;
    let mut udt: Option<UdtDef> = None;
    let mut header = false;
    for (n, raw) in src.lines().enumerate() {
        let line = n as u32 + 1;
        let text = raw.trim();
        if text.is_empty() || text.starts_with(';') {
            continue;
        }
        let mut p = P { toks: text.split_whitespace().collect(), pos: 0, line };
        if !header {
            if p.next()? != "QIR" {
                return err(line, "missing `QIR <version>` header");
            }
            let v = p.u32()?;
            if v == 0 || v > VERSION {
                return err(line, format!("unsupported QIR version {v} (this build reads {VERSION})"));
            }
            p.done()?;
            header = true;
            continue;
        }
        let head = p.toks[0];
        if let Some(t) = udt.as_mut() {
            p.pos = 1;
            match head {
                "field" => {
                    let name = p.name()?;
                    let ty = p.store_ty()?;
                    let offset = p.u32()?;
                    p.done()?;
                    t.fields.push(Field { name, ty, offset });
                }
                "endtype" => {
                    p.done()?;
                    m.types.push(udt.take().unwrap());
                }
                _ => return err(line, format!("expected `field` or `endtype`, found `{head}`")),
            }
            continue;
        }
        // Module-level directives are accepted inside functions too.
        p.pos = 1;
        match head {
            "meta" => {
                let k = p.name()?;
                let v = p.toks[p.pos..].join(" ");
                m.meta.push((k, v));
                continue;
            }
            "global" => {
                m.globals.push(decl(&mut p, VarKind::Global)?);
                continue;
            }
            "data" => {
                let t = p.next()?;
                let b = t.strip_prefix('x').and_then(hex_bytes);
                match b {
                    Some(b) => m.data.push(b),
                    None => return err(line, format!("bad DATA item `{t}`")),
                }
                p.done()?;
                continue;
            }
            "datalabel" => {
                let l = p.name()?;
                p.done()?;
                m.data_labels.push((l, m.data.len()));
                continue;
            }
            "srcfile" => {
                let k = p.u32()?;
                let t = p.next()?;
                p.done()?;
                if k as usize != m.srcfiles.len() {
                    return err(line, format!("srcfile {k}: expected index {}", m.srcfiles.len()));
                }
                match t.strip_prefix('x').and_then(hex_bytes) {
                    Some(b) => m.srcfiles.push(b),
                    None => return err(line, format!("bad srcfile name `{t}`")),
                }
                continue;
            }
            _ => {}
        }
        match func.as_mut() {
            None => match head {
                "type" => {
                    let name = p.name()?;
                    let size = p.u32()?;
                    p.done()?;
                    udt = Some(UdtDef { name, size, fields: Vec::new() });
                }
                "func" => {
                    let name = p.name()?;
                    let mut ret = None;
                    if p.peek() == Some("->") {
                        p.pos += 1;
                        ret = Some(p.ty()?);
                    }
                    p.done()?;
                    func = Some(Func { name, ret, params: vec![], locals: vec![], body: vec![] });
                }
                _ => return err(line, format!("unexpected `{head}` outside a function")),
            },
            Some(f) => match head {
                "endfunc" => {
                    p.done()?;
                    m.funcs.push(func.take().unwrap());
                }
                "param" => f.params.push(decl(&mut p, VarKind::Param { byref: false })?),
                "local" => f.locals.push(decl(&mut p, VarKind::Local)?),
                "static" => f.locals.push(decl(&mut p, VarKind::Static)?),
                _ => {
                    p.pos = 0;
                    f.body.push(inst(&mut p)?);
                }
            },
        }
    }
    if !header {
        return err(0, "empty input (missing `QIR <version>` header)");
    }
    if func.is_some() {
        return err(0, "missing `endfunc`");
    }
    if udt.is_some() {
        return err(0, "missing `endtype`");
    }
    Ok(m)
}

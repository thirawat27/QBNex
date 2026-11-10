//! QIR v1 reference interpreter (docs/rust-port/qir-v1.md).
//!
//! Deliberately simple: instructions are lowered once (labels and variables
//! resolved to indices) and then interpreted one by one. The fast typed VM
//! (R11b) and the JIT (R11c) replace it; this one exists so the emitter can be
//! tested end to end and serves as the executable form of the spec.
//!
//! Memory: every variable, array descriptor and array data area is a block;
//! a `ptr` is `(block << 32) | offset`. A `str` slot in memory holds a handle
//! into `strs` (0 = empty string).

use crate::RunError;
use qb_ir::{self as ir, BinOp, CmpOp, DimKind, Inst, Lit, ResumeKind, StoreTy, Ty, UnOp, VarKind};
use qb_float80::{F80, Round};
use std::collections::HashMap;
use std::io::Write;

mod files;
mod netx;
mod memx;
mod rt;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum V {
    I(i64),
    F(f64),
    /// `_FLOAT`: x87 80-bit extended (bit-exact with the C++ backend)
    X(F80),
    S(Vec<u8>),
    P(u64),
}

impl V {
    fn i(&self) -> i64 {
        match self {
            V::I(v) => *v,
            V::F(f) => *f as i64,
            V::X(x) => x.to_i64(Round::Zero),
            V::P(p) => *p as i64,
            V::S(_) => 0,
        }
    }
    pub(crate) fn f(&self) -> f64 {
        match self {
            V::F(f) => *f,
            V::X(x) => x.to_f64(),
            V::I(v) => *v as f64,
            _ => 0.0,
        }
    }
    pub(crate) fn x(&self) -> F80 {
        match self {
            V::X(x) => *x,
            V::F(f) => F80::from_f64(*f),
            V::I(v) => F80::from_i64(*v),
            _ => F80::from_i64(0),
        }
    }
    fn s(&self) -> &[u8] {
        match self {
            V::S(s) => s,
            _ => &[],
        }
    }
    fn p(&self) -> u64 {
        match self {
            V::P(p) => *p,
            V::I(v) => *v as u64,
            _ => 0,
        }
    }
}

/// Normalizes an integer to the range of `t` (two's complement wrap).
pub(crate) fn wrap(t: Ty, v: i64) -> i64 {
    match t {
        Ty::I8 => v as i8 as i64,
        Ty::U8 => v as u8 as i64,
        Ty::I16 => v as i16 as i64,
        Ty::U16 => v as u16 as i64,
        Ty::I32 => v as i32 as i64,
        Ty::U32 => v as u32 as i64,
        _ => v,
    }
}

/// Float result in the precision of `t` (`f80` is carried as f64 here).
pub(crate) fn fround(t: Ty, v: f64) -> f64 {
    if t == Ty::F32 {
        f64::from(v as f32)
    } else {
        v
    }
}

#[derive(Debug, Clone, Copy)]
enum Loc {
    Global(usize),
    Local(usize),
}

#[derive(Debug, Clone)]
struct VarInfo {
    ty: StoreTy,
    array: bool,
    size: u32,
}

#[derive(Debug, Clone)]
enum Op {
    Line(u32),
    Nop,
    Const(usize, V),
    Mov(usize, usize),
    Bin(BinOp, Ty, usize, usize, usize),
    Un(UnOp, Ty, usize, usize),
    Cmp(CmpOp, Ty, usize, usize, usize),
    Cvt(Ty, Ty, usize, usize, bool),
    Concat(usize, usize, usize),
    Load(Ty, usize, Loc),
    Store(Ty, Loc, usize),
    Addr(usize, Loc),
    AAddr(usize, Loc, Vec<usize>),
    Ld(Ty, usize, usize, u32),
    St(Ty, usize, u32, usize),
    LdFs(usize, usize, u32, u32),
    StFs(usize, u32, u32, usize),
    PAdd(usize, usize, usize),
    Copy(usize, usize, u32),
    Dim(DimKind, Loc, Vec<(usize, usize)>),
    Erase(Loc),
    Bound(bool, usize, Loc, usize),
    Jmp(usize),
    Jz(usize, usize),
    Jnz(usize, usize),
    Gosub(usize),
    GRet,
    GRetL(usize),
    Call(Option<usize>, usize, Vec<usize>),
    Ret(Option<usize>),
    Rt(Option<usize>, &'static str, Vec<usize>),
    OnError(Option<usize>),
    Resume(u8, usize),
    Error(usize),
    Read(Ty, usize),
    Restore(usize),
    End,
    System(Option<usize>),
}

enum SlotInit {
    Param { byref: bool, var: usize },
    Local(usize),
}

struct Func {
    ops: Vec<Op>,
    nregs: usize,
    slots: Vec<SlotInit>,
    /// index into `Prog::vars` per slot (for byval/local creation)
    nparams: usize,
}

struct Prog {
    funcs: Vec<Func>,
    vars: Vec<VarInfo>,
    globals: Vec<usize>,
    data: Vec<Vec<u8>>,
    main: usize,
    console_only: bool,
}

fn store_size(udts: &HashMap<String, u32>, t: &StoreTy) -> u32 {
    match t {
        StoreTy::Scalar(t) => t.size(),
        StoreTy::FStr(n) => *n,
        StoreTy::Udt(n) => udts.get(n).copied().unwrap_or(0),
    }
}

fn lit_value(ty: Ty, l: &Lit) -> V {
    match l {
        Lit::Int(n) => {
            if ty == Ty::Ptr {
                V::P(*n as u64)
            } else if ty.is_float() {
                V::F(*n as f64)
            } else {
                V::I(*n as i64)
            }
        }
        Lit::Float(t) => {
            if ty == Ty::F80 {
                return V::X(match ir::f80_bits(t) {
                    Some((se, m)) => {
                        let mut b = [0u8; 10];
                        b[..8].copy_from_slice(&m.to_le_bytes());
                        b[8..].copy_from_slice(&se.to_le_bytes());
                        F80::from_bytes(b)
                    }
                    None => match t.as_str() {
                        "inf" => F80::from_f64(f64::INFINITY),
                        "-inf" => F80::from_f64(f64::NEG_INFINITY),
                        "nan" => F80::from_f64(f64::NAN),
                        _ => qb_float80::dec::parse(t.as_bytes()).0,
                    },
                });
            }
            let v = if ty == Ty::F32 && !t.starts_with("0x") {
                t.parse::<f32>().map(f64::from).unwrap_or_else(|_| ir::float_value(t).unwrap_or(0.0))
            } else {
                ir::float_value(t).unwrap_or(0.0)
            };
            V::F(fround(ty, v))
        }
        Lit::Str(s) => V::S(s.clone()),
    }
}

fn load(m: &ir::Module) -> Result<Prog, RunError> {
    ir::validate(m).map_err(|e| RunError::load(e.to_string()))?;
    let udts: HashMap<String, u32> = m.types.iter().map(|t| (t.name.clone(), t.size)).collect();
    let mut vars = Vec::new();
    let mut gmap: HashMap<&str, usize> = HashMap::new();
    let mut globals = Vec::new();
    let info = |d: &ir::VarDecl| VarInfo { ty: d.ty.clone(), array: d.array, size: store_size(&udts, &d.ty) };
    for g in &m.globals {
        gmap.insert(&g.name, globals.len());
        globals.push(vars.len());
        vars.push(info(g));
    }
    let fidx: HashMap<&str, usize> = m.funcs.iter().enumerate().map(|(i, f)| (f.name.as_str(), i)).collect();
    let main = fidx["main"];
    let main_labels: HashMap<&str, usize> = labels(&m.funcs[main]);
    let dlabels: HashMap<&str, usize> = m.data_labels.iter().map(|(l, i)| (l.as_str(), *i)).collect();
    let mut funcs = Vec::new();
    for f in &m.funcs {
        let mut lmap: HashMap<&str, Loc> = HashMap::new();
        let mut slots = Vec::new();
        for p in &f.params {
            lmap.insert(&p.name, Loc::Local(slots.len()));
            let byref = matches!(p.kind, VarKind::Param { byref: true }) || p.array;
            slots.push(SlotInit::Param { byref, var: vars.len() });
            vars.push(info(p));
        }
        for l in &f.locals {
            if l.kind == VarKind::Static {
                lmap.insert(&l.name, Loc::Global(globals.len()));
                globals.push(vars.len());
            } else {
                lmap.insert(&l.name, Loc::Local(slots.len()));
                slots.push(SlotInit::Local(vars.len()));
            }
            vars.push(info(l));
        }
        let loc = |n: &str| -> Loc { lmap.get(n).copied().unwrap_or_else(|| Loc::Global(gmap[n])) };
        let lbl = labels(f);
        let r = |r: &u32| *r as usize;
        let mut nregs = 0usize;
        let mut ops = Vec::with_capacity(f.body.len());
        for i in &f.body {
            let op = match i {
                Inst::Line(n) => Op::Line(*n),
                Inst::Label(_) => Op::Nop,
                Inst::Const { ty, dst, val } => Op::Const(r(dst), lit_value(*ty, val)),
                Inst::Mov { dst, src, .. } => Op::Mov(r(dst), r(src)),
                Inst::Bin { op, ty, dst, a, b } => Op::Bin(*op, *ty, r(dst), r(a), r(b)),
                Inst::Un { op, ty, dst, a } => Op::Un(*op, *ty, r(dst), r(a)),
                Inst::Cmp { op, ty, dst, a, b } => Op::Cmp(*op, *ty, r(dst), r(a), r(b)),
                Inst::Cvt { to, from, dst, src } => Op::Cvt(*to, *from, r(dst), r(src), true),
                Inst::Trunc { to, from, dst, src } => Op::Cvt(*to, *from, r(dst), r(src), false),
                Inst::Concat { dst, a, b } => Op::Concat(r(dst), r(a), r(b)),
                Inst::Load { ty, dst, var } => Op::Load(*ty, r(dst), loc(var)),
                Inst::Store { ty, var, src } => Op::Store(*ty, loc(var), r(src)),
                Inst::Addr { dst, var } | Inst::ARef { dst, var } => Op::Addr(r(dst), loc(var)),
                Inst::AAddr { dst, var, idx } => Op::AAddr(r(dst), loc(var), idx.iter().map(r).collect()),
                Inst::Ld { ty, dst, ptr, off } => Op::Ld(*ty, r(dst), r(ptr), *off),
                Inst::St { ty, ptr, off, src } => Op::St(*ty, r(ptr), *off, r(src)),
                Inst::LdFs { dst, ptr, off, len } => Op::LdFs(r(dst), r(ptr), *off, *len),
                Inst::StFs { ptr, off, len, src } => Op::StFs(r(ptr), *off, *len, r(src)),
                Inst::PAdd { dst, ptr, off } => Op::PAdd(r(dst), r(ptr), r(off)),
                Inst::Copy { dst, src, size } => Op::Copy(r(dst), r(src), *size),
                Inst::Dim { kind, var, bounds } => {
                    Op::Dim(*kind, loc(var), bounds.iter().map(|(a, b)| (r(a), r(b))).collect())
                }
                Inst::Erase { var } => Op::Erase(loc(var)),
                Inst::LBound { dst, var, dim } => Op::Bound(false, r(dst), loc(var), r(dim)),
                Inst::UBound { dst, var, dim } => Op::Bound(true, r(dst), loc(var), r(dim)),
                Inst::Jmp(l) => Op::Jmp(lbl[l.as_str()]),
                Inst::Jz { c, target } => Op::Jz(r(c), lbl[target.as_str()]),
                Inst::Jnz { c, target } => Op::Jnz(r(c), lbl[target.as_str()]),
                Inst::Gosub(l) => Op::Gosub(lbl[l.as_str()]),
                Inst::GRet => Op::GRet,
                Inst::GRetL(l) => Op::GRetL(lbl[l.as_str()]),
                Inst::Call { dst, func, args } => Op::Call(dst.map(|d| d as usize), fidx[func.as_str()], args.iter().map(r).collect()),
                Inst::Ret(v) => Op::Ret(v.map(|d| d as usize)),
                Inst::Rt { dst, name, args } => {
                    let b = ir::builtins::lookup(name).expect("validated");
                    Op::Rt(dst.map(|d| d as usize), b.name, args.iter().map(r).collect())
                }
                Inst::OnError(l) => Op::OnError(l.as_ref().map(|l| main_labels[l.as_str()])),
                Inst::Resume(ResumeKind::Same) => Op::Resume(0, 0),
                Inst::Resume(ResumeKind::Next) => Op::Resume(1, 0),
                Inst::Resume(ResumeKind::Label(l)) => Op::Resume(2, main_labels[l.as_str()]),
                Inst::Error(c) => Op::Error(r(c)),
                Inst::Read { ty, dst } => Op::Read(*ty, r(dst)),
                Inst::Restore(l) => Op::Restore(l.as_ref().map_or(0, |l| dlabels[l.as_str()])),
                Inst::End | Inst::Stop => Op::End,
                Inst::System(c) => Op::System(c.map(|d| d as usize)),
            };
            for reg in regs_of(i) {
                nregs = nregs.max(reg as usize + 1);
            }
            ops.push(op);
        }
        funcs.push(Func { ops, nregs, nparams: f.params.len(), slots });
    }
    Ok(Prog {
        funcs,
        vars,
        globals,
        data: m.data.clone(),
        main,
        console_only: m.meta("console").is_some_and(|v| v == "only"),
    })
}

fn labels(f: &ir::Func) -> HashMap<&str, usize> {
    f.body
        .iter()
        .enumerate()
        .filter_map(|(i, x)| if let Inst::Label(l) = x { Some((l.as_str(), i)) } else { None })
        .collect()
}

fn regs_of(i: &Inst) -> Vec<u32> {
    let t = qb_ir::print(&ir::Module {
        funcs: vec![ir::Func { name: "x".into(), ret: None, params: vec![], locals: vec![], body: vec![i.clone()] }],
        ..Default::default()
    });
    t.split_whitespace().filter_map(|w| w.strip_prefix('%')?.parse().ok()).collect()
}

// ---------------------------------------------------------------- memory

enum Block {
    Bytes { data: Vec<u8>, strs: bool },
    Arr(Arr),
    Free,
}

struct Arr {
    elem: StoreTy,
    esize: u32,
    dims: Vec<(i64, i64)>,
    data: Option<u32>,
}

/// Byte offset of the DBLOCK segment (0x1000) in conventional memory.
pub(crate) const DBLOCK: usize = 0x1000 * 16;

/// Tag of an array-descriptor reference (`aref`); every other `ptr` is a real address.
const ARR_TAG: u64 = 1 << 63;


struct Frame {
    f: usize,
    pc: usize,
    regs: Vec<V>,
    slots: Vec<u64>,
    owned: Vec<u32>,
    gosub: Vec<usize>,
    line_pc: usize,
    line: u32,
    ret_to: Option<usize>,
}

pub(crate) struct Vm<'a> {
    prog: &'a Prog,
    blocks: Vec<Block>,
    free_blocks: Vec<u32>,
    strs: Vec<Vec<u8>>,
    free_strs: Vec<u64>,
    globals: Vec<u64>,
    frames: Vec<Frame>,
    handler: Option<usize>,
    in_handler: Option<(usize, usize)>,
    pub(crate) err: i32,
    pub(crate) erl: u32,
    /// source line of the last error (`_ERRORLINE`)
    pub(crate) errline: u32,
    /// last numeric line label executed (legacy `last_line`)
    pub(crate) last_line: u32,
    /// conventional memory (VARPTR/SADD/PEEK/POKE)
    pub(crate) cmem: Vec<u8>,
    pub(crate) cmem_next: usize,
    /// set while LSETVAR/GETFIELD store into FIELD variables
    pub(crate) field_store: bool,
    /// string variable address -> DBLOCK offset of its descriptor
    pub(crate) cmem_strs: HashMap<u64, usize>,
    /// RANDOM record length per open file number (FIELD buffers)
    pub(crate) reclen: HashMap<i32, usize>,
    /// FIELD buffer per file number and the variables linked to it (address, offset, size)
    pub(crate) field_buf: HashMap<i32, Vec<u8>>,
    pub(crate) field_vars: Vec<(u64, i32, usize, usize)>,
    /// FIELD statement in progress: (file number, bytes used)
    pub(crate) field_cur: (i32, usize),
    /// TCP/IP handles
    pub(crate) net: netx::Net,
    pub(crate) net_pending_error: i32,
    /// DEF SEG base (bytes); None = DBLOCK
    pub(crate) def_seg: Option<usize>,
    data_pos: usize,
    pub(crate) rt: rt::RtState<'a>,
}

pub(crate) enum Stop {
    Err(i32),
    Exit(i32),
}

pub(crate) type R<T> = Result<T, Stop>;

pub(crate) fn e<T>(code: i32) -> R<T> {
    Err(Stop::Err(code))
}

impl<'a> Vm<'a> {
    fn alloc(&mut self, b: Block) -> u32 {
        if let Some(i) = self.free_blocks.pop() {
            self.blocks[i as usize] = b;
            i
        } else {
            self.blocks.push(b);
            (self.blocks.len() - 1) as u32
        }
    }
    fn free(&mut self, i: u32) {
        let b = std::mem::replace(&mut self.blocks[i as usize], Block::Free);
        match b {
            Block::Bytes { data, strs: true } => {
                for c in data.chunks_exact(8) {
                    self.free_str(u64::from_le_bytes(c.try_into().unwrap()));
                }
            }
            Block::Arr(a) => {
                if let Some(d) = a.data {
                    self.free(d);
                }
            }
            _ => {}
        }
        self.free_blocks.push(i);
    }
    fn free_str(&mut self, h: u64) {
        if h != 0 {
            self.strs[h as usize] = Vec::new();
            self.free_strs.push(h);
        }
    }
    fn new_str(&mut self, s: &[u8]) -> u64 {
        if s.is_empty() {
            return 0;
        }
        if let Some(h) = self.free_strs.pop() {
            self.strs[h as usize] = s.to_vec();
            h
        } else {
            self.strs.push(s.to_vec());
            (self.strs.len() - 1) as u64
        }
    }
    fn zero_bytes(ty: &StoreTy, size: u32) -> Vec<u8> {
        match ty {
            StoreTy::FStr(_) => vec![b' '; size as usize],
            _ => vec![0; size as usize],
        }
    }
    fn new_var(&mut self, vi: usize) -> u64 {
        self.new_var_b(vi).0
    }
    /// Storage for variable `vi`: (address, block id).
    fn new_var_b(&mut self, vi: usize) -> (u64, u32) {
        let v = &self.prog.vars[vi];
        if v.array {
            let id = self.alloc(Block::Arr(Arr { elem: v.ty.clone(), esize: v.size, dims: vec![], data: None }));
            return (ARR_TAG | u64::from(id), id);
        }
        let data = Self::zero_bytes(&v.ty, v.size.max(1));
        let id = self.alloc(Block::Bytes { data, strs: v.ty == StoreTy::Scalar(Ty::Str) });
        (self.block_addr(id), id)
    }
    fn block_addr(&self, id: u32) -> u64 {
        match &self.blocks[id as usize] {
            Block::Bytes { data, .. } => data.as_ptr() as u64,
            _ => 0,
        }
    }

    /// `len` bytes of memory at the real address `p` (like the C++ backend,
    /// addresses are not checked beyond null and array references).
    pub(crate) fn bytes(&mut self, p: u64, len: usize) -> R<&mut [u8]> {
        if p == 0 || p & ARR_TAG != 0 {
            return e(5);
        }
        // SAFETY: QIR pointers come from `addr`/`aaddr`/`padd` on live storage
        // (or _MEM blocks of qb-runtime); validated programs keep them in range.
        Ok(unsafe { std::slice::from_raw_parts_mut(p as *mut u8, len) })
    }
    fn ld(&mut self, t: Ty, p: u64) -> R<V> {
        let n = t.size().min(8) as usize;
        let b = self.bytes(p, n)?;
        let mut w = [0u8; 8];
        w[..n].copy_from_slice(b);
        let u = u64::from_le_bytes(w);
        Ok(match t {
            Ty::I8 => V::I(u as i8 as i64),
            Ty::U8 => V::I(u as u8 as i64),
            Ty::I16 => V::I(u as i16 as i64),
            Ty::U16 => V::I(u as u16 as i64),
            Ty::I32 => V::I(u as i32 as i64),
            Ty::U32 => V::I(u as u32 as i64),
            Ty::I64 | Ty::U64 => V::I(u as i64),
            Ty::F32 => V::F(f64::from(f32::from_bits(u as u32))),
            Ty::F64 => V::F(f64::from_bits(u)),
            Ty::F80 => {
                let b = self.bytes(p, 10)?;
                V::X(F80::from_bytes(b.try_into().unwrap()))
            }
            Ty::Ptr => V::P(u),
            Ty::Str => V::S(self.strs[u as usize].clone()),
        })
    }
    fn st(&mut self, t: Ty, p: u64, v: &V) -> R<()> {
        if t == Ty::F80 {
            let b = v.x().to_bytes();
            self.bytes(p, 10)?.copy_from_slice(&b);
            return Ok(());
        }
        let u: u64 = match t {
            Ty::F32 => u64::from((v.f() as f32).to_bits()),
            Ty::F64 => v.f().to_bits(),
            Ty::Ptr => v.p(),
            Ty::Str => {
                if !self.field_vars.is_empty() && !self.field_store {
                    // a plain assignment breaks the FIELD link
                    self.field_vars.retain(|f| f.0 != p);
                }
                if let Some(&d) = self.cmem_strs.get(&p) {
                    let len = (v.s().len() as u16).to_le_bytes();
                    self.cmem[DBLOCK + d..DBLOCK + d + 2].copy_from_slice(&len);
                }
                let old = match self.ld(Ty::U64, p)? {
                    V::I(h) => h as u64,
                    _ => 0,
                };
                self.free_str(old);
                self.new_str(v.s())
            }
            _ => v.i() as u64,
        };
        let n = t.size().min(8) as usize;
        self.bytes(p, n)?.copy_from_slice(&u.to_le_bytes()[..n]);
        Ok(())
    }
    fn ldfs(&mut self, p: u64, len: u32) -> R<V> {
        Ok(V::S(self.bytes(p, len as usize)?.to_vec()))
    }
    fn stfs(&mut self, p: u64, len: u32, s: &[u8]) -> R<()> {
        let b = self.bytes(p, len as usize)?;
        for (i, x) in b.iter_mut().enumerate() {
            *x = s.get(i).copied().unwrap_or(b' ');
        }
        Ok(())
    }

    fn frame(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }
    fn reg(&self, r: usize) -> &V {
        &self.frames.last().unwrap().regs[r]
    }
    fn set(&mut self, r: usize, v: V) {
        self.frame().regs[r] = v;
    }
    fn var_ptr(&self, l: Loc) -> u64 {
        match l {
            Loc::Global(i) => self.globals[i],
            Loc::Local(i) => self.frames.last().unwrap().slots[i],
        }
    }
    fn var_info(&self, l: Loc) -> &'a VarInfo {
        let p = self.prog;
        match l {
            Loc::Global(i) => &p.vars[p.globals[i]],
            Loc::Local(i) => {
                let f = &p.funcs[self.frames.last().unwrap().f];
                match f.slots[i] {
                    SlotInit::Param { var, .. } | SlotInit::Local(var) => &p.vars[var],
                }
            }
        }
    }
    fn load_var(&mut self, t: Ty, l: Loc) -> R<V> {
        let p = self.var_ptr(l);
        match &self.var_info(l).ty {
            StoreTy::FStr(n) => self.ldfs(p, *n),
            _ => self.ld(t, p),
        }
    }
    fn store_var(&mut self, t: Ty, l: Loc, v: &V) -> R<()> {
        let p = self.var_ptr(l);
        match &self.var_info(l).ty {
            StoreTy::FStr(n) => self.stfs(p, *n, v.s()),
            _ => self.st(t, p, v),
        }
    }
    fn arr(&mut self, l: Loc) -> R<&mut Arr> {
        let p = self.var_ptr(l);
        if p & ARR_TAG == 0 {
            return e(9);
        }
        match self.blocks.get_mut((p & !ARR_TAG) as usize) {
            Some(Block::Arr(a)) => Ok(a),
            _ => e(9),
        }
    }

    fn dim(&mut self, kind: DimKind, l: Loc, bounds: Vec<(i64, i64)>) -> R<()> {
        let mut n: u64 = 1;
        for (lo, hi) in &bounds {
            if hi < lo {
                return e(9);
            }
            n = n.saturating_mul((hi - lo + 1) as u64);
        }
        let (esize, elem, old, olddims) = {
            let a = self.arr(l)?;
            if kind == DimKind::Dim && a.data.is_some() {
                return e(10);
            }
            (a.esize, a.elem.clone(), a.data.take(), std::mem::take(&mut a.dims))
        };
        let total = n.saturating_mul(u64::from(esize));
        if total > (1 << 31) {
            if let Ok(a) = self.arr(l) {
                a.data = old;
                a.dims = olddims;
            }
            return e(7);
        }
        let strs = elem == StoreTy::Scalar(Ty::Str);
        let mut data = Self::zero_bytes(&elem, total as u32);
        if kind == DimKind::RedimPreserve {
            if let Some(ob) = old {
                if let Block::Bytes { data: od, .. } = std::mem::replace(&mut self.blocks[ob as usize], Block::Bytes { data: vec![], strs: false }) {
                    let k = od.len().min(data.len());
                    data[..k].copy_from_slice(&od[..k]);
                    if strs {
                        for c in od[k..].chunks_exact(8) {
                            self.free_str(u64::from_le_bytes(c.try_into().unwrap()));
                        }
                    }
                }
                self.free(ob);
            }
        } else if let Some(ob) = old {
            self.free(ob);
        }
        let d = self.alloc(Block::Bytes { data, strs });
        let a = self.arr(l)?;
        a.data = Some(d);
        a.dims = bounds;
        Ok(())
    }

    /// (address of the first element, total bytes) of the array whose descriptor `p` references.
    pub(crate) fn array_extent(&mut self, p: u64) -> R<(u64, u64)> {
        if p & ARR_TAG == 0 {
            return e(9);
        }
        let (data, n, esize) = match self.blocks.get((p & !ARR_TAG) as usize) {
            Some(Block::Arr(a)) => {
                let n: i64 = a.dims.iter().map(|(lo, hi)| hi - lo + 1).product();
                (a.data, n, a.esize)
            }
            _ => return e(9),
        };
        match data {
            Some(d) => Ok((self.block_addr(d), n as u64 * u64::from(esize))),
            None => Ok((0, 0)),
        }
    }
    fn aaddr(&mut self, l: Loc, idx: &[i64]) -> R<u64> {
        let a = self.arr(l)?;
        let Some(d) = a.data else { return e(9) };
        if idx.len() != a.dims.len() {
            return e(9);
        }
        let mut off: i64 = 0;
        let mut mul: i64 = 1;
        for (i, (lo, hi)) in idx.iter().zip(&a.dims) {
            if i < lo || i > hi {
                return e(9);
            }
            off += (i - lo) * mul;
            mul *= hi - lo + 1;
        }
        let esize = i64::from(a.esize);
        Ok(self.block_addr(d).wrapping_add((off * esize) as u64))
    }

    fn enter(&mut self, fi: usize, args: Vec<V>, ret_to: Option<usize>) -> R<()> {
        let prog = self.prog;
        let f = &prog.funcs[fi];
        if self.frames.len() > 20_000 {
            return e(7);
        }
        let mut slots = Vec::with_capacity(f.slots.len());
        let mut owned = Vec::new();
        for (k, s) in f.slots.iter().enumerate() {
            match s {
                SlotInit::Param { byref: true, .. } => slots.push(args[k].p()),
                SlotInit::Param { byref: false, var } | SlotInit::Local(var) => {
                    let (p, id) = self.new_var_b(*var);
                    owned.push(id);
                    if k < f.nparams {
                        let t = prog.vars[*var].ty.reg_ty().unwrap_or(Ty::Ptr);
                        self.st(t, p, &args[k])?;
                    }
                    slots.push(p);
                }
            }
        }
        self.frames.push(Frame {
            f: fi,
            pc: 0,
            regs: vec![V::I(0); f.nregs],
            slots,
            owned,
            gosub: vec![],
            line_pc: 0,
            line: 0,
            ret_to,
        });
        Ok(())
    }

    fn pop_frame(&mut self) -> Frame {
        let fr = self.frames.pop().unwrap();
        for b in &fr.owned {
            self.free(*b);
        }
        fr
    }

    /// Runs to completion; returns the exit code.
    fn run(&mut self) -> Result<i32, RunError> {
        self.enter(self.prog.main, vec![], None).map_err(|_| RunError::load("cannot start"))?;
        loop {
            match self.step() {
                Ok(true) => {}
                Ok(false) => return Ok(0),
                Err(Stop::Exit(c)) => return Ok(c),
                Err(Stop::Err(code)) => {
                    if let Err(re) = self.raise(code) {
                        return Err(re);
                    }
                }
            }
        }
    }

    fn raise(&mut self, code: i32) -> Result<(), RunError> {
        let line = self.frames.last().map_or(0, |f| f.line);
        let Some(h) = self.handler.filter(|_| self.in_handler.is_none()) else {
            return Err(RunError { code, line, msg: format!("error {code}") });
        };
        self.err = code;
        self.erl = self.last_line;
        self.errline = line;
        let k = self.frames.len() - 1;
        let lpc = self.frames[k].line_pc;
        self.in_handler = Some((k, lpc));
        let main = self.prog.main;
        let slots = self.frames[0].slots.clone();
        self.frames.push(Frame {
            f: main,
            pc: h,
            regs: vec![V::I(0); self.prog.funcs[main].nregs],
            slots,
            owned: vec![],
            gosub: vec![],
            line_pc: h,
            line,
            ret_to: None,
        });
        Ok(())
    }

    fn resume(&mut self, how: u8, target: usize) -> R<()> {
        let Some((k, lpc)) = self.in_handler.take() else { return e(20) };
        self.err = 0;
        let keep = if how == 2 { 1 } else { k + 1 };
        while self.frames.len() > keep {
            self.pop_frame();
        }
        let prog = self.prog;
        let fr = self.frame();
        match how {
            0 => fr.pc = lpc,
            1 => {
                let ops = &prog.funcs[fr.f].ops;
                let mut pc = lpc + 1;
                while pc < ops.len() && !matches!(ops[pc], Op::Line(_)) {
                    pc += 1;
                }
                fr.pc = pc;
            }
            _ => fr.pc = target,
        }
        Ok(())
    }

    /// Executes one instruction; `Ok(false)` = program finished.
    fn step(&mut self) -> R<bool> {
        let prog = self.prog;
        let (fi, pc) = {
            let fr = self.frame();
            (fr.f, fr.pc)
        };
        let ops = &prog.funcs[fi].ops;
        let Some(op) = ops.get(pc) else {
            return self.do_ret(None);
        };
        self.frame().pc = pc + 1;
        match op {
            Op::Line(n) => {
                let fr = self.frame();
                fr.line = *n;
                fr.line_pc = pc;
            }
            Op::Nop => {}
            Op::Const(d, v) => self.set(*d, v.clone()),
            Op::Mov(d, s) => {
                let v = self.reg(*s).clone();
                self.set(*d, v)
            }
            Op::Bin(o, t, d, a, b) => {
                let v = bin(*o, *t, self.reg(*a), self.reg(*b))?;
                self.set(*d, v)
            }
            Op::Un(o, t, d, a) => {
                let x = self.reg(*a);
                let v = match (o, t.is_float()) {
                    (UnOp::Neg, true) if *t == Ty::F80 => V::X(-x.x()),
                    (UnOp::Neg, true) => V::F(-x.f()),
                    (UnOp::Neg, false) => V::I(wrap(*t, x.i().wrapping_neg())),
                    (UnOp::Not, _) => V::I(wrap(*t, !x.i())),
                };
                self.set(*d, v)
            }
            Op::Cmp(o, t, d, a, b) => {
                let (x, y) = (self.reg(*a), self.reg(*b));
                let ord = if *t == Ty::Str {
                    Some(x.s().cmp(y.s()))
                } else if *t == Ty::F80 {
                    x.x().compare(y.x())
                } else if t.is_float() {
                    x.f().partial_cmp(&y.f())
                } else if *t == Ty::U64 {
                    Some((x.i() as u64).cmp(&(y.i() as u64)))
                } else {
                    Some(x.i().cmp(&y.i()))
                };
                use std::cmp::Ordering::*;
                let r = match (o, ord) {
                    (CmpOp::Ne, None) => true,
                    (_, None) => false,
                    (CmpOp::Eq, Some(c)) => c == Equal,
                    (CmpOp::Ne, Some(c)) => c != Equal,
                    (CmpOp::Lt, Some(c)) => c == Less,
                    (CmpOp::Le, Some(c)) => c != Greater,
                    (CmpOp::Gt, Some(c)) => c == Greater,
                    (CmpOp::Ge, Some(c)) => c != Less,
                };
                self.set(*d, V::I(if r { -1 } else { 0 }))
            }
            Op::Cvt(to, from, d, s, round) => {
                let v = cvt(*to, *from, self.reg(*s), *round);
                self.set(*d, v)
            }
            Op::Concat(d, a, b) => {
                let mut s = self.reg(*a).s().to_vec();
                s.extend_from_slice(self.reg(*b).s());
                self.set(*d, V::S(s))
            }
            Op::Load(t, d, l) => {
                let v = self.load_var(*t, *l)?;
                self.set(*d, v)
            }
            Op::Store(t, l, s) => {
                let v = self.reg(*s).clone();
                self.store_var(*t, *l, &v)?
            }
            Op::Addr(d, l) => {
                let p = self.var_ptr(*l);
                self.set(*d, V::P(p))
            }
            Op::AAddr(d, l, idx) => {
                let ix: Vec<i64> = idx.iter().map(|r| self.reg(*r).i()).collect();
                let p = self.aaddr(*l, &ix)?;
                self.set(*d, V::P(p))
            }
            Op::Ld(t, d, p, off) => {
                let v = self.ld(*t, self.reg(*p).p() + u64::from(*off))?;
                self.set(*d, v)
            }
            Op::St(t, p, off, s) => {
                let v = self.reg(*s).clone();
                self.st(*t, self.reg(*p).p() + u64::from(*off), &v)?
            }
            Op::LdFs(d, p, off, len) => {
                let v = self.ldfs(self.reg(*p).p() + u64::from(*off), *len)?;
                self.set(*d, v)
            }
            Op::StFs(p, off, len, s) => {
                let v = self.reg(*s).s().to_vec();
                self.stfs(self.reg(*p).p() + u64::from(*off), *len, &v)?
            }
            Op::PAdd(d, p, o) => {
                let v = V::P(self.reg(*p).p().wrapping_add(self.reg(*o).i() as u64));
                self.set(*d, v)
            }
            Op::Copy(d, s, n) => {
                let src = self.bytes(self.reg(*s).p(), *n as usize)?.to_vec();
                let dp = self.reg(*d).p();
                self.bytes(dp, *n as usize)?.copy_from_slice(&src);
            }
            Op::Dim(k, l, b) => {
                let bounds = b.iter().map(|(x, y)| (self.reg(*x).i(), self.reg(*y).i())).collect();
                self.dim(*k, *l, bounds)?
            }
            Op::Erase(l) => {
                let a = self.arr(*l)?;
                a.dims.clear();
                if let Some(d) = a.data.take() {
                    self.free(d);
                }
            }
            Op::Bound(upper, d, l, k) => {
                let k = self.reg(*k).i();
                let a = self.arr(*l)?;
                if a.data.is_none() || k < 1 || k as usize > a.dims.len() {
                    return e(9);
                }
                let (lo, hi) = a.dims[k as usize - 1];
                self.set(*d, V::I(if *upper { hi } else { lo }))
            }
            Op::Jmp(t) => self.frame().pc = *t,
            Op::Jz(c, t) => {
                if self.reg(*c).i() == 0 {
                    self.frame().pc = *t
                }
            }
            Op::Jnz(c, t) => {
                if self.reg(*c).i() != 0 {
                    self.frame().pc = *t
                }
            }
            Op::Gosub(t) => {
                let fr = self.frame();
                fr.gosub.push(pc + 1);
                fr.pc = *t;
            }
            Op::GRet | Op::GRetL(_) => {
                let fr = self.frame();
                let Some(back) = fr.gosub.pop() else { return e(3) };
                fr.pc = if let Op::GRetL(t) = op { *t } else { back };
            }
            Op::Call(d, f, args) => {
                let a: Vec<V> = args.iter().map(|r| self.reg(*r).clone()).collect();
                self.enter(*f, a, *d)?
            }
            Op::Ret(v) => {
                let v = v.map(|r| self.reg(r).clone());
                return self.do_ret(v);
            }
            Op::Rt(d, name, args) => {
                let a: Vec<V> = args.iter().map(|r| self.reg(*r).clone()).collect();
                let v = rt::call(self, name, a)?;
                if let (Some(d), Some(v)) = (d, v) {
                    self.set(*d, v)
                }
            }
            Op::OnError(h) => self.handler = *h,
            Op::Resume(how, t) => self.resume(*how, *t)?,
            Op::Error(c) => {
                let c = self.reg(*c).i();
                return e(if (1..=255).contains(&c) { c as i32 } else { 5 });
            }
            Op::Read(t, d) => {
                let Some(item) = prog.data.get(self.data_pos) else { return e(4) };
                self.data_pos += 1;
                let v = if *t == Ty::Str {
                    V::S(item.clone())
                } else {
                    let trimmed: Vec<u8> = item.iter().copied().filter(|c| *c != b' ').collect();
                    if trimmed.is_empty() {
                        cvt(*t, Ty::F64, &V::F(0.0), true)
                    } else {
                        if !rt::numeric_text(&trimmed) {
                            return e(13);
                        }
                        let f = rt::val(&trimmed);
                        cvt(*t, Ty::F64, &V::F(f), true)
                    }
                };
                self.set(*d, v)
            }
            Op::Restore(i) => self.data_pos = *i,
            Op::End => {
                self.rt.flush();
                return Ok(false);
            }
            Op::System(c) => {
                self.rt.flush();
                let c = c.map_or(0, |r| self.reg(r).i() as i32);
                return Err(Stop::Exit(c));
            }
        }
        Ok(true)
    }

    fn do_ret(&mut self, v: Option<V>) -> R<bool> {
        if self.frames.len() == 1 || self.in_handler.is_some_and(|(k, _)| self.frames.len() == k + 2) {
            self.rt.flush();
            return Ok(false);
        }
        let fr = self.pop_frame();
        if let (Some(d), Some(v)) = (fr.ret_to, v) {
            self.set(d, v);
        }
        Ok(true)
    }
}

fn bin(o: BinOp, t: Ty, a: &V, b: &V) -> R<V> {
    if t == Ty::F80 {
        let (x, y) = (a.x(), b.x());
        return Ok(V::X(match o {
            BinOp::Add => x + y,
            BinOp::Sub => x - y,
            BinOp::Mul => x * y,
            BinOp::Div => x / y,
            BinOp::Pow => qb_float80::qb::pow2(x, y).map_err(Stop::Err)?,
            _ => return e(5),
        }));
    }
    if t.is_float() {
        let (x, y) = (a.f(), b.f());
        let r = match o {
            BinOp::Add => x + y,
            BinOp::Sub => x - y,
            BinOp::Mul => x * y,
            BinOp::Div => x / y,
            BinOp::Pow => x.powf(y),
            _ => return e(5),
        };
        return Ok(V::F(fround(t, r)));
    }
    let (x, y) = (a.i(), b.i());
    let uns = t == Ty::U64;
    let r = match o {
        BinOp::Add => x.wrapping_add(y),
        BinOp::Sub => x.wrapping_sub(y),
        BinOp::Mul => x.wrapping_mul(y),
        BinOp::IDiv | BinOp::Mod => {
            if y == 0 {
                return e(11);
            }
            match (o, uns) {
                (BinOp::IDiv, true) => ((x as u64) / (y as u64)) as i64,
                (BinOp::IDiv, false) => x.wrapping_div(y),
                (_, true) => ((x as u64) % (y as u64)) as i64,
                _ => x.wrapping_rem(y),
            }
        }
        BinOp::And => x & y,
        BinOp::Or => x | y,
        BinOp::Xor => x ^ y,
        BinOp::Shl => x.wrapping_shl((y as u32) % (t.size() * 8)),
        BinOp::Shr => {
            let s = (y as u32) % (t.size() * 8);
            if t.is_signed() {
                x >> s
            } else {
                let m = if t.size() == 8 { u64::MAX } else { (1u64 << (t.size() * 8)) - 1 };
                (((x as u64) & m) >> s) as i64
            }
        }
        _ => return e(5),
    };
    Ok(V::I(wrap(t, r)))
}

pub(crate) fn cvt(to: Ty, from: Ty, v: &V, round: bool) -> V {
    if to == Ty::F80 {
        return V::X(match from {
            Ty::F80 => v.x(),
            Ty::F32 | Ty::F64 => F80::from_f64(v.f()),
            Ty::U64 => F80::from_u64(v.i() as u64),
            _ => F80::from_i64(v.i()),
        });
    }
    if from == Ty::F80 {
        let x = v.x();
        if to.is_float() {
            return V::F(if to == Ty::F32 { f64::from(x.to_f32()) } else { x.to_f64() });
        }
        if to == Ty::U64 {
            return V::I(x.to_u64_cast() as i64);
        }
        let n = if round { qb_float80::qb::qbr(x) } else { x.to_i64(Round::Zero) };
        return V::I(wrap(to, n));
    }
    if to.is_float() {
        let f = if from.is_float() {
            v.f()
        } else if from == Ty::U64 {
            v.i() as u64 as f64
        } else {
            v.i() as f64
        };
        return V::F(fround(to, f));
    }
    if from.is_float() {
        let f = v.f();
        if round && to != Ty::U64 {
            // x87 FISTP semantics of the C++ backend's qbr()
            return V::I(wrap(to, qb_runtime::round::qb_qbr_f64(f)));
        }
        let f = if round { f.round_ties_even() } else { f.trunc() };
        let n = if to == Ty::U64 { f as u64 as i64 } else { f as i64 };
        return V::I(wrap(to, n));
    }
    V::I(wrap(to, v.i()))
}

/// Runs a parsed QIR v1 module; returns the program's exit code.
pub fn run(m: &ir::Module, out: &mut dyn Write) -> Result<i32, RunError> {
    let prog = load(m)?;
    let mut vm = Vm {
        prog: &prog,
        blocks: Vec::new(),
        free_blocks: Vec::new(),
        strs: vec![Vec::new()],
        free_strs: Vec::new(),
        globals: Vec::new(),
        frames: Vec::new(),
        handler: None,
        in_handler: None,
        err: 0,
        erl: 0,
        errline: 0,
        last_line: 0,
        cmem: vec![0; 1 << 20],
        cmem_next: 16,
        field_store: false,
        cmem_strs: HashMap::new(),
        def_seg: None,
        reclen: HashMap::new(),
        field_buf: HashMap::new(),
        field_vars: Vec::new(),
        field_cur: (0, 0),
        net: netx::Net::default(),
        net_pending_error: 0,
        data_pos: 0,
        rt: rt::RtState::new(out),
    };
    for g in 0..prog.globals.len() {
        let p = vm.new_var(prog.globals[g]);
        vm.globals.push(p);
    }
    let _ = prog.console_only;
    let r = vm.run();
    vm.rt.flush();
    r
}

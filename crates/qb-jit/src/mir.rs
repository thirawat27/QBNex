//! MIR: the typed, block-structured form the JIT compiles.
//!
//! It mirrors QIR v1 (typed registers, labels resolved to blocks) one level
//! lower: every register is an 8-byte *slot* of the frame the VM and the JIT
//! share, so leaving native code at any resume point is just "store the
//! registers back and tell the VM which pc to continue at".
//!
//! Slot encoding (shared with the VM): integers sign-extended to 64 bits,
//! SINGLE = its `f32` bits in the low 32 bits, DOUBLE = its `f64` bits,
//! STRING / array / BYREF pointer = an opaque 64-bit handle.
//!
//! Semantics (copied from the qb-exec VM, which copies the C++ backend):
//! * `+ - *` on integers wrap in the operand width (C cast);
//! * `\` and `MOD` by zero raise error 11, `/` by 0.0 raises error 11;
//! * float -> integer conversion rounds half to even exactly like
//!   `qb_runtime::round::qb_qbr_f64` and raises error 6 when the result does
//!   not fit an INTEGER/LONG target;
//! * integer -> SINGLE goes through DOUBLE (the VM holds SINGLE values at
//!   double precision until they are stored);
//! * comparisons yield -1 / 0.

/// Frame slot index.
pub type Reg = u32;
/// Block index inside a function.
pub type BlockId = u32;
/// Function index inside a module.
pub type FuncId = u32;
/// Index into the host function table of the module.
pub type HostId = u32;

/// Sentinel "entry pc" meaning a normal call (start at block 0).
pub const ENTRY_CALL: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    I16,
    I32,
    I64,
    F32,
    F64,
    /// Opaque 64-bit handle: string, array descriptor, BYREF pointer.
    Ptr,
}

impl Ty {
    pub fn is_int(self) -> bool {
        matches!(self, Ty::I16 | Ty::I32 | Ty::I64 | Ty::Ptr)
    }
    pub fn is_float(self) -> bool {
        matches!(self, Ty::F32 | Ty::F64)
    }
    /// Size of an array element of this type.
    pub fn size(self) -> u32 {
        match self {
            Ty::I16 => 2,
            Ty::I32 | Ty::F32 => 4,
            Ty::I64 | Ty::F64 | Ty::Ptr => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Imm {
    I(i64),
    F(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    /// Float only; divisor 0 raises error 11 (`/` in the VM).
    Div,
    /// Float only; IEEE division (QIR `div`).
    FDiv,
    /// Float only.
    Pow,
    /// Integer only.
    IDiv,
    /// Integer only.
    Mod,
    And,
    Or,
    Xor,
    Eqv,
    Imp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MathOp {
    /// Same type in/out (int: wrapping).
    Abs,
    /// Result: any int type (-1/0/1).
    Sgn,
    /// INT(): floor, float in/out.
    Floor,
    /// FIX(): truncate, float in/out.
    Fix,
    /// SQR: float; negative raises error 5.
    Sqrt,
    /// F64 -> F64 rounded through f32 (store into SINGLE held as double).
    RoundF32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cc {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Argument of a JIT-to-JIT call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    /// Copy this slot's value into the callee parameter slot.
    Val(Reg),
    /// Pass the address of this caller slot (BYREF); the callee slot holds a pointer.
    Ref(Reg),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inst {
    /// Statement start: records line/pc for errors and polls events.
    Line { line: u32, pc: u32 },
    Const { dst: Reg, imm: Imm },
    Mov { dst: Reg, src: Reg },
    /// Conversion between the slot types of `src` and `dst`.
    Conv { dst: Reg, src: Reg },
    /// `a`, `b`, `dst` share one type.
    Bin { op: BinOp, dst: Reg, a: Reg, b: Reg },
    /// `a`, `b` share one numeric type; `dst` is an integer slot (-1/0).
    Cmp { cc: Cc, dst: Reg, a: Reg, b: Reg },
    Neg { dst: Reg, src: Reg },
    Not { dst: Reg, src: Reg },
    /// Module-level (SHARED) variable access; `ty` is the slot type.
    GLoad { dst: Reg, slot: u32 },
    GStore { slot: u32, src: Reg },
    /// Address of a frame slot (makes the slot memory-resident).
    AddrOf { dst: Reg, slot: Reg },
    /// Address of a global slot.
    GAddrOf { dst: Reg, slot: u32 },
    /// Load through a pointer slot; type = type of `dst`.
    LoadPtr { dst: Reg, ptr: Reg },
    /// Store through a pointer slot; type = type of `src`.
    StorePtr { ptr: Reg, src: Reg },
    /// Array element load; `arr` holds a `*mut ArrDesc`, element type = type of `dst`.
    ALoad { dst: Reg, arr: Reg, idx: Vec<Reg> },
    AStore { arr: Reg, idx: Vec<Reg>, src: Reg },
    /// Call another function of the same module. `pc` = VM pc of the call
    /// (recorded in the deopt chain when the callee leaves abnormally).
    Call { func: FuncId, args: Vec<Arg>, dst: Option<Reg>, pc: u32 },
    /// Call a host (runtime) function: `extern "C" fn(*mut Ctx, args...) -> ret`.
    Host { f: HostId, args: Vec<Reg>, dst: Option<Reg> },
    /// Raise QB error `code` (ERROR n).
    Raise { code: Reg },
    /// Float -> integer: QB rounding then C cast to the target width (no
    /// overflow check; QIR `cvt`). Int sources: same as `Conv`.
    ConvWrap { dst: Reg, src: Reg },
    /// Float -> integer truncating toward zero, saturating (QIR `trunc`).
    Trunc { dst: Reg, src: Reg },
    /// Unary math; types per [`MathOp`].
    Math { op: MathOp, dst: Reg, src: Reg },
    /// Let the VM execute its instruction `op` on this frame (anything the
    /// JIT does not compile natively). `reads` are flushed to the frame before,
    /// `writes` reloaded after. The VM callback returns a status; non-OK
    /// leaves native code with that status (ctx fields set by the VM).
    VmOp { op: u32, reads: Vec<Reg>, writes: Vec<Reg> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Jmp(BlockId),
    Br { cond: Reg, then: BlockId, els: BlockId },
    /// Leave the function normally (END SUB / END FUNCTION / EXIT SUB).
    Ret,
    /// `GOSUB target`; `ret` continues after RETURN. `ret_pc` = VM pc of `ret`
    /// (what goes on the shared GOSUB stack).
    Gosub { target: BlockId, ret: BlockId, ret_pc: u32 },
    /// `RETURN`; `pc` = VM pc of the RETURN (deopt point if the return pc is unknown).
    Return { pc: u32 },
    /// END / SYSTEM.
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// VM pc of the block start, if the VM can resume here (OSR entry / exit target).
    pub pc: Option<u32>,
    pub insts: Vec<Inst>,
    pub term: Term,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Func {
    pub name: String,
    /// Type of every frame slot.
    pub slots: Vec<Ty>,
    /// Parameter slots, in order (filled by the caller).
    pub params: Vec<Reg>,
    /// Slot holding the FUNCTION result, copied to the caller's `dst`.
    pub ret: Option<Reg>,
    pub blocks: Vec<Block>,
}

/// Host function signature.
#[derive(Debug, Clone, PartialEq)]
pub struct HostSig {
    pub name: String,
    pub params: Vec<Ty>,
    pub ret: Option<Ty>,
    /// The function may set `Ctx::err`; checked after the call.
    pub may_error: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostFn {
    pub sig: HostSig,
    pub addr: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub funcs: Vec<Func>,
    pub hosts: Vec<HostFn>,
    /// Type of every global slot.
    pub globals: Vec<Ty>,
    /// VM callback for `VmOp`: `extern "C" fn(*mut Ctx, frame: *mut u64, op: u32) -> u32`.
    pub vm_op: usize,
}

/// Validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirError(pub String);

impl Module {
    /// Checks the type rules the code generator relies on.
    pub fn validate(&self) -> Result<(), MirError> {
        for (fi, f) in self.funcs.iter().enumerate() {
            let e = |m: String| MirError(format!("{} (func {fi} {}): {m}", "mir", f.name));
            let ty = |r: Reg| f.slots.get(r as usize).copied().ok_or_else(|| e(format!("slot {r} out of range")));
            if f.blocks.is_empty() {
                return Err(e("no blocks".into()));
            }
            let nb = f.blocks.len() as u32;
            for p in &f.params {
                ty(*p)?;
            }
            if let Some(r) = f.ret {
                ty(r)?;
            }
            for (bi, b) in f.blocks.iter().enumerate() {
                for i in &b.insts {
                    match i {
                        Inst::Line { .. } => {}
                        Inst::Const { dst, imm } => {
                            let t = ty(*dst)?;
                            if matches!(imm, Imm::F(_)) && !t.is_float() {
                                return Err(e(format!("block {bi}: float constant into int slot")));
                            }
                        }
                        Inst::Mov { dst, src } => {
                            if ty(*dst)? != ty(*src)? {
                                return Err(e(format!("block {bi}: Mov type mismatch")));
                            }
                        }
                        Inst::Conv { dst, src } => {
                            ty(*dst)?;
                            ty(*src)?;
                        }
                        Inst::Bin { op, dst, a, b } => {
                            let t = ty(*dst)?;
                            if ty(*a)? != t || ty(*b)? != t || t == Ty::Ptr {
                                return Err(e(format!("block {bi}: Bin operand types")));
                            }
                            let float_only = matches!(op, BinOp::Div | BinOp::FDiv | BinOp::Pow);
                            let int_only = !float_only && !matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul);
                            if (float_only && !t.is_float()) || (int_only && t.is_float()) {
                                return Err(e(format!("block {bi}: {op:?} on {t:?}")));
                            }
                        }
                        Inst::Cmp { dst, a, b, .. } => {
                            if ty(*a)? != ty(*b)? || !ty(*dst)?.is_int() {
                                return Err(e(format!("block {bi}: Cmp types")));
                            }
                        }
                        Inst::Neg { dst, src } | Inst::Not { dst, src } => {
                            if ty(*dst)? != ty(*src)? {
                                return Err(e(format!("block {bi}: unary types")));
                            }
                            if matches!(i, Inst::Not { .. }) && !ty(*dst)?.is_int() {
                                return Err(e(format!("block {bi}: NOT on float")));
                            }
                        }
                        Inst::GLoad { dst, slot } => {
                            let g = self.globals.get(*slot as usize).ok_or_else(|| e("global out of range".into()))?;
                            if *g != ty(*dst)? {
                                return Err(e(format!("block {bi}: GLoad type")));
                            }
                        }
                        Inst::GStore { slot, src } => {
                            let g = self.globals.get(*slot as usize).ok_or_else(|| e("global out of range".into()))?;
                            if *g != ty(*src)? {
                                return Err(e(format!("block {bi}: GStore type")));
                            }
                        }
                        Inst::AddrOf { dst, slot } => {
                            ty(*slot)?;
                            if ty(*dst)? != Ty::Ptr {
                                return Err(e("AddrOf dst must be Ptr".into()));
                            }
                        }
                        Inst::GAddrOf { dst, slot } => {
                            if *slot as usize >= self.globals.len() || ty(*dst)? != Ty::Ptr {
                                return Err(e("GAddrOf".into()));
                            }
                        }
                        Inst::LoadPtr { dst, ptr } | Inst::StorePtr { ptr, src: dst } => {
                            ty(*dst)?;
                            if ty(*ptr)? != Ty::Ptr {
                                return Err(e("pointer slot must be Ptr".into()));
                            }
                        }
                        Inst::ALoad { dst: v, arr, idx } | Inst::AStore { arr, idx, src: v } => {
                            ty(*v)?;
                            if ty(*arr)? != Ty::Ptr || idx.is_empty() || idx.len() > crate::ctx::MAX_DIMS {
                                return Err(e("array access".into()));
                            }
                            for r in idx {
                                if !ty(*r)?.is_int() {
                                    return Err(e("array index must be integer".into()));
                                }
                            }
                        }
                        Inst::Call { func, args, dst, .. } => {
                            let callee = self.funcs.get(*func as usize).ok_or_else(|| e("call target".into()))?;
                            if callee.params.len() != args.len() {
                                return Err(e(format!("call {}: arity", callee.name)));
                            }
                            for (a, p) in args.iter().zip(&callee.params) {
                                let pt = callee.slots[*p as usize];
                                match a {
                                    Arg::Val(r) if ty(*r)? != pt => return Err(e("call arg type".into())),
                                    Arg::Ref(r) => {
                                        ty(*r)?;
                                        if pt != Ty::Ptr {
                                            return Err(e("BYREF param must be Ptr".into()));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            match (dst, callee.ret) {
                                (Some(d), Some(r)) if ty(*d)? != callee.slots[r as usize] => {
                                    return Err(e("call result type".into()))
                                }
                                (Some(_), None) => return Err(e("call result of a SUB".into())),
                                _ => {}
                            }
                        }
                        Inst::Host { f: h, args, dst } => {
                            let hf = self.hosts.get(*h as usize).ok_or_else(|| e("host id".into()))?;
                            if hf.sig.params.len() != args.len() {
                                return Err(e(format!("host {}: arity", hf.sig.name)));
                            }
                            for (a, t) in args.iter().zip(&hf.sig.params) {
                                if ty(*a)? != *t {
                                    return Err(e(format!("host {}: arg type", hf.sig.name)));
                                }
                            }
                            match (dst, hf.sig.ret) {
                                (Some(d), Some(t)) if ty(*d)? != t => return Err(e("host ret type".into())),
                                (Some(_), None) => return Err(e("host has no result".into())),
                                _ => {}
                            }
                        }
                        Inst::ConvWrap { dst, src } | Inst::Trunc { dst, src } => {
                            ty(*dst)?;
                            ty(*src)?;
                            if matches!(i, Inst::Trunc { .. }) && !(ty(*src)?.is_float() && ty(*dst)? != Ty::Ptr && !ty(*dst)?.is_float()) {
                                return Err(e("Trunc is float -> int".into()));
                            }
                        }
                        Inst::Math { op, dst, src } => {
                            let (d, s) = (ty(*dst)?, ty(*src)?);
                            let ok = match op {
                                MathOp::Abs => d == s && d != Ty::Ptr,
                                MathOp::Sgn => d.is_int() && d != Ty::Ptr && s != Ty::Ptr,
                                MathOp::Floor | MathOp::Fix | MathOp::Sqrt => d == s && d.is_float(),
                                MathOp::RoundF32 => d == Ty::F64 && s == Ty::F64,
                            };
                            if !ok {
                                return Err(e(format!("block {bi}: Math {op:?} types")));
                            }
                        }
                        Inst::VmOp { reads, writes, .. } => {
                            if self.vm_op == 0 {
                                return Err(e("VmOp without a VM callback".into()));
                            }
                            for r in reads.iter().chain(writes) {
                                ty(*r)?;
                            }
                        }
                        Inst::Raise { code } => {
                            if !ty(*code)?.is_int() {
                                return Err(e("Raise code must be int".into()));
                            }
                        }
                    }
                }
                let chk = |t: BlockId| if t < nb { Ok(()) } else { Err(e(format!("block {bi}: bad target {t}"))) };
                match &b.term {
                    Term::Jmp(t) => chk(*t)?,
                    Term::Br { cond, then, els } => {
                        if !ty(*cond)?.is_int() {
                            return Err(e("branch on float".into()));
                        }
                        chk(*then)?;
                        chk(*els)?;
                    }
                    Term::Gosub { target, ret, .. } => {
                        chk(*target)?;
                        chk(*ret)?;
                    }
                    Term::Ret | Term::Return { .. } | Term::End => {}
                }
            }
        }
        Ok(())
    }
}

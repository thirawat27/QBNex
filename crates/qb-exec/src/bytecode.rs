//! Typed register bytecode executed by [`crate::Vm`].
//!
//! The loader (`lower.rs`) turns QIR into this form once, at start-up:
//! every value has a static type, every variable and temporary is a register
//! of the frame, labels are absolute instruction indices and builtins are
//! resolved. The interpreter therefore never looks at names or type tags.
//!
//! Three register banks per frame:
//! - `r` numeric: 64-bit cells. Integers are stored sign-extended and already
//!   wrapped to their QBasic width (`INTEGER` = i16, `LONG` = i32,
//!   `_INTEGER64` = i64); `SINGLE`/`DOUBLE` are `f64` bits (`SINGLE` temporaries
//!   keep double precision until stored, like the C++ backend).
//! - `s` strings (`Vec<u8>`). A string *source* operand with [`K`] set names a
//!   constant of [`Module::kstr`] instead of a register.
//! - `a` arrays. An array operand with [`G`] set is a module-level array
//!   (absolute index), otherwise a register of the current frame.
//!
//! A numeric operand of `GetG`/`SetG` is an absolute index of the module frame
//! (frame 0 starts at 0), used for `DIM SHARED` variables inside SUBs.
//!
//! Every function's code ends with `Ret` (SUB/FUNCTION) or `End` (main), and
//! [`Module::validate`] checks all register operands against the frame sizes
//! before the VM runs, which is what lets the interpreter index unchecked.

use std::fmt::Write as _;

pub type R = u32;

/// String source operand: constant pool index instead of a register.
pub const K: u32 = 1 << 31;
/// Array operand: module-level array (absolute index).
pub const G: u32 = 1 << 31;
/// "no operand" (optional register / handler).
pub const NONE: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    I16,
    I32,
    I64,
    /// SINGLE (values are f64 that were rounded to f32 when stored)
    F32,
    F64,
    Str,
    /// `_FLOAT` (x87 80-bit): two numeric registers `r` (significand), `r + 1` (sign/exponent)
    F80,
}

impl Ty {
    pub fn is_int(self) -> bool {
        matches!(self, Ty::I16 | Ty::I32 | Ty::I64)
    }
    pub fn is_float(self) -> bool {
        matches!(self, Ty::F32 | Ty::F64 | Ty::F80)
    }
    pub fn is_num(self) -> bool {
        self != Ty::Str
    }
    pub fn width(self) -> W {
        match self {
            Ty::I16 => W::W16,
            Ty::I32 => W::W32,
            _ => W::W64,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Ty::I16 => "i16",
            Ty::I32 => "i32",
            Ty::I64 => "i64",
            Ty::F32 => "f32",
            Ty::F64 => "f64",
            Ty::Str => "str",
            Ty::F80 => "f80",
        }
    }
}

/// Integer width of a wrapping/range-checked integer operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W {
    W16,
    W32,
    W64,
    /// QIR widths: `_BYTE`, `_UNSIGNED _BYTE / INTEGER / LONG`
    W8,
    U8,
    U16,
    U32,
    /// `_UNSIGNED _INTEGER64`: same bits as W64; division, shifts, compares and
    /// float conversion are unsigned
    U64,
}

impl W {
    #[inline(always)]
    pub fn wrap(self, v: i64) -> i64 {
        match self {
            W::W16 => v as i16 as i64,
            W::W32 => v as i32 as i64,
            W::W64 => v,
            W::W8 => v as i8 as i64,
            W::U8 => v as u8 as i64,
            W::U16 => v as u16 as i64,
            W::U32 => v as u32 as i64,
            W::U64 => v,
        }
    }
    pub fn bits(self) -> u32 {
        match self {
            W::W8 | W::U8 => 8,
            W::W16 | W::U16 => 16,
            W::W32 | W::U32 => 32,
            W::W64 | W::U64 => 64,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            W::W16 => "i16",
            W::W32 => "i32",
            W::W64 => "i64",
            W::W8 => "i8",
            W::U8 => "u8",
            W::U16 => "u16",
            W::U32 => "u32",
            W::U64 => "u64",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Cmp {
    pub fn not(self) -> Cmp {
        match self {
            Cmp::Eq => Cmp::Ne,
            Cmp::Ne => Cmp::Eq,
            Cmp::Lt => Cmp::Ge,
            Cmp::Le => Cmp::Gt,
            Cmp::Gt => Cmp::Le,
            Cmp::Ge => Cmp::Lt,
        }
    }
    #[inline(always)]
    pub fn test<T: PartialOrd>(self, a: T, b: T) -> bool {
        match self {
            Cmp::Eq => a == b,
            Cmp::Ne => a != b,
            Cmp::Lt => a < b,
            Cmp::Le => a <= b,
            Cmp::Gt => a > b,
            Cmp::Ge => a >= b,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Cmp::Eq => "eq",
            Cmp::Ne => "ne",
            Cmp::Lt => "lt",
            Cmp::Le => "le",
            Cmp::Gt => "gt",
            Cmp::Ge => "ge",
        }
    }
}

/// Numeric unary operations (operand and result types fixed by the loader).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Un {
    NegI(W),
    NegF,
    /// bitwise NOT of a normalized integer (stays normalized)
    Not,
}

/// Scalar type of a memory access (QIR `ld`/`st`, memory-backed variables).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MTy {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    /// `_FLOAT`: 10 bytes <-> register pair
    F80,
    Ptr,
    /// string handle (8 bytes); the register is a string register
    Str,
}

impl MTy {
    pub fn size(self) -> usize {
        match self {
            MTy::I8 | MTy::U8 => 1,
            MTy::I16 | MTy::U16 => 2,
            MTy::I32 | MTy::U32 | MTy::F32 => 4,
            MTy::F80 => 10,
            _ => 8,
        }
    }
}

/// Storage of a memory-backed variable or array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MVar {
    /// bytes of the variable / of one element
    pub size: u32,
    /// initial byte (space for `STRING * n`)
    pub fill: u8,
    /// holds string handles
    pub strs: bool,
    pub array: bool,
}

/// `_FLOAT` binary operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XOp {
    Add,
    Sub,
    Mul,
    Div,
    /// QB `^` (powl semantics, error 5 on a domain error)
    Pow,
}

/// Inner operation of the accumulate superinstruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccOp {
    Mul,
    IDiv,
    Mod,
    And,
    Or,
    Xor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimMode {
    Dim,
    Redim,
    RedimPreserve,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumeK {
    Same,
    Next,
    Label,
}

/// One instruction. `d` = destination, `a`/`b`/`s` = sources, `t` = jump target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ins {
    // ---- numeric moves and conversions
    Ldi { d: R, v: i32 },
    /// raw 64-bit constant `Module::nconst[k]`
    Ldk { d: R, k: u32 },
    Mov { d: R, s: R },
    GetG { d: R, g: R },
    SetG { g: R, s: R },
    IToF { d: R, s: R },
    /// unsigned 64-bit integer -> float
    IToFU { d: R, s: R },
    /// integer -> narrower integer (C cast, wraps)
    Wrap { d: R, s: R, w: W },
    /// f64 -> f32 -> f64 (store into SINGLE)
    RoundF32 { d: R, s: R },
    /// QIR `cvt`/`trunc` float -> int: ties-to-even (or toward zero), C cast, wrap; never an error
    CvtFI { d: R, s: R, w: W, trunc: bool },
    /// QIR shifts (count taken modulo the width)
    Shl { d: R, a: R, b: R, w: W },
    Shr { d: R, a: R, b: R, w: W },
    /// IEEE division (QIR `div`: no error on zero)
    DivFI { d: R, a: R, b: R },
    // ---- integer arithmetic (operands normalized, result wrapped)
    AddI16 { d: R, a: R, b: R },
    AddI32 { d: R, a: R, b: R },
    AddI64 { d: R, a: R, b: R },
    SubI16 { d: R, a: R, b: R },
    SubI32 { d: R, a: R, b: R },
    SubI64 { d: R, a: R, b: R },
    MulI16 { d: R, a: R, b: R },
    MulI32 { d: R, a: R, b: R },
    MulI64 { d: R, a: R, b: R },
    IDiv { d: R, a: R, b: R, w: W },
    Mod { d: R, a: R, b: R, w: W },
    And { d: R, a: R, b: R },
    Or { d: R, a: R, b: R },
    Xor { d: R, a: R, b: R },
    /// `d = wo(d + wi(a op b))` — superinstruction for `s = s + a op b`
    /// (reads `d`; never retargeted)
    Acc { op: AccOp, d: R, a: R, b: R, wi: W, wo: W },
    // ---- float arithmetic
    AddF { d: R, a: R, b: R },
    SubF { d: R, a: R, b: R },
    MulF { d: R, a: R, b: R },
    PowF { d: R, a: R, b: R },
    Un { op: Un, d: R, s: R },
    // ---- comparisons: d = -1 / 0 (INTEGER)
    CmpI { c: Cmp, d: R, a: R, b: R },
    CmpF { c: Cmp, d: R, a: R, b: R },
    CmpS { c: Cmp, d: R, a: R, b: R },
    /// unsigned 64-bit compare
    CmpU { c: Cmp, d: R, a: R, b: R },
    // ---- control flow
    Jmp { t: u32 },
    /// backward jump: event safe point
    Loop { t: u32 },
    JzI { s: R, t: u32 },
    JnzI { s: R, t: u32 },
    /// jump when `a c b` holds (compare fused with JZ/JNZ)
    BrI { c: Cmp, a: R, b: R, t: u32 },
    BrF { c: Cmp, a: R, b: R, t: u32 },
    BrS { c: Cmp, a: R, b: R, t: u32 },
    BrU { c: Cmp, a: R, b: R, t: u32 },
    /// FOR test with an explicit continue point: exit to `t` when done, else
    /// continue at this pc + `skip` (replaces the emitter's two-way step test)
    ForTestKI { v: R, lim: R, step: R, t: u32, skip: u8 },
    ForTestKF { v: R, lim: R, step: R, t: u32, skip: u8 },
    /// Back edge of an emitter FOR loop: `c = lim+1 (step) + iv` (i64 counter),
    /// `iv = w(c)`, then jump to `body` (safe point) while in range, else fall
    /// through to the loop exit. `lim` and `step` are registers `lim`, `lim + 1`.
    LoopForI { c: R, iv: R, lim: R, body: u32, w: W },
    Gosub { t: u32 },
    Return,
    /// RETURN label
    ReturnL { t: u32 },
    /// call function `f` with argument descriptor `Module::calls[desc]` (safe point)
    Call { f: u32, desc: u32 },
    Ret,
    End,
    /// SYSTEM [code]: `s == NONE` = 0
    System { s: R },
    OnError { t: u32 },
    /// ON TIMER(slot, secs) GOSUB t (`s` slot register, `x` seconds register)
    OnTimer { s: R, x: R, t: u32 },
    /// DECLARE LIBRARY call (`Module::xcalls[c]`)
    XCall { c: u32 },
    Resume { k: ResumeK, t: u32 },
    /// raise the error whose code is in register `s`
    Raise { s: R },
    // ---- strings
    MovS { d: R, s: R },
    /// move (swap buffers) — source is a dead temporary
    TakeS { d: R, s: R },
    GetGS { d: R, g: R },
    SetGS { g: R, s: R },
    /// d = a + b; `d == a` appends in place
    Concat { d: R, a: R, b: R },
    LenS { d: R, s: R },
    Chr { d: R, s: R },
    Asc { d: R, s: R },
    Left { d: R, s: R, n: R },
    Right { d: R, s: R, n: R },
    /// MID$(s, start[, n]); `n == NONE` = to the end
    Mid { d: R, s: R, start: R, n: R },
    /// QIR `rt` builtin `Module::rtfns[f]` (shared runtime, `qb_ir::builtins`); `d == NONE` = no result
    Rt { f: u32, d: R, args: u32 },
    // ---- arrays (`i`/`dims` = first of `n` / `2n` consecutive integer registers)
    Dim { a: R, dims: R, n: u32, mode: DimMode },
    LdE { d: R, a: R, i: R, n: u32 },
    StE { a: R, i: R, n: u32, s: R },
    LdES { d: R, a: R, i: R, n: u32 },
    StES { a: R, i: R, n: u32, s: R },
    LdE1 { d: R, a: R, i: R },
    StE1 { a: R, i: R, s: R },
    LdE1S { d: R, a: R, i: R },
    StE1S { a: R, i: R, s: R },
    Bound { d: R, a: R, dim: R, upper: bool },
    Erase { a: R },
    // ---- _FLOAT (register pairs, see Ty::F80)
    MovX { d: R, s: R },
    XBin { op: XOp, d: R, a: R, b: R },
    XNeg { d: R, s: R },
    /// compare -> -1 / 0 in a numeric register (NaN: only `ne` holds)
    XCmp { c: Cmp, d: R, a: R, b: R },
    /// integer register -> F80 (`u` = unsigned 64-bit source)
    XFromI { d: R, s: R, u: bool },
    /// f64 register -> F80
    XFromF { d: R, s: R },
    /// F80 -> f64 register (`f32` = round to SINGLE)
    XToF { d: R, s: R, f32: bool },
    /// F80 -> integer: qbr (or truncation), wrapped to `w`; U64 = C cast
    XToI { d: R, s: R, w: W, trunc: bool },
    // ---- memory (QIR programs with TYPE / STRING * n / pointers); `p`, `a` = pointer registers
    /// d = new storage for `Module::mvars[v]`, freed when the frame returns
    NewV { d: R, v: u32 },
    LdM { t: MTy, d: R, p: R, off: u32 },
    StM { t: MTy, p: R, off: u32, s: R },
    LdFs { d: R, p: R, off: u32, len: u32 },
    StFs { p: R, off: u32, len: u32, s: R },
    PAddM { d: R, p: R, o: R },
    CopyM { d: R, s: R, n: u32 },
    DimM { a: R, dims: R, n: u32, mode: DimMode },
    AAddrM { d: R, a: R, i: R, n: u32 },
    BoundM { d: R, a: R, dim: R, upper: bool },
    EraseM { a: R },
    // ---- misc runtime
    /// READ: next DATA item as string (`str`) into s-register, else its VAL into r-register (f64)
    /// READ of a string: next DATA item
    ReadS { d: R },
    Restore { i: u32 },
    /// QIR numeric READ of the next DATA item into a target of QB type
    /// `typ` (`func_read_float` / `func_read_int64` / `func_read_uint64`):
    /// integer targets get the i64 value, float targets the f64 value;
    /// overflow = error 6, malformed number = error 2 (the item is not consumed)
    ReadN { d: R, typ: i32 },
    // ---- events (ON TIMER / ON KEY / ...), delivered at safe points
}

/// Where a BYREF argument lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// numeric register of the frame
    N(R, Ty),
    /// module-level numeric (absolute)
    NG(R, Ty),
    S(R),
    SG(R),
    /// whole array (array operand, may carry [`G`])
    A(R),
    /// array element: array operand + `n` consecutive index registers
    E { a: R, i: R, n: u32, ty: Ty },
}

impl Place {
    pub fn ty(&self) -> Option<Ty> {
        match *self {
            Place::N(_, t) | Place::NG(_, t) | Place::E { ty: t, .. } => Some(t),
            Place::S(_) | Place::SG(_) => Some(Ty::Str),
            Place::A(_) => None,
        }
    }
}

/// One argument of a call, already converted to the parameter's type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Arg {
    /// copy numeric register
    N(R),
    /// copy string source operand (register or constant)
    S(R),
    /// move string out of a dead temporary
    SMove(R),
    /// BYREF: copied in at the call, copied back at return (strings/arrays are moved, not copied)
    Ref(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CallDesc {
    pub args: Vec<Arg>,
    /// FUNCTION result destination in the caller (register of the result type's bank)
    pub dest: Option<R>,
}

/// Parameter / result slot of a function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Slot {
    N(R),
    S(R),
    A(R),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Func {
    pub name: String,
    /// first instruction / one past the last
    pub entry: u32,
    pub end: u32,
    /// initial numeric registers (zeros + constants)
    pub num_init: Vec<u64>,
    pub n_str: u32,
    /// element type of each array register (for implicit `DIM a(10)`)
    pub arr_tys: Vec<Ty>,
    pub params: Vec<Slot>,
    pub ret: Option<(Slot, Ty)>,
}

impl Func {
    pub fn n_num(&self) -> u32 {
        self.num_init.len() as u32
    }
    pub fn n_arr(&self) -> u32 {
        self.arr_tys.len() as u32
    }
}

/// A loaded program: one flat code vector, function 0 is the main module.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Module {
    pub code: Vec<Ins>,
    pub funcs: Vec<Func>,
    pub nconst: Vec<u64>,
    pub kstr: Vec<Vec<u8>>,
    pub places: Vec<Place>,
    pub calls: Vec<CallDesc>,
    pub bargs: Vec<Vec<(R, Ty)>>,
    /// QIR `rt` builtins: name, result type (`None` = no result)
    pub rtfns: Vec<(&'static str, Option<Ty>)>,
    /// memory-backed variables (`NewV`)
    pub mvars: Vec<MVar>,
    /// statement starts: (pc, source line), sorted by pc
    pub stmts: Vec<(u32, u32)>,
    pub data: Vec<Vec<u8>>,
    /// DECLARE LIBRARY functions (QIR `meta extfn`) and their call sites
    pub xfns: Vec<XFn>,
    pub xcalls: Vec<XCall>,
    /// `$CONSOLE` (`Some(false)`) / `$CONSOLE:ONLY` (`Some(true)`)
    pub console: Option<bool>,
    /// `$SCREENHIDE`
    pub screenhide: bool,
    /// `SUB _GL` (QIR `meta glsub`): (function, call descriptor without arguments)
    pub gl_sub: Option<(u32, u32)>,
}

/// One `DECLARE LIBRARY` function: C name, library (`""` = the runner's own
/// symbols and the default system libraries), argument and result types.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XFn {
    pub name: String,
    pub lib: String,
    /// library `opengl` (`_gl*` statements): resolved through the GL context,
    /// callable only while SUB _GL runs
    pub gl: bool,
    pub args: Vec<XTy>,
    pub ret: XTy,
}

/// C-level type of an extern argument or result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum XTy {
    #[default]
    Void,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    /// address (BYREF, `_OFFSET`)
    Ptr,
    /// string: pointer to the bytes (argument) / NUL-terminated `char*` (result)
    Str,
}

/// An extern call: function, argument registers (numeric or string), result register.
#[derive(Clone, Debug, PartialEq)]
pub struct XCall {
    pub f: u32,
    pub args: Vec<R>,
    pub dest: Option<R>,
}

/// Operand roles, for validation and for tools (disassembler, JIT).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opnd {
    /// numeric register
    N(R),
    /// string destination register
    S(R),
    /// string source operand (may carry [`K`])
    SK(R),
    /// `n` consecutive numeric registers starting at `R`
    NRun(R, u32),
    /// array operand (may carry [`G`])
    A(R),
    /// module-level numeric (absolute)
    NG(R),
    SG(R),
    /// jump target inside the same function
    T(u32),
    /// jump target in the main module (handlers)
    TMain(u32),
    Place(u32),
    Call(u32),
    Func(u32),
    /// rt builtin + destination (bank from its result type, `NONE` = none)
    Rt(u32, R),
    BArgs(u32),
    NConst(u32),
    Data(u32),
}

impl Ins {
    /// Calls `f` with every operand of the instruction.
    pub fn operands(&self, f: &mut impl FnMut(Opnd)) {
        use Ins::*;
        use Opnd as O;
        match *self {
            Ldi { d, .. } => f(O::N(d)),
            Ldk { d, k } => {
                f(O::N(d));
                f(O::NConst(k))
            }
            Mov { d, s } | IToF { d, s } | IToFU { d, s } | Wrap { d, s, .. } | RoundF32 { d, s } | Un { d, s, .. } | CvtFI { d, s, .. } => {
                f(O::N(d));
                f(O::N(s))
            }
            GetG { d, g } => {
                f(O::N(d));
                f(O::NG(g))
            }
            SetG { g, s } => {
                f(O::NG(g));
                f(O::N(s))
            }
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
            | AddF { d, a, b }
            | SubF { d, a, b }
            | MulF { d, a, b }
            | PowF { d, a, b }
            | CmpI { d, a, b, .. }
            | CmpF { d, a, b, .. }
            | CmpU { d, a, b, .. }
            | Acc { d, a, b, .. }
            | Shl { d, a, b, .. }
            | Shr { d, a, b, .. }
            | DivFI { d, a, b } => {
                f(O::N(d));
                f(O::N(a));
                f(O::N(b))
            }
            CmpS { d, a, b, .. } => {
                f(O::N(d));
                f(O::SK(a));
                f(O::SK(b))
            }
            Jmp { t } | Loop { t } | Gosub { t } | ReturnL { t } => f(O::T(t)),
            JzI { s, t } | JnzI { s, t } => {
                f(O::N(s));
                f(O::T(t))
            }
            BrI { a, b, t, .. } | BrF { a, b, t, .. } | BrU { a, b, t, .. } => {
                f(O::N(a));
                f(O::N(b));
                f(O::T(t))
            }
            BrS { a, b, t, .. } => {
                f(O::SK(a));
                f(O::SK(b));
                f(O::T(t))
            }
            ForTestKI { v, lim, step, t, .. } | ForTestKF { v, lim, step, t, .. } => {
                f(O::N(v));
                f(O::N(lim));
                f(O::N(step));
                f(O::T(t))
            }
            LoopForI { c, iv, lim, body, .. } => {
                f(O::N(c));
                f(O::N(iv));
                f(O::NRun(lim, 2));
                f(O::T(body))
            }
            Return | Ret | End => {}
            System { s } => {
                if s != NONE {
                    f(O::N(s))
                }
            }
            Rt { f: b, d, args } => {
                f(O::Rt(b, d));
                f(O::BArgs(args))
            }
            Erase { a } => f(O::A(a)),
            NewV { d, .. } => f(O::N(d)),
            MovX { d, s } | XNeg { d, s } => {
                f(O::NRun(d, 2));
                f(O::NRun(s, 2))
            }
            XBin { d, a, b, .. } => {
                f(O::NRun(d, 2));
                f(O::NRun(a, 2));
                f(O::NRun(b, 2))
            }
            XCmp { d, a, b, .. } => {
                f(O::N(d));
                f(O::NRun(a, 2));
                f(O::NRun(b, 2))
            }
            XFromI { d, s, .. } | XFromF { d, s } => {
                f(O::NRun(d, 2));
                f(O::N(s))
            }
            XToF { d, s, .. } | XToI { d, s, .. } => {
                f(O::N(d));
                f(O::NRun(s, 2))
            }
            LdM { t, d, p, .. } => {
                f(if t == MTy::Str { O::S(d) } else if t == MTy::F80 { O::NRun(d, 2) } else { O::N(d) });
                f(O::N(p))
            }
            StM { t, p, s, .. } => {
                f(O::N(p));
                f(if t == MTy::Str { O::SK(s) } else if t == MTy::F80 { O::NRun(s, 2) } else { O::N(s) })
            }
            LdFs { d, p, .. } => {
                f(O::S(d));
                f(O::N(p))
            }
            StFs { p, s, .. } => {
                f(O::N(p));
                f(O::SK(s))
            }
            PAddM { d, p, o } => {
                f(O::N(d));
                f(O::N(p));
                f(O::N(o))
            }
            CopyM { d, s, .. } => {
                f(O::N(d));
                f(O::N(s))
            }
            DimM { a, dims, n, .. } => {
                f(O::N(a));
                f(O::NRun(dims, 2 * n))
            }
            AAddrM { d, a, i, n } => {
                f(O::N(d));
                f(O::N(a));
                f(O::NRun(i, n))
            }
            BoundM { d, a, dim, .. } => {
                f(O::N(d));
                f(O::N(a));
                f(O::N(dim))
            }
            EraseM { a } => f(O::N(a)),
            ReadN { d, .. } => f(O::N(d)),
            Call { f: func, desc } => {
                f(O::Func(func));
                f(O::Call(desc))
            }
            OnError { t } => {
                if t != NONE {
                    f(O::TMain(t))
                }
            }
            OnTimer { s, x, t } => {
                f(O::N(s));
                f(O::N(x));
                f(O::TMain(t))
            }
            XCall { .. } => {}
            Resume { k, t } => {
                if k == ResumeK::Label {
                    f(O::TMain(t))
                }
            }
            Raise { s } => f(O::N(s)),
            MovS { d, s } | TakeS { d, s } => {
                f(O::S(d));
                f(O::SK(s))
            }
            GetGS { d, g } => {
                f(O::S(d));
                f(O::SG(g))
            }
            SetGS { g, s } => {
                f(O::SG(g));
                f(O::SK(s))
            }
            Concat { d, a, b } => {
                f(O::S(d));
                f(O::SK(a));
                f(O::SK(b))
            }
            LenS { d, s } | Asc { d, s } => {
                f(O::N(d));
                f(O::SK(s))
            }
            Chr { d, s } => {
                f(O::S(d));
                f(O::N(s))
            }
            Left { d, s, n } | Right { d, s, n } => {
                f(O::S(d));
                f(O::SK(s));
                f(O::N(n))
            }
            Mid { d, s, start, n } => {
                f(O::S(d));
                f(O::SK(s));
                f(O::N(start));
                if n != NONE {
                    f(O::N(n))
                }
            }
            Dim { a, dims, n, .. } => {
                f(O::A(a));
                f(O::NRun(dims, 2 * n))
            }
            LdE { d, a, i, n } => {
                f(O::N(d));
                f(O::A(a));
                f(O::NRun(i, n))
            }
            StE { a, i, n, s } => {
                f(O::A(a));
                f(O::NRun(i, n));
                f(O::N(s))
            }
            LdES { d, a, i, n } => {
                f(O::S(d));
                f(O::A(a));
                f(O::NRun(i, n))
            }
            StES { a, i, n, s } => {
                f(O::A(a));
                f(O::NRun(i, n));
                f(O::SK(s))
            }
            LdE1 { d, a, i } => {
                f(O::N(d));
                f(O::A(a));
                f(O::N(i))
            }
            StE1 { a, i, s } => {
                f(O::A(a));
                f(O::N(i));
                f(O::N(s))
            }
            LdE1S { d, a, i } => {
                f(O::S(d));
                f(O::A(a));
                f(O::N(i))
            }
            StE1S { a, i, s } => {
                f(O::A(a));
                f(O::N(i));
                f(O::SK(s))
            }
            Bound { d, a, dim, .. } => {
                f(O::N(d));
                f(O::A(a));
                f(O::N(dim))
            }
            Restore { i } => f(O::Data(i)),
            ReadS { d } => f(O::S(d)),
        }
    }

    /// Mnemonic for the disassembler.
    pub fn mnemonic(&self) -> String {
        use Ins::*;
        match *self {
            Ldi { .. } => "ldi".into(),
            Ldk { .. } => "ldk".into(),
            Mov { .. } => "mov".into(),
            GetG { .. } => "getg".into(),
            SetG { .. } => "setg".into(),
            IToF { .. } => "itof".into(),
            IToFU { .. } => "itof.u64".into(),
            CmpU { c, .. } => format!("cmp.u {}", c.name()),
            BrU { c, .. } => format!("br.u {}", c.name()),
            Wrap { w, .. } => format!("wrap.{}", w.name()),
            RoundF32 { .. } => "round.f32".into(),
            CvtFI { w, trunc, .. } => format!("{}.{}", if trunc { "trunc" } else { "cvt" }, w.name()),
            Shl { w, .. } => format!("shl.{}", w.name()),
            Shr { w, .. } => format!("shr.{}", w.name()),
            DivFI { .. } => "div.ieee".into(),
            ReturnL { .. } => "return_l".into(),
            System { .. } => "system".into(),
            Rt { .. } => "rt".into(),
            Erase { .. } => "erase".into(),
            NewV { v, .. } => format!("newv mvar{v}"),
            MovX { .. } => "mov.x".into(),
            XBin { op, .. } => format!("{}.x", format!("{op:?}").to_lowercase()),
            XNeg { .. } => "neg.x".into(),
            XCmp { c, .. } => format!("cmp.x {}", c.name()),
            XFromI { u, .. } => if u { "x.from_u64" } else { "x.from_int" }.into(),
            XFromF { .. } => "x.from_f".into(),
            XToF { f32, .. } => if f32 { "x.to_f32" } else { "x.to_f64" }.into(),
            XToI { w, trunc, .. } => format!("x.to_{}{}", w.name(), if trunc { ".trunc" } else { "" }),
            LdM { t, off, .. } => format!("ld.{} +{off}", format!("{t:?}").to_lowercase()),
            StM { t, off, .. } => format!("st.{} +{off}", format!("{t:?}").to_lowercase()),
            LdFs { off, len, .. } => format!("ldfs +{off} {len}"),
            StFs { off, len, .. } => format!("stfs +{off} {len}"),
            PAddM { .. } => "padd".into(),
            CopyM { n, .. } => format!("copy {n}"),
            DimM { mode, .. } => format!("dim.m.{mode:?}").to_lowercase(),
            AAddrM { .. } => "aaddr".into(),
            BoundM { upper, .. } => if upper { "ubound.m" } else { "lbound.m" }.into(),
            EraseM { .. } => "erase.m".into(),
            ReadN { .. } => "read.n".into(),
            AddI16 { .. } => "add.i16".into(),
            AddI32 { .. } => "add.i32".into(),
            AddI64 { .. } => "add.i64".into(),
            SubI16 { .. } => "sub.i16".into(),
            SubI32 { .. } => "sub.i32".into(),
            SubI64 { .. } => "sub.i64".into(),
            MulI16 { .. } => "mul.i16".into(),
            MulI32 { .. } => "mul.i32".into(),
            MulI64 { .. } => "mul.i64".into(),
            IDiv { w, .. } => format!("idiv.{}", w.name()),
            Mod { w, .. } => format!("mod.{}", w.name()),
            And { .. } => "and".into(),
            Or { .. } => "or".into(),
            Xor { .. } => "xor".into(),
            Acc { op, wi, wo, .. } => format!("acc.{} {}.{}", wo.name(), format!("{op:?}").to_lowercase(), wi.name()),
            AddF { .. } => "add.f".into(),
            SubF { .. } => "sub.f".into(),
            MulF { .. } => "mul.f".into(),
            PowF { .. } => "pow.f".into(),
            Un { op, .. } => format!("un.{op:?}").to_lowercase(),
            CmpI { c, .. } => format!("cmp.i {}", c.name()),
            CmpF { c, .. } => format!("cmp.f {}", c.name()),
            CmpS { c, .. } => format!("cmp.s {}", c.name()),
            Jmp { .. } => "jmp".into(),
            Loop { .. } => "loop".into(),
            JzI { .. } => "jz.i".into(),
            JnzI { .. } => "jnz.i".into(),
            BrI { c, .. } => format!("br.i {}", c.name()),
            BrF { c, .. } => format!("br.f {}", c.name()),
            BrS { c, .. } => format!("br.s {}", c.name()),
            ForTestKI { skip, .. } => format!("for_test.i +{skip}"),
            ForTestKF { skip, .. } => format!("for_test.f +{skip}"),
            LoopForI { w, .. } => format!("loop_for.{}", w.name()),
            Gosub { .. } => "gosub".into(),
            Return => "return".into(),
            Call { .. } => "call".into(),
            Ret => "ret".into(),
            End => "end".into(),
            OnError { .. } => "on_error".into(),
            OnTimer { .. } => "on_timer".into(),
            XCall { .. } => "xcall".into(),
            Resume { k, .. } => format!("resume.{k:?}").to_lowercase(),
            Raise { .. } => "raise".into(),
            MovS { .. } => "mov.s".into(),
            TakeS { .. } => "take.s".into(),
            GetGS { .. } => "getg.s".into(),
            SetGS { .. } => "setg.s".into(),
            Concat { .. } => "concat".into(),
            LenS { .. } => "len".into(),
            Chr { .. } => "chr".into(),
            Asc { .. } => "asc".into(),
            Left { .. } => "left".into(),
            Right { .. } => "right".into(),
            Mid { .. } => "mid".into(),
            Dim { mode, .. } => format!("dim.{mode:?}").to_lowercase(),
            LdE { .. } => "lde".into(),
            StE { .. } => "ste".into(),
            LdES { .. } => "lde.s".into(),
            StES { .. } => "ste.s".into(),
            LdE1 { .. } => "lde1".into(),
            StE1 { .. } => "ste1".into(),
            LdE1S { .. } => "lde1.s".into(),
            StE1S { .. } => "ste1.s".into(),
            Bound { upper, .. } => if upper { "ubound" } else { "lbound" }.into(),
            Restore { .. } => "restore".into(),
            ReadS { .. } => "read.s".into(),
        }
    }

    /// Destination register written by a plain value-producing instruction
    /// (used by the loader to retarget a temporary into a variable).
    pub fn dest_mut(&mut self) -> Option<(bool, &mut R)> {
        use Ins::*;
        match self {
            Ldi { d, .. }
            | Ldk { d, .. }
            | Mov { d, .. }
            | GetG { d, .. }
            | IToF { d, .. }
            | IToFU { d, .. }
            | CmpU { d, .. }
            | Wrap { d, .. }
            | RoundF32 { d, .. }
            | CvtFI { d, .. }
            | Shl { d, .. }
            | Shr { d, .. }
            | DivFI { d, .. }
            | AddI16 { d, .. }
            | AddI32 { d, .. }
            | AddI64 { d, .. }
            | SubI16 { d, .. }
            | SubI32 { d, .. }
            | SubI64 { d, .. }
            | MulI16 { d, .. }
            | MulI32 { d, .. }
            | MulI64 { d, .. }
            | IDiv { d, .. }
            | Mod { d, .. }
            | And { d, .. }
            | Or { d, .. }
            | Xor { d, .. }
            | AddF { d, .. }
            | SubF { d, .. }
            | MulF { d, .. }
            | PowF { d, .. }
            | Un { d, .. }
            | CmpI { d, .. }
            | CmpF { d, .. }
            | CmpS { d, .. }
            | LenS { d, .. }
            | Asc { d, .. }
            | LdE { d, .. }
            | LdE1 { d, .. }
            | Bound { d, .. }


 => Some((false, d)),
            MovS { d, .. }
            | TakeS { d, .. }
            | GetGS { d, .. }
            | Concat { d, .. }
            | Chr { d, .. }
            | Left { d, .. }
            | Right { d, .. }
            | Mid { d, .. }
            | LdES { d, .. }
            | LdE1S { d, .. } => Some((true, d)),
            _ => None,
        }
    }

    /// Mutable jump target (for label fix-ups).
    pub fn target_mut(&mut self) -> Option<&mut u32> {
        use Ins::*;
        match self {
            Jmp { t }
            | Loop { t }
            | JzI { t, .. }
            | JnzI { t, .. }
            | BrI { t, .. }
            | BrF { t, .. }
            | BrS { t, .. }
            | BrU { t, .. }
            | ForTestKI { t, .. }
            | ForTestKF { t, .. }
            | Gosub { t }
            | ReturnL { t }
            | OnError { t }
            | OnTimer { t, .. }
            | Resume { t, .. }

 => Some(t),
            LoopForI { body, .. } => Some(body),
            _ => None,
        }
    }
}

fn fmt_opnd(o: Opnd) -> String {
    match o {
        Opnd::N(r) => format!("r{r}"),
        Opnd::S(r) => format!("s{r}"),
        Opnd::SK(r) if r & K != 0 => format!("k{}", r & !K),
        Opnd::SK(r) => format!("s{r}"),
        Opnd::NRun(r, n) => format!("r{r}..+{n}"),
        Opnd::A(a) if a & G != 0 => format!("g:a{}", a & !G),
        Opnd::A(a) => format!("a{a}"),
        Opnd::NG(g) => format!("g:r{g}"),
        Opnd::SG(g) => format!("g:s{g}"),
        Opnd::T(t) | Opnd::TMain(t) => format!("@{t}"),
        Opnd::Place(p) => format!("place{p}"),
        Opnd::Call(c) => format!("desc{c}"),
        Opnd::Func(f) => format!("f{f}"),
        Opnd::Rt(b, d) if d == NONE => format!("rt{b}"),
        Opnd::Rt(b, d) => format!("rt{b} -> {d}"),
        Opnd::BArgs(a) => format!("args{a}"),
        Opnd::NConst(k) => format!("#{k}"),
        Opnd::Data(i) => format!("data{i}"),
    }
}

impl Module {
    /// Human-readable listing (one instruction per line).
    pub fn disasm(&self) -> String {
        let mut s = String::new();
        for (fi, f) in self.funcs.iter().enumerate() {
            let _ = writeln!(s, "func {fi} {} [{}..{}) r={} s={} a={}", f.name, f.entry, f.end, f.n_num(), f.n_str, f.n_arr());
            for pc in f.entry..f.end {
                let ins = &self.code[pc as usize];
                let mut ops = Vec::new();
                ins.operands(&mut |o| ops.push(fmt_opnd(o)));
                let imm = match *ins {
                    Ins::Ldi { v, .. } => format!(", {v}"),
                    _ => String::new(),
                };
                let _ = writeln!(s, "{pc:6}  {} {}{imm}", ins.mnemonic(), ops.join(", "));
            }
        }
        s
    }

    /// Checks every operand against frame sizes and tables. The VM indexes
    /// registers without bounds checks, so this must pass before running.
    pub fn validate(&self) -> Result<(), String> {
        let n = self.code.len() as u32;
        let main = self.funcs.first().ok_or("no main function")?;
        if self.stmts.windows(2).any(|w| w[0].0 > w[1].0) {
            return Err("statement table not sorted".into());
        }
        for (fi, f) in self.funcs.iter().enumerate() {
            if f.entry >= f.end || f.end > n {
                return Err(format!("func {fi}: bad range"));
            }
            match self.code[f.end as usize - 1] {
                Ins::Ret | Ins::End => {}
                _ => return Err(format!("func {fi}: does not end with ret/end")),
            }
            for p in &f.params {
                check_slot(f, *p)?;
            }
            if let Some((s, _)) = f.ret {
                check_slot(f, s)?;
            }
            let mut bad: Option<String> = None;
            for pc in f.entry..f.end {
                let ins = &self.code[pc as usize];
                let mut chk = |o: Opnd| {
                    if bad.is_none() {
                        if let Err(e) = self.check(f, main, o) {
                            bad = Some(format!("pc {pc} ({}): {e}", ins.mnemonic()));
                        }
                    }
                };
                ins.operands(&mut chk);
                if let Ins::Call { f: callee, desc } = *ins {
                    let (Some(cf), Some(cd)) = (self.funcs.get(callee as usize), self.calls.get(desc as usize)) else {
                        return Err(format!("pc {pc}: bad call"));
                    };
                    if cd.args.len() != cf.params.len() || cd.dest.is_some() != cf.ret.is_some() {
                        return Err(format!("pc {pc}: call does not match {}", cf.name));
                    }
                    for (a, p) in cd.args.iter().zip(&cf.params) {
                        let ok = match (a, p) {
                            (Arg::N(r), Slot::N(_)) => *r < f.n_num(),
                            (Arg::S(r), Slot::S(_)) => self.sk_ok(f, *r),
                            (Arg::SMove(r), Slot::S(_)) => *r < f.n_str,
                            (Arg::Ref(pl), _) => (*pl as usize) < self.places.len(),
                            _ => false,
                        };
                        if !ok {
                            return Err(format!("pc {pc}: bad argument"));
                        }
                    }
                    if let (Some(d), Some((s, rty))) = (cd.dest, cf.ret) {
                        let ok = match s {
                            Slot::N(_) => d + u32::from(rty == Ty::F80) < f.n_num(),
                            Slot::S(_) => d < f.n_str,
                            Slot::A(_) => false,
                        };
                        if !ok {
                            return Err(format!("pc {pc}: bad result register"));
                        }
                    }
                }
                if let Ins::Rt { args, .. } = *ins {
                    for &(r, ty) in &self.bargs[args as usize] {
                        let ok = match ty {
                            Ty::Str => self.sk_ok(f, r),
                            Ty::F80 => r.checked_add(2).is_some_and(|e| e <= f.n_num()),
                            _ => r < f.n_num(),
                        };
                        if !ok {
                            return Err(format!("pc {pc}: bad builtin argument"));
                        }
                    }
                }
            }
            if let Some(e) = bad {
                return Err(format!("func {fi} {}: {e}", f.name));
            }
            // places used by this function's calls must be valid in this frame
            for pc in f.entry..f.end {
                if let Ins::Call { desc, .. } = self.code[pc as usize] {
                    for a in &self.calls[desc as usize].args {
                        if let Arg::Ref(pl) = a {
                            self.check_place(f, main, &self.places[*pl as usize])?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn sk_ok(&self, f: &Func, r: R) -> bool {
        if r & K != 0 {
            ((r & !K) as usize) < self.kstr.len()
        } else {
            r < f.n_str
        }
    }

    fn check_place(&self, f: &Func, main: &Func, p: &Place) -> Result<(), String> {
        let ok = match *p {
            Place::N(r, _) => r < f.n_num(),
            Place::NG(g, _) => g < main.n_num(),
            Place::S(r) => r < f.n_str,
            Place::SG(g) => g < main.n_str,
            Place::A(a) => arr_ok(f, main, a),
            Place::E { a, i, n, .. } => arr_ok(f, main, a) && n >= 1 && i.checked_add(n).is_some_and(|e| e <= f.n_num()),
        };
        if ok {
            Ok(())
        } else {
            Err(format!("bad place {p:?}"))
        }
    }

    fn check(&self, f: &Func, main: &Func, o: Opnd) -> Result<(), String> {
        let ok = match o {
            Opnd::N(r) => r < f.n_num(),
            Opnd::S(r) => r < f.n_str,
            Opnd::SK(r) => self.sk_ok(f, r),
            Opnd::NRun(r, n) => r.checked_add(n).is_some_and(|e| e <= f.n_num()),
            Opnd::A(a) => arr_ok(f, main, a),
            Opnd::NG(g) => g < main.n_num(),
            Opnd::SG(g) => g < main.n_str,
            Opnd::T(t) => t >= f.entry && t < f.end,
            Opnd::TMain(t) => t >= main.entry && t < main.end,
            Opnd::Place(p) => (p as usize) < self.places.len(),
            Opnd::Call(c) => (c as usize) < self.calls.len(),
            Opnd::Func(x) => (x as usize) < self.funcs.len(),
            Opnd::Rt(b, d) => match self.rtfns.get(b as usize) {
                None => false,
                Some((_, None)) => d == NONE,
                Some((_, Some(Ty::Str))) => d < f.n_str,
                Some((_, Some(Ty::F80))) => d.checked_add(2).is_some_and(|e| e <= f.n_num()),
                Some((_, Some(_))) => d < f.n_num(),
            },
            Opnd::BArgs(a) => (a as usize) < self.bargs.len(),
            Opnd::NConst(k) => (k as usize) < self.nconst.len(),
            Opnd::Data(i) => (i as usize) <= self.data.len(),
        };
        if ok {
            Ok(())
        } else {
            Err(format!("operand out of range: {o:?}"))
        }
    }
}

fn arr_ok(f: &Func, main: &Func, a: R) -> bool {
    if a & G != 0 {
        (a & !G) < main.n_arr()
    } else {
        a < f.n_arr()
    }
}

fn check_slot(f: &Func, s: Slot) -> Result<(), String> {
    let ok = match s {
        Slot::N(r) => r < f.n_num(),
        Slot::S(r) => r < f.n_str,
        Slot::A(r) => r < f.n_arr(),
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{}: bad parameter slot", f.name))
    }
}

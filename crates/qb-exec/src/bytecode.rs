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
}

impl Ty {
    pub fn is_int(self) -> bool {
        matches!(self, Ty::I16 | Ty::I32 | Ty::I64)
    }
    pub fn is_float(self) -> bool {
        matches!(self, Ty::F32 | Ty::F64)
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
        }
    }
}

/// Integer width of a wrapping/range-checked integer operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W {
    W16,
    W32,
    W64,
}

impl W {
    #[inline(always)]
    pub fn wrap(self, v: i64) -> i64 {
        match self {
            W::W16 => v as i16 as i64,
            W::W32 => v as i32 as i64,
            W::W64 => v,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            W::W16 => "i16",
            W::W32 => "i32",
            W::W64 => "i64",
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
    AbsI(W),
    AbsF,
    /// SGN -> INTEGER
    SgnI,
    SgnF,
    /// INT / FIX of a float
    Int,
    Fix,
    Sqr,
    Log,
    Exp,
    Sin,
    Cos,
    Tan,
    Atn,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvMode {
    On,
    Off,
    Stop,
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
    /// float -> integer with QBasic rounding (half to even); out of range = error 6
    FToI { d: R, s: R, w: W },
    /// integer -> narrower integer (C cast, wraps)
    Wrap { d: R, s: R, w: W },
    /// f64 -> f32 -> f64 (store into SINGLE)
    RoundF32 { d: R, s: R },
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
    Eqv { d: R, a: R, b: R },
    Imp { d: R, a: R, b: R },
    /// `d = wo(d + wi(a op b))` — superinstruction for `s = s + a op b`
    /// (reads `d`; never retargeted)
    Acc { op: AccOp, d: R, a: R, b: R, wi: W, wo: W },
    // ---- float arithmetic
    AddF { d: R, a: R, b: R },
    SubF { d: R, a: R, b: R },
    MulF { d: R, a: R, b: R },
    DivF { d: R, a: R, b: R },
    PowF { d: R, a: R, b: R },
    Un { op: Un, d: R, s: R },
    // ---- comparisons: d = -1 / 0 (INTEGER)
    CmpI { c: Cmp, d: R, a: R, b: R },
    CmpF { c: Cmp, d: R, a: R, b: R },
    CmpS { c: Cmp, d: R, a: R, b: R },
    // ---- control flow
    Jmp { t: u32 },
    /// backward jump: event safe point
    Loop { t: u32 },
    JzI { s: R, t: u32 },
    JnzI { s: R, t: u32 },
    JzF { s: R, t: u32 },
    JnzF { s: R, t: u32 },
    /// jump when `a c b` holds (compare fused with JZ/JNZ)
    BrI { c: Cmp, a: R, b: R, t: u32 },
    BrF { c: Cmp, a: R, b: R, t: u32 },
    BrS { c: Cmp, a: R, b: R, t: u32 },
    /// FOR test: jump to `t` (loop exit) when `v` is past `lim` in the direction of `step`
    ForTestI { v: R, lim: R, step: R, t: u32 },
    ForTestF { v: R, lim: R, step: R, t: u32 },
    /// NEXT + FOR test: `v += step` (wrapping `w`); jump to `body` while in range (safe point)
    ForNextI { v: R, step: R, lim: R, body: u32, w: W },
    Gosub { t: u32 },
    Return,
    /// call function `f` with argument descriptor `Module::calls[desc]` (safe point)
    Call { f: u32, desc: u32 },
    Ret,
    End,
    OnError { t: u32 },
    Resume { k: ResumeK, t: u32 },
    /// raise the error whose code is in register `s`
    Raise { s: R },
    /// raise a constant error code (type mismatch found by the loader, ...)
    RaiseK { code: u32 },
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
    /// generic builtin `Module::bfns[f]` with args `Module::bargs[args]`
    Bi { f: u32, d: R, args: u32 },
    PrintS { s: R },
    PrintN { s: R, ty: Ty },
    PrintZone,
    PrintNl,
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
    /// SWAP two places of `Module::places`
    Swap { p: u32, q: u32 },
    // ---- misc runtime
    /// READ: next DATA item as string (`str`) into s-register, else its VAL into r-register (f64)
    Read { d: R, str: bool },
    Restore { i: u32 },
    Randomize { s: R, using: bool },
    /// RND [(s)] -> SINGLE; `s == NONE` = no argument
    Rnd { d: R, s: R },
    Err { d: R },
    Erl { d: R },
    // ---- events (ON TIMER / ON KEY / ...), delivered at safe points
    OnEvent { id: u32, t: u32 },
    EventCtl { id: u32, mode: EvMode },
    /// ON TIMER(interval in f64 register `s`) GOSUB t  (event id 0)
    OnTimer { s: R, t: u32 },
}

/// Where a BYREF argument or SWAP operand lives.
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
    pub is_static: bool,
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
    /// generic builtins: name, result type
    pub bfns: Vec<(String, Ty)>,
    pub bargs: Vec<Vec<(R, Ty)>>,
    /// statement starts: (pc, source line), sorted by pc
    pub stmts: Vec<(u32, u32)>,
    pub data: Vec<Vec<u8>>,
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
    Bi(u32),
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
            Mov { d, s } | IToF { d, s } | FToI { d, s, .. } | Wrap { d, s, .. } | RoundF32 { d, s } | Un { d, s, .. } => {
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
            | Eqv { d, a, b }
            | Imp { d, a, b }
            | AddF { d, a, b }
            | SubF { d, a, b }
            | MulF { d, a, b }
            | DivF { d, a, b }
            | PowF { d, a, b }
            | CmpI { d, a, b, .. }
            | CmpF { d, a, b, .. }
            | Acc { d, a, b, .. } => {
                f(O::N(d));
                f(O::N(a));
                f(O::N(b))
            }
            CmpS { d, a, b, .. } => {
                f(O::N(d));
                f(O::SK(a));
                f(O::SK(b))
            }
            Jmp { t } | Loop { t } | Gosub { t } => f(O::T(t)),
            JzI { s, t } | JnzI { s, t } | JzF { s, t } | JnzF { s, t } => {
                f(O::N(s));
                f(O::T(t))
            }
            BrI { a, b, t, .. } | BrF { a, b, t, .. } => {
                f(O::N(a));
                f(O::N(b));
                f(O::T(t))
            }
            BrS { a, b, t, .. } => {
                f(O::SK(a));
                f(O::SK(b));
                f(O::T(t))
            }
            ForTestI { v, lim, step, t } | ForTestF { v, lim, step, t } => {
                f(O::N(v));
                f(O::N(lim));
                f(O::N(step));
                f(O::T(t))
            }
            ForNextI { v, step, lim, body, .. } => {
                f(O::N(v));
                f(O::N(step));
                f(O::N(lim));
                f(O::T(body))
            }
            Return | Ret | End | PrintZone | PrintNl => {}
            Call { f: func, desc } => {
                f(O::Func(func));
                f(O::Call(desc))
            }
            OnError { t } => {
                if t != NONE {
                    f(O::TMain(t))
                }
            }
            Resume { k, t } => {
                if k == ResumeK::Label {
                    f(O::TMain(t))
                }
            }
            Raise { s } => f(O::N(s)),
            RaiseK { .. } => {}
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
            Bi { f: b, args, .. } => {
                f(O::Bi(b));
                f(O::BArgs(args))
            }
            PrintS { s } => f(O::SK(s)),
            PrintN { s, .. } => f(O::N(s)),
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
            Swap { p, q } => {
                f(O::Place(p));
                f(O::Place(q))
            }
            Read { d, str } => f(if str { O::S(d) } else { O::N(d) }),
            Restore { i } => f(O::Data(i)),
            Randomize { s, .. } => f(O::N(s)),
            Rnd { d, s } => {
                f(O::N(d));
                if s != NONE {
                    f(O::N(s))
                }
            }
            Err { d } | Erl { d } => f(O::N(d)),
            OnEvent { t, .. } => f(O::TMain(t)),
            EventCtl { .. } => {}
            OnTimer { s, t } => {
                f(O::N(s));
                f(O::TMain(t))
            }
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
            FToI { w, .. } => format!("ftoi.{}", w.name()),
            Wrap { w, .. } => format!("wrap.{}", w.name()),
            RoundF32 { .. } => "round.f32".into(),
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
            Eqv { .. } => "eqv".into(),
            Imp { .. } => "imp".into(),
            Acc { op, wi, wo, .. } => format!("acc.{} {}.{}", wo.name(), format!("{op:?}").to_lowercase(), wi.name()),
            AddF { .. } => "add.f".into(),
            SubF { .. } => "sub.f".into(),
            MulF { .. } => "mul.f".into(),
            DivF { .. } => "div.f".into(),
            PowF { .. } => "pow.f".into(),
            Un { op, .. } => format!("un.{op:?}").to_lowercase(),
            CmpI { c, .. } => format!("cmp.i {}", c.name()),
            CmpF { c, .. } => format!("cmp.f {}", c.name()),
            CmpS { c, .. } => format!("cmp.s {}", c.name()),
            Jmp { .. } => "jmp".into(),
            Loop { .. } => "loop".into(),
            JzI { .. } => "jz.i".into(),
            JnzI { .. } => "jnz.i".into(),
            JzF { .. } => "jz.f".into(),
            JnzF { .. } => "jnz.f".into(),
            BrI { c, .. } => format!("br.i {}", c.name()),
            BrF { c, .. } => format!("br.f {}", c.name()),
            BrS { c, .. } => format!("br.s {}", c.name()),
            ForTestI { .. } => "for_test.i".into(),
            ForTestF { .. } => "for_test.f".into(),
            ForNextI { w, .. } => format!("for_next.{}", w.name()),
            Gosub { .. } => "gosub".into(),
            Return => "return".into(),
            Call { .. } => "call".into(),
            Ret => "ret".into(),
            End => "end".into(),
            OnError { .. } => "on_error".into(),
            Resume { k, .. } => format!("resume.{k:?}").to_lowercase(),
            Raise { .. } => "raise".into(),
            RaiseK { code } => format!("raise {code}"),
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
            Bi { .. } => "bi".into(),
            PrintS { .. } => "print.s".into(),
            PrintN { ty, .. } => format!("print.{}", ty.name()),
            PrintZone => "print_zone".into(),
            PrintNl => "print_nl".into(),
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
            Swap { .. } => "swap".into(),
            Read { .. } => "read".into(),
            Restore { .. } => "restore".into(),
            Randomize { using, .. } => if using { "randomize_using" } else { "randomize" }.into(),
            Rnd { .. } => "rnd".into(),
            Err { .. } => "err".into(),
            Erl { .. } => "erl".into(),
            OnEvent { id, .. } => format!("on_event {id}"),
            EventCtl { id, mode } => format!("event.{mode:?} {id}").to_lowercase(),
            OnTimer { .. } => "on_timer".into(),
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
            | FToI { d, .. }
            | Wrap { d, .. }
            | RoundF32 { d, .. }
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
            | Eqv { d, .. }
            | Imp { d, .. }
            | AddF { d, .. }
            | SubF { d, .. }
            | MulF { d, .. }
            | DivF { d, .. }
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
            | Rnd { d, .. }
            | Err { d, .. }
            | Erl { d, .. } => Some((false, d)),
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
            | JzF { t, .. }
            | JnzF { t, .. }
            | BrI { t, .. }
            | BrF { t, .. }
            | BrS { t, .. }
            | ForTestI { t, .. }
            | ForTestF { t, .. }
            | Gosub { t }
            | OnError { t }
            | Resume { t, .. }
            | OnEvent { t, .. }
            | OnTimer { t, .. } => Some(t),
            ForNextI { body, .. } => Some(body),
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
        Opnd::Bi(b) => format!("bi{b}"),
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
                    if let (Some(d), Some((s, _))) = (cd.dest, cf.ret) {
                        let ok = match s {
                            Slot::N(_) => d < f.n_num(),
                            Slot::S(_) => d < f.n_str,
                            Slot::A(_) => false,
                        };
                        if !ok {
                            return Err(format!("pc {pc}: bad result register"));
                        }
                    }
                }
                if let Ins::Swap { p, q } = *ins {
                    for x in [p, q] {
                        self.check_place(f, main, &self.places[x as usize])?;
                    }
                }
                if let Ins::Bi { args, .. } = *ins {
                    for &(r, ty) in &self.bargs[args as usize] {
                        let ok = if ty == Ty::Str { self.sk_ok(f, r) } else { r < f.n_num() };
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
            Opnd::Bi(b) => (b as usize) < self.bfns.len(),
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

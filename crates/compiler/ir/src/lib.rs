//! `qb-ir` — QIR, the typed register IR emitted by the QBNex compiler.
//!
//! The normative description is `docs/dev/qir.md`. This crate holds
//! the in-memory form ([`Module`]), the text parser ([`parse`]) and printer
//! ([`print`]), the validator ([`validate`]) and the builtin signature table
//! ([`builtins`]) that every QIR executor (VM, JIT) implements.

pub mod builtins;
mod parse;
mod print;
mod validate;

pub use parse::parse;
pub use print::print;
pub use validate::validate;

/// Current format version (`QIR <n>` header).
pub const VERSION: u32 = 1;

/// Register (and scalar storage) type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
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
    F80,
    /// Variable-length string (a handle; the executor owns the bytes).
    Str,
    /// Address of storage (variable, array element, UDT, array descriptor).
    Ptr,
}

impl Ty {
    pub const ALL: [Ty; 13] = [
        Ty::I8,
        Ty::U8,
        Ty::I16,
        Ty::U16,
        Ty::I32,
        Ty::U32,
        Ty::I64,
        Ty::U64,
        Ty::F32,
        Ty::F64,
        Ty::F80,
        Ty::Str,
        Ty::Ptr,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Ty::I8 => "i8",
            Ty::U8 => "u8",
            Ty::I16 => "i16",
            Ty::U16 => "u16",
            Ty::I32 => "i32",
            Ty::U32 => "u32",
            Ty::I64 => "i64",
            Ty::U64 => "u64",
            Ty::F32 => "f32",
            Ty::F64 => "f64",
            Ty::F80 => "f80",
            Ty::Str => "str",
            Ty::Ptr => "ptr",
        }
    }
    pub fn from_name(s: &str) -> Option<Ty> {
        Ty::ALL.iter().copied().find(|t| t.name() == s)
    }
    pub fn is_int(self) -> bool {
        matches!(
            self,
            Ty::I8 | Ty::U8 | Ty::I16 | Ty::U16 | Ty::I32 | Ty::U32 | Ty::I64 | Ty::U64
        )
    }
    pub fn is_float(self) -> bool {
        matches!(self, Ty::F32 | Ty::F64 | Ty::F80)
    }
    pub fn is_num(self) -> bool {
        self.is_int() || self.is_float()
    }
    pub fn is_signed(self) -> bool {
        matches!(self, Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64) || self.is_float()
    }
    /// Size in bytes when stored in memory (QB64 layout; `f80` uses 32 bytes like `_FLOAT`).
    pub fn size(self) -> u32 {
        match self {
            Ty::I8 | Ty::U8 => 1,
            Ty::I16 | Ty::U16 => 2,
            Ty::I32 | Ty::U32 | Ty::F32 => 4,
            Ty::I64 | Ty::U64 | Ty::F64 | Ty::Ptr | Ty::Str => 8,
            Ty::F80 => 32,
        }
    }
}

/// Storage type of a variable, field or array element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreTy {
    Scalar(Ty),
    /// `STRING * n`: n bytes, space padded.
    FStr(u32),
    /// A user-defined type, by name.
    Udt(String),
}

impl StoreTy {
    /// Register type used to `load`/`store` the whole value, if it has one.
    pub fn reg_ty(&self) -> Option<Ty> {
        match self {
            StoreTy::Scalar(t) => Some(*t),
            StoreTy::FStr(_) => Some(Ty::Str),
            StoreTy::Udt(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Global,
    Local,
    Static,
    /// Parameter; `byref` params receive a `ptr` argument.
    Param { byref: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarDecl {
    /// Includes the leading `$`.
    pub name: String,
    pub kind: VarKind,
    pub ty: StoreTy,
    /// Array of `ty` (descriptor; dimensioned with `dim`/`redim`).
    pub array: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: StoreTy,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdtDef {
    pub name: String,
    pub size: u32,
    pub fields: Vec<Field>,
}

/// A register number (`%n`).
pub type Reg = u32;

/// Constant operand of `const.T`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lit {
    Int(i128),
    /// Decimal text, `inf`/`-inf`/`nan`, or exact bits `0x...` (see [`float_value`]).
    Float(String),
    Str(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    /// Floating division (float types only).
    Div,
    /// Integer division, truncating; error 11 on zero (int types only).
    IDiv,
    /// Integer remainder, sign of dividend; error 11 on zero (int types only).
    Mod,
    /// Power (float types only).
    Pow,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimKind {
    Dim,
    /// `dims`: DIM of a static array (constant bounds; QBasic allocates it once):
    /// dimensions it unless it already has data, and ERASE only clears it
    Static,
    Redim,
    RedimPreserve,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeKind {
    Same,
    Next,
    Label(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inst {
    /// Statement boundary: source line for errors, RESUME target, event poll point.
    Line(u32),
    Label(String),
    Const { ty: Ty, dst: Reg, val: Lit },
    Mov { ty: Ty, dst: Reg, src: Reg },
    Bin { op: BinOp, ty: Ty, dst: Reg, a: Reg, b: Reg },
    Un { op: UnOp, ty: Ty, dst: Reg, a: Reg },
    /// Result is always `i32` (-1 true, 0 false).
    Cmp { op: CmpOp, ty: Ty, dst: Reg, a: Reg, b: Reg },
    /// Numeric conversion; float->int rounds half to even (QBasic `CINT`).
    Cvt { to: Ty, from: Ty, dst: Reg, src: Reg },
    /// Numeric conversion; float->int truncates toward zero (C cast).
    Trunc { to: Ty, from: Ty, dst: Reg, src: Reg },
    Concat { dst: Reg, a: Reg, b: Reg },
    Load { ty: Ty, dst: Reg, var: String },
    Store { ty: Ty, var: String, src: Reg },
    /// Address of a (non-array) variable's storage.
    Addr { dst: Reg, var: String },
    /// Address of an array descriptor (to pass an array to a `param ... array`).
    ARef { dst: Reg, var: String },
    /// Address of an array element; error 9 when out of bounds or not dimensioned.
    AAddr { dst: Reg, var: String, idx: Vec<Reg> },
    Ld { ty: Ty, dst: Reg, ptr: Reg, off: u32 },
    St { ty: Ty, ptr: Reg, off: u32, src: Reg },
    /// Load a fixed-length string field of `len` bytes as `str`.
    LdFs { dst: Reg, ptr: Reg, off: u32, len: u32 },
    /// Store `str` into a fixed-length field: truncate or pad with spaces.
    StFs { ptr: Reg, off: u32, len: u32, src: Reg },
    /// `ptr + int` (byte offset).
    PAdd { dst: Reg, ptr: Reg, off: Reg },
    /// Copy `size` bytes (UDT assignment).
    Copy { dst: Reg, src: Reg, size: u32 },
    /// Dimension an array; `bounds` = (lower, upper) per dimension.
    Dim { kind: DimKind, var: String, bounds: Vec<(Reg, Reg)> },
    Erase { var: String },
    /// `dim` is a 1-based dimension number register; result `i32`.
    LBound { dst: Reg, var: String, dim: Reg },
    UBound { dst: Reg, var: String, dim: Reg },
    Jmp(String),
    Jz { c: Reg, target: String },
    Jnz { c: Reg, target: String },
    Gosub(String),
    /// `RETURN`.
    GRet,
    /// `RETURN label`.
    GRetL(String),
    Call { dst: Option<Reg>, func: String, args: Vec<Reg> },
    Ret(Option<Reg>),
    /// Runtime builtin call (see [`builtins::TABLE`]).
    Rt { dst: Option<Reg>, name: String, args: Vec<Reg> },
    OnError(Option<String>),
    /// `ON TIMER([slot,] secs) GOSUB target`: `slot` i32 (0 = plain ON TIMER), `secs` f64;
    /// the handler runs as a GOSUB at module level when the timer fires.
    OnTimer { slot: Reg, secs: Reg, target: String },
    /// `ON KEY(key) GOSUB target`: `key` i32 1..31; the handler runs as a GOSUB at
    /// module level when the key is pressed while trapping is on (`KEY(n) ON`).
    OnKey { key: Reg, target: String },
    /// `ON PLAY(n) GOSUB target`: `n` i32; the handler runs as a GOSUB at module level when the
    /// background music queue drops from `n` notes to fewer while trapping is on (`PLAY ON`).
    OnPlay { n: Reg, target: String },
    Resume(ResumeKind),
    Error(Reg),
    Read { ty: Ty, dst: Reg },
    Restore(Option<String>),
    End,
    Stop,
    System(Option<Reg>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Func {
    pub name: String,
    pub ret: Option<Ty>,
    /// In argument order.
    pub params: Vec<VarDecl>,
    /// `local` and `static` declarations.
    pub locals: Vec<VarDecl>,
    pub body: Vec<Inst>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Module {
    pub meta: Vec<(String, String)>,
    pub types: Vec<UdtDef>,
    pub globals: Vec<VarDecl>,
    /// DATA items in program order (raw text of each item, unquoted).
    pub data: Vec<Vec<u8>>,
    /// `RESTORE` targets: label -> index into `data`.
    pub data_labels: Vec<(String, usize)>,
    pub funcs: Vec<Func>,
}

impl Module {
    pub fn func(&self, name: &str) -> Option<&Func> {
        self.funcs.iter().find(|f| f.name == name)
    }
    pub fn udt(&self, name: &str) -> Option<&UdtDef> {
        self.types.iter().find(|t| t.name == name)
    }
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.meta.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

/// Parse / validation error with the 1-based source line of the QIR text (0 = n/a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrError {
    pub line: u32,
    pub msg: String,
}

impl std::fmt::Display for IrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "QIR line {}: {}", self.line, self.msg)
        } else {
            write!(f, "QIR: {}", self.msg)
        }
    }
}

impl std::error::Error for IrError {}

/// Value of a `Lit::Float` text as `f64` (`f80` literals are rounded).
pub fn float_value(text: &str) -> Option<f64> {
    match text {
        "inf" => return Some(f64::INFINITY),
        "-inf" => return Some(f64::NEG_INFINITY),
        "nan" => return Some(f64::NAN),
        _ => {}
    }
    if let Some(h) = text.strip_prefix("0x") {
        return match h.len() {
            8 => u32::from_str_radix(h, 16).ok().map(|b| f64::from(f32::from_bits(b))),
            16 => u64::from_str_radix(h, 16).ok().map(f64::from_bits),
            20 => f80_bits_to_f64(h),
            _ => None,
        };
    }
    text.parse::<f64>().ok()
}

/// Exact bits of an f80 literal written as 20 hex digits (sign+exponent 4, mantissa 16).
pub fn f80_bits(text: &str) -> Option<(u16, u64)> {
    let h = text.strip_prefix("0x")?;
    // Hex digits only: also guarantees byte 4 is a char boundary (no panic on
    // non-ASCII input of 20 bytes).
    if h.len() != 20 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some((u16::from_str_radix(&h[..4], 16).ok()?, u64::from_str_radix(&h[4..], 16).ok()?))
}

fn f80_bits_to_f64(h: &str) -> Option<f64> {
    let (se, m) = f80_bits(&format!("0x{h}"))?;
    let sign = if se & 0x8000 != 0 { -1.0 } else { 1.0 };
    let e = i32::from(se & 0x7FFF);
    if e == 0x7FFF {
        return Some(if m << 1 == 0 { sign * f64::INFINITY } else { f64::NAN });
    }
    if m == 0 {
        return Some(sign * 0.0);
    }
    Some(sign * (m as f64) * 2f64.powi(e - 16383 - 63))
}

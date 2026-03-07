use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum QError {
    #[error("Syntax error: {0}")]
    Syntax(String),

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Illegal function call: {0}")]
    IllegalFunctionCall(String),

    #[error("Subscript out of range")]
    SubscriptOutOfRange,

    #[error("Index out of range: {0}")]
    IndexOutOfRange(String),

    #[error("Overflow: {0}")]
    Overflow(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("File I/O error: {0}")]
    FileIO(String),

    #[error("Bad file name or number")]
    BadFileNameOrNumber,

    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Disk full")]
    DiskFull,

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Label not found: {0}")]
    LabelNotFound(String),

    #[error("Duplicate label: {0}")]
    DuplicateLabel(String),

    #[error("GOSUB without RETURN")]
    GosubWithoutReturn,

    #[error("RETURN without GOSUB")]
    ReturnWithoutGosub,

    #[error("NEXT without FOR")]
    NextWithoutFor,

    #[error("FOR without NEXT")]
    ForWithoutNext,

    #[error("END without IF")]
    EndWithoutIf,

    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),

    #[error("Duplicate definition: {0}")]
    DuplicateDefinition(String),

    #[error("Invalid qualifier: {0}")]
    InvalidQualifier(String),

    #[error("Invalid procedure: {0}")]
    InvalidProcedure(String),

    #[error("Input past end of file")]
    InputPastEndOfFile,

    #[error("Device timeout")]
    DeviceTimeout,

    #[error("Device fault")]
    DeviceFault,

    #[error("Already in use")]
    AlreadyInUse,

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Out of data")]
    OutOfData,

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Runtime error: {0}")]
    Runtime(String),
}

pub type QResult<T> = Result<T, QError>;

impl QError {
    pub fn code(&self) -> i16 {
        match self {
            QError::Syntax(_) => 1,
            QError::TypeMismatch(_) => 2,
            QError::DivisionByZero => 3,
            QError::IllegalFunctionCall(_) => 4,
            QError::SubscriptOutOfRange => 5,
            QError::IndexOutOfRange(_) => 6,
            QError::Overflow(_) => 7,
            QError::FileNotFound(_) => 8,
            QError::FileIO(_) => 9,
            QError::BadFileNameOrNumber => 10,
            QError::PathNotFound(_) => 11,
            QError::DiskFull => 12,
            QError::OutOfMemory => 13,
            QError::LabelNotFound(_) => 14,
            QError::DuplicateLabel(_) => 15,
            QError::GosubWithoutReturn => 16,
            QError::ReturnWithoutGosub => 17,
            QError::NextWithoutFor => 18,
            QError::ForWithoutNext => 19,
            QError::EndWithoutIf => 20,
            QError::UndefinedVariable(_) => 21,
            QError::DuplicateDefinition(_) => 22,
            QError::InvalidQualifier(_) => 23,
            QError::InvalidProcedure(_) => 24,
            QError::InputPastEndOfFile => 25,
            QError::DeviceTimeout => 26,
            QError::DeviceFault => 27,
            QError::AlreadyInUse => 28,
            QError::PermissionDenied => 29,
            QError::OutOfData => 30,
            QError::UnsupportedFeature(_) => 31,
            QError::Internal(_) => 99,
            QError::Runtime(_) => 255, // User-defined error
        }
    }
}

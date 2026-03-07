use core_types::QType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByRefTarget {
    None,
    Global(usize),
    ArrayElement {
        name: String,
        index_slots: Vec<usize>,
    },
}

#[derive(Debug, Clone)]
pub enum OpCode {
    NoOp,

    LoadConstant(QType),
    LoadVariable(String),
    StoreVariable(String),
    LoadFast(usize),
    StoreFast(usize),
    SetStringWidth {
        slot: usize,
        width: usize,
    },
    SetStringArrayWidth {
        name: String,
        width: usize,
    },
    InitGlobals(usize),

    ReadFast(usize),
    SwapFast(usize, usize),

    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
    Modulo,
    Power,

    Negate,
    Not,

    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessOrEqual,
    GreaterOrEqual,

    And,
    Or,
    Xor,
    Eqv,
    Imp,

    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),

    Gosub(usize),
    Return,

    ForInit {
        var_name: String,
        end_label: usize,
        step: f64,
    },
    ForInitFast {
        var_index: usize,
        end_label: usize,
        step: f64,
    },
    ForStep {
        var_name: String,
        step: f64,
    },
    ForStepFast {
        var_index: usize,
        step: f64,
    },
    Next(String),
    NextFast(usize),

    Print,
    PrintNewline,
    PrintTab,
    PrintSpace,
    PrintUsing(usize),

    Input,

    Screen(i32),
    Pset {
        x: i32,
        y: i32,
        color: i32,
    },
    Preset {
        x: i32,
        y: i32,
        color: i32,
    },
    Line {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: i32,
    },
    Circle {
        x: i32,
        y: i32,
        radius: i32,
        color: i32,
    },

    Sound {
        frequency: i32,
        duration: i32,
    },
    Play(String),
    Beep,

    Open {
        mode: String,
    },
    Close,
    Get,
    Put,

    End,
    Stop,

    Call(String),
    CallNative(String),

    Dup,
    Pop,

    ArrayIndex(String),
    ArrayStore(String, usize),
    ArrayLoad(String, usize),
    ArrayDim {
        name: String,
        dimensions: Vec<(i32, i32)>,
    },
    ArrayRedim {
        name: String,
        dimensions: Vec<(i32, i32)>,
        preserve: bool,
    },

    // Function/Sub definitions
    DefineFunction {
        name: String,
        params: Vec<usize>,
        result: usize,
        body_start: usize,
        body_end: usize,
    },
    DefineSub {
        name: String,
        params: Vec<usize>,
        body_start: usize,
        body_end: usize,
    },
    CallFunction {
        name: String,
        by_ref: Vec<ByRefTarget>,
    },
    CallSub {
        name: String,
        by_ref: Vec<ByRefTarget>,
    },
    FunctionReturn,
    SubReturn,

    // SELECT CASE
    SelectCase,
    CaseValue(QType),
    CaseRange(QType, QType),
    CaseIs {
        op: String,
        value: QType,
    },
    CaseElse,
    EndSelect,

    ErrorHandler(usize),
    Resume,
    ResumeNext,
    OnError(usize),
    OnErrorResumeNext,
    OnTimer {
        interval_secs: f64,
        handler: usize,
    },
    TimerOn,
    TimerOff,
    TimerStop,

    // String functions
    Left,
    Right,
    Mid,
    Len,
    InStr,
    LCase,
    UCase,
    LTrim,
    RTrim,
    Trim,
    StrFunc,
    ValFunc,
    ChrFunc,
    AscFunc,
    SpaceFunc,
    StringFunc,

    // Math functions
    Abs,
    Sgn,
    Sin,
    Cos,
    Tan,
    Atn,
    ExpFunc,
    LogFunc,
    Sqr,
    IntFunc,
    Fix,
    Rnd,

    // Type conversion
    CInt,
    CLng,
    CSng,
    CDbl,
    CStr,

    // Array functions
    LBound(String, i32),
    UBound(String, i32),
    Erase(String),

    // Misc
    Swap(String, String),
    Sleep,
    Timer,
    Date,
    Time,
    Clear,
    Cls,
    Locate(i32, i32),
    Color(i32, i32),

    // File I/O
    LineInput(String),
    LineInputDynamic, // Pops file_number from stack, pushes line to variable
    WriteFile(String),
    WriteFileDynamic(usize),
    InputFile(String),
    InputFileDynamic(usize),
    PrintFile(String),
    PrintFileDynamic, // Pops file_number and value from stack
    Eof(String),
    Lof(String),
    Seek(String, i32),
    SeekDynamic,
    FreeFile,

    // Graphics
    Paint {
        x: i32,
        y: i32,
        paint_color: i32,
        border_color: i32,
    },
    Draw {
        commands: String,
    },
    Palette {
        attribute: i32,
        color: i32,
    },
    View {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        fill_color: i32,
        border_color: i32,
    },
    ViewPrint {
        top: i32,
        bottom: i32,
    },
    ViewReset,
    Window {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    WindowReset,

    // DATA/READ/RESTORE
    Data(Vec<QType>),
    Read(String),
    Restore(Option<String>),

    // Error handling
    Err,
    Erl,
    ErDev,
    ErDevStr,

    // Additional string/numeric functions
    HexFunc,
    OctFunc,
    MkiFunc,
    MklFunc,
    MksFunc,
    MkdFunc,
    CviFunc,
    CvlFunc,
    CvsFunc,
    CvdFunc,

    // System functions
    FreFunc(i32), // FRE with argument type: 0=string, -1=array, -2=stack
    CsrLinFunc,   // Current cursor line
    PosFunc(i32), // Current cursor position
    EnvironFunc,  // Environment variable
    CommandFunc,  // Command line arguments
    InKeyFunc,    // Get key press without waiting

    // Advanced features
    DefFn {
        // DEF FN inline function
        name: String,
        params: Vec<String>,
        body: Vec<OpCode>,
    },
    CallDefFn(String), // Call DEF FN function
    MidAssign {
        // MID$ as statement (assign to substring)
        var_name: String,
        start: i32,
        length: Option<i32>,
    },

    // Memory/Hardware (placeholders for compatibility)
    PeekFunc(i32),      // PEEK(address)
    PeekDynamic,        // PEEK(expr)
    PokeFunc(i32, i32), // POKE address, value
    PokeDynamic,        // POKE expr, expr
    WaitDynamic {
        has_xor: bool,
    }, // WAIT expr, expr [, expr]
    BLoadDynamic {
        has_offset: bool,
    }, // BLOAD file$ [, offset]
    BSaveDynamic,       // BSAVE file$, offset, length
    InpDynamic,         // INP(expr)
    OutDynamic,         // OUT expr, expr
    DefSeg(i32),        // DEF SEG = segment
    VarPtrFunc(String), // VARPTR(variable)
    VarSegFunc(String), // VARSEG(variable)
    SaddFunc(String),   // SADD(string)
    VarPtrStrFunc(String), // VARPTR$(variable)

    // File system
    FieldStmt {
        // FIELD statement for RANDOM files
        file_num: i32,
        fields: Vec<(i32, usize)>, // (width, global_var_index)
    },
    LSetField {
        var_index: usize,
        width: usize,
    },
    RSetField {
        var_index: usize,
        width: usize,
    },

    // Graphics advanced
    PointFunc(i32, i32), // POINT(x, y) - get pixel color
    PMapFunc(f64, i32),  // PMAP(coord, function)
    GetImage {
        // GET (graphics)
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        array: String,
    },
    PutImage {
        // PUT (graphics)
        x: i32,
        y: i32,
        array: String,
        action: String, // PSET, PRESET, AND, OR, XOR
    },

    // System commands
    Shell, // SHELL command - execute OS command
    Chain, // CHAIN command - load and run another program
    KillFile,
    RenameFile,
    ChangeDir,
    MakeDir,
    RemoveDir,
    Files,

    // Error handling
    ErrorStmt,          // ERROR statement - trigger error
    ResumeLabel(usize), // RESUME <label> - resume at label
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpCode::NoOp => write!(f, "NOP"),
            OpCode::LoadConstant(qtype) => write!(f, "LOAD_CONST {:?}", qtype),
            OpCode::LoadVariable(name) => write!(f, "LOAD {}", name),
            OpCode::StoreVariable(name) => write!(f, "STORE {}", name),
            OpCode::LoadFast(idx) => write!(f, "LOAD_FAST {}", idx),
            OpCode::StoreFast(idx) => write!(f, "STORE_FAST {}", idx),
            OpCode::SetStringWidth { slot, width } => {
                write!(f, "SET_STRING_WIDTH {} {}", slot, width)
            }
            OpCode::SetStringArrayWidth { name, width } => {
                write!(f, "SET_STRING_ARRAY_WIDTH {} {}", name, width)
            }
            OpCode::InitGlobals(count) => write!(f, "INIT_GLOBALS {}", count),
            OpCode::ReadFast(idx) => write!(f, "READ_FAST {}", idx),
            OpCode::SwapFast(idx1, idx2) => write!(f, "SWAP_FAST {} {}", idx1, idx2),
            OpCode::Add => write!(f, "ADD"),
            OpCode::Subtract => write!(f, "SUB"),
            OpCode::Multiply => write!(f, "MUL"),
            OpCode::Divide => write!(f, "DIV"),
            OpCode::IntegerDivide => write!(f, "IDIV"),
            OpCode::Modulo => write!(f, "MOD"),
            OpCode::Power => write!(f, "POW"),
            OpCode::Negate => write!(f, "NEG"),
            OpCode::Not => write!(f, "NOT"),
            OpCode::Equal => write!(f, "EQ"),
            OpCode::NotEqual => write!(f, "NEQ"),
            OpCode::LessThan => write!(f, "LT"),
            OpCode::GreaterThan => write!(f, "GT"),
            OpCode::LessOrEqual => write!(f, "LE"),
            OpCode::GreaterOrEqual => write!(f, "GE"),
            OpCode::And => write!(f, "AND"),
            OpCode::Or => write!(f, "OR"),
            OpCode::Xor => write!(f, "XOR"),
            OpCode::Eqv => write!(f, "EQV"),
            OpCode::Imp => write!(f, "IMP"),
            OpCode::Jump(addr) => write!(f, "JMP {}", addr),
            OpCode::JumpIfFalse(addr) => write!(f, "JZF {}", addr),
            OpCode::JumpIfTrue(addr) => write!(f, "JZT {}", addr),
            OpCode::Gosub(addr) => write!(f, "GOSUB {}", addr),
            OpCode::Return => write!(f, "RETURN"),
            OpCode::ForInit {
                var_name,
                end_label,
                step,
            } => write!(f, "FOR_INIT {} {} STEP {}", var_name, end_label, step),
            OpCode::ForInitFast {
                var_index,
                end_label,
                step,
            } => write!(f, "FOR_INIT_FAST {} {} STEP {}", var_index, end_label, step),
            OpCode::ForStep { var_name, step } => write!(f, "FOR_STEP {} {}", var_name, step),
            OpCode::ForStepFast { var_index, step } => {
                write!(f, "FOR_STEP_FAST {} {}", var_index, step)
            }
            OpCode::Next(name) => write!(f, "NEXT {}", name),
            OpCode::NextFast(idx) => write!(f, "NEXT_FAST {}", idx),
            OpCode::Print => write!(f, "PRINT"),
            OpCode::PrintNewline => write!(f, "PRINT_NL"),
            OpCode::PrintTab => write!(f, "PRINT_TAB"),
            OpCode::PrintSpace => write!(f, "PRINT_SPC"),
            OpCode::PrintUsing(n) => write!(f, "PRINT_USING {}", n),
            OpCode::Input => write!(f, "INPUT"),
            OpCode::Screen(mode) => write!(f, "SCREEN {}", mode),
            OpCode::Pset { x, y, color } => write!(f, "PSET({},{}) {}", x, y, color),
            OpCode::Preset { x, y, color } => write!(f, "PRESET({},{}) {}", x, y, color),
            OpCode::Line {
                x1,
                y1,
                x2,
                y2,
                color,
            } => write!(f, "LINE({},{}),({},{}) {}", x1, y1, x2, y2, color),
            OpCode::Circle {
                x,
                y,
                radius,
                color,
            } => write!(f, "CIRCLE({},{}) {} {}", x, y, radius, color),
            OpCode::Sound {
                frequency,
                duration,
            } => write!(f, "SOUND {} {}", frequency, duration),
            OpCode::Play(melody) => write!(f, "PLAY {}", melody),
            OpCode::Beep => write!(f, "BEEP"),
            OpCode::Open { mode } => write!(f, "OPEN {}", mode),
            OpCode::Close => write!(f, "CLOSE"),
            OpCode::Get => write!(f, "GET"),
            OpCode::Put => write!(f, "PUT"),
            OpCode::End => write!(f, "END"),
            OpCode::Stop => write!(f, "STOP"),
            OpCode::Call(name) => write!(f, "CALL {}", name),
            OpCode::CallNative(name) => write!(f, "CALL_NATIVE {}", name),
            OpCode::Dup => write!(f, "DUP"),
            OpCode::Pop => write!(f, "POP"),
            OpCode::ArrayIndex(name) => write!(f, "INDEX {}", name),
            OpCode::ArrayStore(name, num) => write!(f, "ARRAY_STORE {} {}", name, num),
            OpCode::ArrayLoad(name, num) => write!(f, "ARRAY_LOAD {} {}", name, num),
            OpCode::ArrayDim { name, dimensions } => {
                write!(f, "DIM {}({})", name, dimensions.len())
            }
            OpCode::ArrayRedim {
                name,
                dimensions,
                preserve,
            } => write!(
                f,
                "REDIM {} {} dims preserve={}",
                name,
                dimensions.len(),
                preserve
            ),
            OpCode::DefineFunction {
                name,
                params,
                result,
                body_start,
                body_end,
            } => write!(
                f,
                "DEF_FUNC {} ({}) result={} [{}-{}]",
                name,
                params
                    .iter()
                    .map(|param| param.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                result,
                body_start,
                body_end
            ),
            OpCode::DefineSub {
                name,
                params,
                body_start,
                body_end,
            } => write!(
                f,
                "DEF_SUB {} ({}) [{}-{}]",
                name,
                params
                    .iter()
                    .map(|param| param.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                body_start,
                body_end
            ),
            OpCode::CallFunction { name, by_ref } => {
                write!(f, "CALL_FUNC {} refs={:?}", name, by_ref)
            }
            OpCode::CallSub { name, by_ref } => write!(f, "CALL_SUB {} refs={:?}", name, by_ref),
            OpCode::FunctionReturn => write!(f, "FUNC_RETURN"),
            OpCode::SubReturn => write!(f, "SUB_RETURN"),
            OpCode::SelectCase => write!(f, "SELECT_CASE"),
            OpCode::CaseValue(val) => write!(f, "CASE {:?}", val),
            OpCode::CaseRange(start, end) => write!(f, "CASE {:?} TO {:?}", start, end),
            OpCode::CaseIs { op, value } => write!(f, "CASE IS {} {:?}", op, value),
            OpCode::CaseElse => write!(f, "CASE_ELSE"),
            OpCode::EndSelect => write!(f, "END_SELECT"),
            OpCode::ErrorHandler(addr) => write!(f, "ON_ERROR_HANDLER {}", addr),
            OpCode::Resume => write!(f, "RESUME"),
            OpCode::ResumeNext => write!(f, "RESUME_NEXT"),
            OpCode::OnError(addr) => write!(f, "ON_ERROR {}", addr),
            OpCode::OnErrorResumeNext => write!(f, "ON_ERROR_RESUME_NEXT"),
            OpCode::OnTimer {
                interval_secs,
                handler,
            } => write!(f, "ON_TIMER {} {}", interval_secs, handler),
            OpCode::TimerOn => write!(f, "TIMER_ON"),
            OpCode::TimerOff => write!(f, "TIMER_OFF"),
            OpCode::TimerStop => write!(f, "TIMER_STOP"),
            OpCode::Left => write!(f, "LEFT$"),
            OpCode::Right => write!(f, "RIGHT$"),
            OpCode::Mid => write!(f, "MID$"),
            OpCode::Len => write!(f, "LEN"),
            OpCode::InStr => write!(f, "INSTR"),
            OpCode::LCase => write!(f, "LCASE$"),
            OpCode::UCase => write!(f, "UCASE$"),
            OpCode::LTrim => write!(f, "LTRIM$"),
            OpCode::RTrim => write!(f, "RTRIM$"),
            OpCode::Trim => write!(f, "TRIM$"),
            OpCode::StrFunc => write!(f, "STR$"),
            OpCode::ValFunc => write!(f, "VAL"),
            OpCode::ChrFunc => write!(f, "CHR$"),
            OpCode::AscFunc => write!(f, "ASC"),
            OpCode::SpaceFunc => write!(f, "SPACE$"),
            OpCode::StringFunc => write!(f, "STRING$"),
            OpCode::Abs => write!(f, "ABS"),
            OpCode::Sgn => write!(f, "SGN"),
            OpCode::Sin => write!(f, "SIN"),
            OpCode::Cos => write!(f, "COS"),
            OpCode::Tan => write!(f, "TAN"),
            OpCode::Atn => write!(f, "ATN"),
            OpCode::ExpFunc => write!(f, "EXP"),
            OpCode::LogFunc => write!(f, "LOG"),
            OpCode::Sqr => write!(f, "SQR"),
            OpCode::IntFunc => write!(f, "INT"),
            OpCode::Fix => write!(f, "FIX"),
            OpCode::Rnd => write!(f, "RND"),
            OpCode::CInt => write!(f, "CINT"),
            OpCode::CLng => write!(f, "CLNG"),
            OpCode::CSng => write!(f, "CSNG"),
            OpCode::CDbl => write!(f, "CDBL"),
            OpCode::CStr => write!(f, "CSTR"),
            OpCode::LBound(name, dim) => write!(f, "LBOUND {} {}", name, dim),
            OpCode::UBound(name, dim) => write!(f, "UBOUND {} {}", name, dim),
            OpCode::Erase(name) => write!(f, "ERASE {}", name),
            OpCode::Swap(a, b) => write!(f, "SWAP {} {}", a, b),
            OpCode::Sleep => write!(f, "SLEEP"),
            OpCode::Timer => write!(f, "TIMER"),
            OpCode::Date => write!(f, "DATE$"),
            OpCode::Time => write!(f, "TIME$"),
            OpCode::Clear => write!(f, "CLEAR"),
            OpCode::Cls => write!(f, "CLS"),
            OpCode::Locate(row, col) => write!(f, "LOCATE {} {}", row, col),
            OpCode::Color(fg, bg) => write!(f, "COLOR {} {}", fg, bg),
            OpCode::LineInput(file) => write!(f, "LINE INPUT #{}", file),
            OpCode::LineInputDynamic => write!(f, "LINE INPUT #(dynamic)"),
            OpCode::WriteFile(file) => write!(f, "WRITE #{}", file),
            OpCode::WriteFileDynamic(count) => write!(f, "WRITE #(dynamic) {}", count),
            OpCode::InputFile(file) => write!(f, "INPUT #{}", file),
            OpCode::InputFileDynamic(count) => write!(f, "INPUT #(dynamic) {}", count),
            OpCode::PrintFile(file) => write!(f, "PRINT #{}", file),
            OpCode::PrintFileDynamic => write!(f, "PRINT #(dynamic)"),
            OpCode::Eof(file) => write!(f, "EOF({})", file),
            OpCode::Lof(file) => write!(f, "LOF({})", file),
            OpCode::Seek(file, pos) => write!(f, "SEEK {} {}", file, pos),
            OpCode::SeekDynamic => write!(f, "SEEK (dynamic)"),
            OpCode::FreeFile => write!(f, "FREEFILE"),
            OpCode::Paint {
                x,
                y,
                paint_color,
                border_color,
            } => {
                write!(f, "PAINT({},{}) {} {}", x, y, paint_color, border_color)
            }
            OpCode::Draw { commands } => write!(f, "DRAW {}", commands),
            OpCode::Palette { attribute, color } => {
                write!(f, "PALETTE {} {}", attribute, color)
            }
            OpCode::View {
                x1,
                y1,
                x2,
                y2,
                fill_color,
                border_color,
            } => {
                write!(
                    f,
                    "VIEW({},{})-({},{}) {} {}",
                    x1, y1, x2, y2, fill_color, border_color
                )
            }
            OpCode::ViewPrint { top, bottom } => write!(f, "VIEW PRINT {} TO {}", top, bottom),
            OpCode::ViewReset => write!(f, "VIEW RESET"),
            OpCode::Window { x1, y1, x2, y2 } => {
                write!(f, "WINDOW({},{})-({},{})", x1, y1, x2, y2)
            }
            OpCode::WindowReset => write!(f, "WINDOW RESET"),
            OpCode::Data(values) => write!(f, "DATA {:?}", values),
            OpCode::Read(var) => write!(f, "READ {}", var),
            OpCode::Restore(label) => write!(f, "RESTORE {:?}", label),
            OpCode::Err => write!(f, "ERR"),
            OpCode::Erl => write!(f, "ERL"),
            OpCode::ErDev => write!(f, "ERDEV"),
            OpCode::ErDevStr => write!(f, "ERDEV$"),
            OpCode::HexFunc => write!(f, "HEX$"),
            OpCode::OctFunc => write!(f, "OCT$"),
            OpCode::MkiFunc => write!(f, "MKI$"),
            OpCode::MklFunc => write!(f, "MKL$"),
            OpCode::MksFunc => write!(f, "MKS$"),
            OpCode::MkdFunc => write!(f, "MKD$"),
            OpCode::CviFunc => write!(f, "CVI"),
            OpCode::CvlFunc => write!(f, "CVL"),
            OpCode::CvsFunc => write!(f, "CVS"),
            OpCode::CvdFunc => write!(f, "CVD"),
            OpCode::FreFunc(arg) => write!(f, "FRE({})", arg),
            OpCode::CsrLinFunc => write!(f, "CSRLIN"),
            OpCode::PosFunc(arg) => write!(f, "POS({})", arg),
            OpCode::EnvironFunc => write!(f, "ENVIRON$"),
            OpCode::CommandFunc => write!(f, "COMMAND$"),
            OpCode::InKeyFunc => write!(f, "INKEY$"),
            OpCode::DefFn { name, params, .. } => write!(f, "DEF FN{}({:?})", name, params),
            OpCode::CallDefFn(name) => write!(f, "FN{}", name),
            OpCode::MidAssign {
                var_name,
                start,
                length,
            } => {
                write!(f, "MID$({}, {}, {:?}) =", var_name, start, length)
            }
            OpCode::PeekFunc(addr) => write!(f, "PEEK({})", addr),
            OpCode::PeekDynamic => write!(f, "PEEK(<expr>)"),
            OpCode::PokeFunc(addr, val) => write!(f, "POKE {}, {}", addr, val),
            OpCode::PokeDynamic => write!(f, "POKE <expr>, <expr>"),
            OpCode::WaitDynamic { has_xor } => {
                if *has_xor {
                    write!(f, "WAIT <expr>, <expr>, <expr>")
                } else {
                    write!(f, "WAIT <expr>, <expr>")
                }
            }
            OpCode::BLoadDynamic { has_offset } => {
                if *has_offset {
                    write!(f, "BLOAD <expr>, <expr>")
                } else {
                    write!(f, "BLOAD <expr>")
                }
            }
            OpCode::BSaveDynamic => write!(f, "BSAVE <expr>, <expr>, <expr>"),
            OpCode::InpDynamic => write!(f, "INP(<expr>)"),
            OpCode::OutDynamic => write!(f, "OUT <expr>, <expr>"),
            OpCode::DefSeg(seg) => write!(f, "DEF SEG = {}", seg),
            OpCode::VarPtrFunc(var) => write!(f, "VARPTR({})", var),
            OpCode::VarSegFunc(var) => write!(f, "VARSEG({})", var),
            OpCode::SaddFunc(var) => write!(f, "SADD({})", var),
            OpCode::VarPtrStrFunc(var) => write!(f, "VARPTR$({})", var),
            OpCode::FieldStmt { file_num, fields } => {
                write!(f, "FIELD #{}, {} fields", file_num, fields.len())
            }
            OpCode::LSetField { var_index, width } => {
                write!(f, "LSET_FAST {} {}", var_index, width)
            }
            OpCode::RSetField { var_index, width } => {
                write!(f, "RSET_FAST {} {}", var_index, width)
            }
            OpCode::PointFunc(x, y) => write!(f, "POINT({}, {})", x, y),
            OpCode::PMapFunc(coord, func) => write!(f, "PMAP({}, {})", coord, func),
            OpCode::GetImage {
                x1,
                y1,
                x2,
                y2,
                array,
            } => {
                write!(f, "GET ({}, {})-({}, {}), {}", x1, y1, x2, y2, array)
            }
            OpCode::PutImage {
                x,
                y,
                array,
                action,
            } => {
                write!(f, "PUT ({}, {}), {}, {}", x, y, array, action)
            }
            OpCode::Shell => write!(f, "SHELL"),
            OpCode::Chain => write!(f, "CHAIN"),
            OpCode::KillFile => write!(f, "KILL"),
            OpCode::RenameFile => write!(f, "NAME ... AS"),
            OpCode::ChangeDir => write!(f, "CHDIR"),
            OpCode::MakeDir => write!(f, "MKDIR"),
            OpCode::RemoveDir => write!(f, "RMDIR"),
            OpCode::Files => write!(f, "FILES"),
            OpCode::ErrorStmt => write!(f, "ERROR"),
            OpCode::ResumeLabel(addr) => write!(f, "RESUME {}", addr),
        }
    }
}

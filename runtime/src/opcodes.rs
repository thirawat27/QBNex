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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryFileKind {
    Integer,
    Long,
    Single,
    Double,
    String,
}

#[derive(Debug, Clone)]
pub enum OpCode {
    NoOp,

    LoadConstant(QType),
    LoadVariable(String),
    StoreVariable(String),
    LoadFast(usize),
    StoreFast(usize),
    SetNumericType {
        slot: usize,
        kind: BinaryFileKind,
    },
    SetStringWidth {
        slot: usize,
        width: usize,
    },
    SetNumericArrayType {
        name: String,
        kind: BinaryFileKind,
    },
    SetStringArrayWidth {
        name: String,
        width: usize,
    },
    InitGlobals(usize),
    SetOptionBase(i32),

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
    PrintComma,
    PrintTab,
    PrintSpace,
    PrintUsing {
        count: usize,
        comma_after: Vec<bool>,
    },
    LPrint,
    LPrintNewline,
    LPrintComma,
    LPrintTab,
    LPrintSpace,
    LPrintUsing {
        count: usize,
        comma_after: Vec<bool>,
    },

    Input,

    Screen(i32),
    ScreenDynamic,
    Pset {
        x: i32,
        y: i32,
        color: i32,
    },
    PsetDynamic,
    Preset {
        x: i32,
        y: i32,
        color: i32,
    },
    PresetDynamic,
    Line {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: i32,
    },
    LineDynamic,
    Circle {
        x: i32,
        y: i32,
        radius: i32,
        color: i32,
    },
    CircleDynamic,

    Sound {
        frequency: i32,
        duration: i32,
    },
    SoundDynamic,
    Play(String),
    PlayDynamic,
    Beep,

    Open {
        mode: String,
    },
    Close,
    GetBinary {
        kind: BinaryFileKind,
        fixed_length: usize,
    },
    PutBinary {
        kind: BinaryFileKind,
        fixed_length: usize,
    },
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
    ArrayDimDynamic {
        name: String,
        dimensions: usize,
    },
    ArrayRedim {
        name: String,
        dimensions: Vec<(i32, i32)>,
        preserve: bool,
    },
    ArrayRedimDynamic {
        name: String,
        dimensions: usize,
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
    OnPlay {
        queue_limit: usize,
        handler: usize,
    },
    OnPlayDynamic {
        handler: usize,
    },
    TimerOn,
    TimerOff,
    TimerStop,
    PlayOn,
    PlayOff,
    PlayStop,
    PlayFunc,

    // String functions
    Left,
    Right,
    Mid,
    MidNoLen,
    Len,
    InStr,
    InStrFrom,
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
    RndWithArg,

    // Type conversion
    CInt,
    CLng,
    CSng,
    CDbl,
    CStr,

    // Array functions
    LBound(String, i32),
    UBound(String, i32),
    LBoundDynamic(String),
    UBoundDynamic(String),
    Erase(String),

    // Misc
    Swap(String, String),
    Sleep,
    Randomize,
    RandomizeDynamic,
    Timer,
    Date,
    Time,
    ScreenFn(usize),
    Clear,
    Cls(i32),
    ClsDynamic,
    Locate(i32, i32),
    LocateDynamic,
    SetCursorState {
        visible: i32,
        start: i32,
        stop: i32,
    },
    SetCursorStateDynamic,
    Width {
        columns: i32,
        rows: i32,
    },
    WidthDynamic,
    Color(i32, i32),
    ColorDynamic,

    // File I/O
    LineInput(String),
    LineInputDynamic, // Pops file_number from stack, pushes line to variable
    WriteConsole(usize),
    WriteFile(String),
    WriteFileDynamic(usize),
    InputChars {
        has_file_number: bool,
    },
    InputFile(String),
    InputFileDynamic(usize),
    PrintFile(String),
    PrintFileDynamic, // Pops file_number and value from stack
    PrintFileUsingDynamic {
        count: usize,
        comma_after: Vec<bool>,
    },
    PrintFileCommaDynamic,   // Pops file_number and advances to next print zone
    PrintFileNewlineDynamic, // Pops file_number and writes newline
    Eof(String),
    EofDynamic,
    Lof(String),
    LofDynamic,
    Loc(String),
    LocDynamic,
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
    PaintDynamic,
    Draw {
        commands: String,
    },
    DrawDynamic,
    Palette {
        attribute: i32,
        color: i32,
    },
    PaletteDynamic,
    View {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        fill_color: i32,
        border_color: i32,
    },
    ViewDynamic,
    ViewPrint {
        top: i32,
        bottom: i32,
    },
    ViewPrintDynamic,
    ViewReset,
    Window {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    WindowDynamic,
    WindowReset,

    // DATA/READ/RESTORE
    Data(Vec<QType>),
    Read(String),
    Restore(Option<usize>),

    // Error handling
    Err,
    Erl,
    ErDev,
    ErDevStr,
    SetCurrentLine(u16),
    TraceOn,
    TraceOff,

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
    CvFunc(String),
    FileExistsFunc,
    DirExistsFunc,

    // System functions
    FreFunc(i32),  // FRE with argument type: 0=string, -1=array, -2=stack
    FreDynamic,    // FRE(expr)
    CsrLinFunc,    // Current cursor line
    PosFunc(i32),  // Current cursor position
    PosDynamic,    // POS(expr)
    LPosFunc(i32), // LPOS(expr)
    LPosDynamic,   // LPOS(expr dynamic)
    EnvironFunc,   // Environment variable
    CommandFunc,   // Command line arguments
    InKeyFunc,     // Get key press without waiting
    KeySetDynamic,
    KeyOn,
    KeyOff,
    KeyList,

    // Advanced features
    DefFn {
        // DEF FN inline function
        name: String,
        param_slots: Vec<usize>,
        body: Vec<OpCode>,
    },
    MarkConst(usize),
    CallDefFn(String), // Call DEF FN function
    MidAssign {
        // MID$ as statement (assign to substring)
        var_name: String,
        start: i32,
        length: Option<i32>,
    },
    AscAssign,

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
    DefSegDynamic,      // DEF SEG = expr
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
    PointDynamic,        // POINT(expr, expr)
    PMapFunc(f64, i32),  // PMAP(coord, function)
    PMapDynamic,         // PMAP(expr, expr)
    GetImage {
        // GET (graphics)
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        array: String,
    },
    GetImageDynamic {
        array: String,
    },
    PutImage {
        // PUT (graphics)
        x: i32,
        y: i32,
        array: String,
        action: String, // PSET, PRESET, AND, OR, XOR
    },
    PutImageDynamic {
        array: String,
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
            OpCode::SetNumericType { slot, kind } => {
                write!(f, "SET_NUMERIC_TYPE {} {:?}", slot, kind)
            }
            OpCode::SetStringWidth { slot, width } => {
                write!(f, "SET_STRING_WIDTH {} {}", slot, width)
            }
            OpCode::SetNumericArrayType { name, kind } => {
                write!(f, "SET_NUMERIC_ARRAY_TYPE {} {:?}", name, kind)
            }
            OpCode::SetStringArrayWidth { name, width } => {
                write!(f, "SET_STRING_ARRAY_WIDTH {} {}", name, width)
            }
            OpCode::InitGlobals(count) => write!(f, "INIT_GLOBALS {}", count),
            OpCode::SetOptionBase(base) => write!(f, "OPTION_BASE {}", base),
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
            OpCode::PrintComma => write!(f, "PRINT_COMMA"),
            OpCode::PrintTab => write!(f, "PRINT_TAB"),
            OpCode::PrintSpace => write!(f, "PRINT_SPC"),
            OpCode::PrintUsing { count, .. } => write!(f, "PRINT_USING {}", count),
            OpCode::LPrint => write!(f, "LPRINT"),
            OpCode::LPrintNewline => write!(f, "LPRINT_NL"),
            OpCode::LPrintComma => write!(f, "LPRINT_COMMA"),
            OpCode::LPrintTab => write!(f, "LPRINT_TAB"),
            OpCode::LPrintSpace => write!(f, "LPRINT_SPC"),
            OpCode::LPrintUsing { count, .. } => write!(f, "LPRINT_USING {}", count),
            OpCode::Input => write!(f, "INPUT"),
            OpCode::Screen(mode) => write!(f, "SCREEN {}", mode),
            OpCode::ScreenDynamic => write!(f, "SCREEN <expr>"),
            OpCode::Pset { x, y, color } => write!(f, "PSET({},{}) {}", x, y, color),
            OpCode::PsetDynamic => write!(f, "PSET(<expr>,<expr>) <expr>"),
            OpCode::Preset { x, y, color } => write!(f, "PRESET({},{}) {}", x, y, color),
            OpCode::PresetDynamic => write!(f, "PRESET(<expr>,<expr>) <expr>"),
            OpCode::Line {
                x1,
                y1,
                x2,
                y2,
                color,
            } => write!(f, "LINE({},{}),({},{}) {}", x1, y1, x2, y2, color),
            OpCode::LineDynamic => write!(f, "LINE(<expr>,<expr>),(<expr>,<expr>) <expr>"),
            OpCode::Circle {
                x,
                y,
                radius,
                color,
            } => write!(f, "CIRCLE({},{}) {} {}", x, y, radius, color),
            OpCode::CircleDynamic => write!(f, "CIRCLE(<expr>,<expr>) <expr> <expr>"),
            OpCode::Sound {
                frequency,
                duration,
            } => write!(f, "SOUND {} {}", frequency, duration),
            OpCode::SoundDynamic => write!(f, "SOUND <expr> <expr>"),
            OpCode::Play(melody) => write!(f, "PLAY {}", melody),
            OpCode::PlayDynamic => write!(f, "PLAY <expr>"),
            OpCode::Beep => write!(f, "BEEP"),
            OpCode::Open { mode } => write!(f, "OPEN {}", mode),
            OpCode::Close => write!(f, "CLOSE"),
            OpCode::GetBinary { kind, fixed_length } => {
                write!(f, "GET_BINARY {:?} {}", kind, fixed_length)
            }
            OpCode::PutBinary { kind, fixed_length } => {
                write!(f, "PUT_BINARY {:?} {}", kind, fixed_length)
            }
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
            OpCode::ArrayDimDynamic { name, dimensions } => {
                write!(f, "DIM_DYNAMIC {}({})", name, dimensions)
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
            OpCode::ArrayRedimDynamic {
                name,
                dimensions,
                preserve,
            } => write!(
                f,
                "REDIM_DYNAMIC {} {} dims preserve={}",
                name, dimensions, preserve
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
            OpCode::OnPlay {
                queue_limit,
                handler,
            } => write!(f, "ON_PLAY {} {}", queue_limit, handler),
            OpCode::OnPlayDynamic { handler } => write!(f, "ON_PLAY_DYNAMIC {}", handler),
            OpCode::TimerOn => write!(f, "TIMER_ON"),
            OpCode::TimerOff => write!(f, "TIMER_OFF"),
            OpCode::TimerStop => write!(f, "TIMER_STOP"),
            OpCode::PlayOn => write!(f, "PLAY_ON"),
            OpCode::PlayOff => write!(f, "PLAY_OFF"),
            OpCode::PlayStop => write!(f, "PLAY_STOP"),
            OpCode::PlayFunc => write!(f, "PLAY_FUNC"),
            OpCode::Left => write!(f, "LEFT$"),
            OpCode::Right => write!(f, "RIGHT$"),
            OpCode::Mid => write!(f, "MID$"),
            OpCode::MidNoLen => write!(f, "MID$(<start-to-end>)"),
            OpCode::Len => write!(f, "LEN"),
            OpCode::InStr => write!(f, "INSTR"),
            OpCode::InStrFrom => write!(f, "INSTR(<start>, ...)"),
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
            OpCode::RndWithArg => write!(f, "RND(<expr>)"),
            OpCode::CInt => write!(f, "CINT"),
            OpCode::CLng => write!(f, "CLNG"),
            OpCode::CSng => write!(f, "CSNG"),
            OpCode::CDbl => write!(f, "CDBL"),
            OpCode::CStr => write!(f, "CSTR"),
            OpCode::LBound(name, dim) => write!(f, "LBOUND {} {}", name, dim),
            OpCode::UBound(name, dim) => write!(f, "UBOUND {} {}", name, dim),
            OpCode::LBoundDynamic(name) => write!(f, "LBOUND_DYNAMIC {}", name),
            OpCode::UBoundDynamic(name) => write!(f, "UBOUND_DYNAMIC {}", name),
            OpCode::Erase(name) => write!(f, "ERASE {}", name),
            OpCode::Swap(a, b) => write!(f, "SWAP {} {}", a, b),
            OpCode::Sleep => write!(f, "SLEEP"),
            OpCode::Randomize => write!(f, "RANDOMIZE"),
            OpCode::RandomizeDynamic => write!(f, "RANDOMIZE <expr>"),
            OpCode::Timer => write!(f, "TIMER"),
            OpCode::Date => write!(f, "DATE$"),
            OpCode::Time => write!(f, "TIME$"),
            OpCode::ScreenFn(arg_count) => write!(f, "SCREEN_FN {}", arg_count),
            OpCode::Clear => write!(f, "CLEAR"),
            OpCode::Cls(mode) => write!(f, "CLS {}", mode),
            OpCode::ClsDynamic => write!(f, "CLS <expr>"),
            OpCode::Locate(row, col) => write!(f, "LOCATE {} {}", row, col),
            OpCode::LocateDynamic => write!(f, "LOCATE <expr>, <expr>"),
            OpCode::SetCursorState {
                visible,
                start,
                stop,
            } => write!(f, "LOCATE CURSOR {} {} {}", visible, start, stop),
            OpCode::SetCursorStateDynamic => {
                write!(f, "LOCATE CURSOR <expr>, <expr>, <expr>")
            }
            OpCode::Width { columns, rows } => write!(f, "WIDTH {} {}", columns, rows),
            OpCode::WidthDynamic => write!(f, "WIDTH <expr>, <expr>"),
            OpCode::Color(fg, bg) => write!(f, "COLOR {} {}", fg, bg),
            OpCode::ColorDynamic => write!(f, "COLOR <expr>, <expr>"),
            OpCode::LineInput(file) => write!(f, "LINE INPUT #{}", file),
            OpCode::LineInputDynamic => write!(f, "LINE INPUT #(dynamic)"),
            OpCode::WriteConsole(count) => write!(f, "WRITE {}", count),
            OpCode::WriteFile(file) => write!(f, "WRITE #{}", file),
            OpCode::WriteFileDynamic(count) => write!(f, "WRITE #(dynamic) {}", count),
            OpCode::InputChars { has_file_number } => {
                if *has_file_number {
                    write!(f, "INPUT$(<count>, <file>)")
                } else {
                    write!(f, "INPUT$(<count>)")
                }
            }
            OpCode::InputFile(file) => write!(f, "INPUT #{}", file),
            OpCode::InputFileDynamic(count) => write!(f, "INPUT #(dynamic) {}", count),
            OpCode::PrintFile(file) => write!(f, "PRINT #{}", file),
            OpCode::PrintFileDynamic => write!(f, "PRINT #(dynamic)"),
            OpCode::PrintFileUsingDynamic { count, .. } => {
                write!(f, "PRINT #(dynamic) USING {}", count)
            }
            OpCode::PrintFileCommaDynamic => write!(f, "PRINT #(dynamic) COMMA"),
            OpCode::PrintFileNewlineDynamic => write!(f, "PRINT #(dynamic) NL"),
            OpCode::Eof(file) => write!(f, "EOF({})", file),
            OpCode::EofDynamic => write!(f, "EOF(dynamic)"),
            OpCode::Lof(file) => write!(f, "LOF({})", file),
            OpCode::LofDynamic => write!(f, "LOF(dynamic)"),
            OpCode::Loc(file) => write!(f, "LOC({})", file),
            OpCode::LocDynamic => write!(f, "LOC(dynamic)"),
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
            OpCode::PaintDynamic => write!(f, "PAINT(<expr>,<expr>) <expr> <expr>"),
            OpCode::Draw { commands } => write!(f, "DRAW {}", commands),
            OpCode::DrawDynamic => write!(f, "DRAW <expr>"),
            OpCode::Palette { attribute, color } => {
                write!(f, "PALETTE {} {}", attribute, color)
            }
            OpCode::PaletteDynamic => write!(f, "PALETTE <expr> <expr>"),
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
            OpCode::ViewDynamic => write!(f, "VIEW(<expr>,<expr>)-(<expr>,<expr>) <expr> <expr>"),
            OpCode::ViewPrint { top, bottom } => write!(f, "VIEW PRINT {} TO {}", top, bottom),
            OpCode::ViewPrintDynamic => write!(f, "VIEW PRINT <expr> TO <expr>"),
            OpCode::ViewReset => write!(f, "VIEW RESET"),
            OpCode::Window { x1, y1, x2, y2 } => {
                write!(f, "WINDOW({},{})-({},{})", x1, y1, x2, y2)
            }
            OpCode::WindowDynamic => write!(f, "WINDOW(<expr>,<expr>)-(<expr>,<expr>)"),
            OpCode::WindowReset => write!(f, "WINDOW RESET"),
            OpCode::Data(values) => write!(f, "DATA {:?}", values),
            OpCode::Read(var) => write!(f, "READ {}", var),
            OpCode::Restore(section) => write!(f, "RESTORE {:?}", section),
            OpCode::Err => write!(f, "ERR"),
            OpCode::Erl => write!(f, "ERL"),
            OpCode::ErDev => write!(f, "ERDEV"),
            OpCode::ErDevStr => write!(f, "ERDEV$"),
            OpCode::SetCurrentLine(line) => write!(f, "LINE {}", line),
            OpCode::TraceOn => write!(f, "TRON"),
            OpCode::TraceOff => write!(f, "TROFF"),
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
            OpCode::CvFunc(type_name) => write!(f, "_CV({})", type_name),
            OpCode::FileExistsFunc => write!(f, "_FILEEXISTS"),
            OpCode::DirExistsFunc => write!(f, "_DIREXISTS"),
            OpCode::FreFunc(arg) => write!(f, "FRE({})", arg),
            OpCode::FreDynamic => write!(f, "FRE(<expr>)"),
            OpCode::CsrLinFunc => write!(f, "CSRLIN"),
            OpCode::PosFunc(arg) => write!(f, "POS({})", arg),
            OpCode::PosDynamic => write!(f, "POS(<expr>)"),
            OpCode::LPosFunc(arg) => write!(f, "LPOS({})", arg),
            OpCode::LPosDynamic => write!(f, "LPOS(<expr>)"),
            OpCode::EnvironFunc => write!(f, "ENVIRON$"),
            OpCode::CommandFunc => write!(f, "COMMAND$"),
            OpCode::InKeyFunc => write!(f, "INKEY$"),
            OpCode::KeySetDynamic => write!(f, "KEY <expr>, <expr>"),
            OpCode::KeyOn => write!(f, "KEY ON"),
            OpCode::KeyOff => write!(f, "KEY OFF"),
            OpCode::KeyList => write!(f, "KEY LIST"),
            OpCode::DefFn {
                name, param_slots, ..
            } => write!(f, "DEF FN{}({:?})", name, param_slots),
            OpCode::MarkConst(slot) => write!(f, "MARK_CONST {}", slot),
            OpCode::CallDefFn(name) => write!(f, "FN{}", name),
            OpCode::MidAssign {
                var_name,
                start,
                length,
            } => {
                write!(f, "MID$({}, {}, {:?}) =", var_name, start, length)
            }
            OpCode::AscAssign => write!(f, "ASC(<string>, <position>) = <value>"),
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
            OpCode::DefSegDynamic => write!(f, "DEF SEG = <expr>"),
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
            OpCode::PointDynamic => write!(f, "POINT(<expr>, <expr>)"),
            OpCode::PMapFunc(coord, func) => write!(f, "PMAP({}, {})", coord, func),
            OpCode::PMapDynamic => write!(f, "PMAP(<expr>, <expr>)"),
            OpCode::GetImage {
                x1,
                y1,
                x2,
                y2,
                array,
            } => {
                write!(f, "GET ({}, {})-({}, {}), {}", x1, y1, x2, y2, array)
            }
            OpCode::GetImageDynamic { array } => {
                write!(f, "GET (<expr>, <expr>)-(<expr>, <expr>), {}", array)
            }
            OpCode::PutImage {
                x,
                y,
                array,
                action,
            } => {
                write!(f, "PUT ({}, {}), {}, {}", x, y, array, action)
            }
            OpCode::PutImageDynamic { array } => {
                write!(f, "PUT (<expr>, <expr>), {}, <expr>", array)
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

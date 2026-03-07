use core_types::QType;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub line_numbers: HashMap<u16, usize>,
    pub labels: HashMap<String, usize>,
    pub functions: HashMap<String, FunctionDef>,
    pub subs: HashMap<String, SubDef>,
    pub user_types: HashMap<String, UserType>,
    pub data_statements: Vec<Vec<Expression>>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
            line_numbers: HashMap::new(),
            labels: HashMap::new(),
            functions: HashMap::new(),
            subs: HashMap::new(),
            user_types: HashMap::new(),
            data_statements: Vec::new(),
        }
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub return_type: QType,
    pub params: Vec<Variable>,
    pub body: Vec<Statement>,
    pub is_static: bool,
}

#[derive(Debug, Clone)]
pub struct SubDef {
    pub name: String,
    pub params: Vec<Variable>,
    pub body: Vec<Statement>,
    pub is_static: bool,
}

#[derive(Debug, Clone)]
pub struct UserType {
    pub name: String,
    pub fields: Vec<TypeField>,
}

#[derive(Debug, Clone)]
pub struct TypeField {
    pub name: String,
    pub field_type: QType,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Print {
        expressions: Vec<Expression>,
        newline: bool,
    },
    PrintUsing {
        format: Expression,
        expressions: Vec<Expression>,
        newline: bool,
    },
    PrintFile {
        file_number: Box<Expression>,
        expressions: Vec<Expression>,
        newline: bool,
    },
    Assignment {
        target: Expression,
        value: Expression,
    },
    IfBlock {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    IfElseBlock {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_ifs: Vec<(Expression, Vec<Statement>)>,
        else_branch: Option<Vec<Statement>>,
    },
    ForLoop {
        variable: Variable,
        start: Expression,
        end: Expression,
        step: Option<Expression>,
        body: Vec<Statement>,
    },
    WhileLoop {
        condition: Expression,
        body: Vec<Statement>,
    },
    DoLoop {
        condition: Option<Expression>,
        body: Vec<Statement>,
        pre_condition: bool,
    },
    Select {
        expression: Expression,
        cases: Vec<(Expression, Vec<Statement>)>,
    },
    Goto {
        target: GotoTarget,
    },
    Gosub {
        target: GotoTarget,
    },
    Return,
    Label {
        name: String,
    },
    LineNumber {
        number: u16,
    },
    Input {
        prompt: Option<Expression>,
        variables: Vec<Expression>,
        semicolon: bool,
    },
    Open {
        filename: Expression,
        mode: OpenMode,
        file_number: Expression,
        record_len: Option<Expression>,
        access: Option<OpenAccess>,
        lock: Option<OpenLock>,
    },
    Close {
        file_numbers: Vec<Expression>,
    },
    Get {
        file_number: Expression,
        record: Option<Expression>,
        variable: Option<Expression>,
    },
    Put {
        file_number: Expression,
        record: Option<Expression>,
        variable: Option<Expression>,
    },
    GetImage {
        coords: ((Expression, Expression), (Expression, Expression)),
        variable: Expression,
    },
    PutImage {
        coords: (Expression, Expression),
        variable: Expression,
        action: Option<Expression>,
    },
    Write {
        expressions: Vec<Expression>,
    },
    Screen {
        mode: Option<Expression>,
    },
    Pset {
        coords: (Expression, Expression),
        color: Option<Expression>,
    },
    Preset {
        coords: (Expression, Expression),
        color: Option<Expression>,
    },
    Line {
        coords: ((Expression, Expression), (Expression, Expression)),
        color: Option<Expression>,
        style: Option<Expression>,
        step: (bool, bool),
    },
    Circle {
        center: (Expression, Expression),
        radius: Expression,
        color: Option<Expression>,
        start: Option<Expression>,
        end: Option<Expression>,
        aspect: Option<Expression>,
    },
    Paint {
        coords: (Expression, Expression),
        paint_color: Option<Expression>,
        border_color: Option<Expression>,
    },
    Sound {
        frequency: Expression,
        duration: Expression,
    },
    Play {
        melody: Expression,
    },
    Beep,
    ForEach {
        variable: Variable,
        array: Expression,
        body: Vec<Statement>,
    },
    Call {
        name: String,
        args: Vec<Expression>,
    },
    FunctionCall(FunctionCall),
    DefType {
        letter_range: (char, char),
        type_name: String,
    },
    Dim {
        variables: Vec<(Variable, Option<Expression>)>,
        is_static: bool,
        is_shared: bool,
        is_common: bool,
    },
    Redim {
        variables: Vec<(Variable, Option<Expression>)>,
        preserve: bool,
    },
    Erase {
        variables: Vec<Variable>,
    },
    Data {
        values: Vec<String>,
    },
    Read {
        variables: Vec<Variable>,
    },
    Restore {
        label: Option<String>,
    },
    Const {
        name: String,
        value: Expression,
    },
    Randomize {
        seed: Option<Expression>,
    },
    Cls,
    Locate {
        row: Option<Expression>,
        col: Option<Expression>,
    },
    OnError {
        label: Option<String>,
    },
    OnErrorResumeNext,
    Error {
        code: Expression,
    },
    Resume,
    ResumeNext,
    ResumeLabel {
        label: String,
    },
    End,
    Stop,
    Clear,
    Chain {
        filename: Expression,
        delete: Option<Expression>,
    },
    Shell {
        command: Option<Expression>,
    },
    Exit {
        exit_type: ExitType,
    },
    Swap {
        var1: Expression,
        var2: Expression,
    },
    Sleep {
        duration: Option<Expression>,
    },
    System,
    Kill {
        filename: Expression,
    },
    NameFile {
        old_name: Expression,
        new_name: Expression,
    },
    Files {
        pattern: Option<Expression>,
    },
    ChDir {
        path: Expression,
    },
    MkDir {
        path: Expression,
    },
    RmDir {
        path: Expression,
    },
    Field {
        file_number: Expression,
        fields: Vec<(Expression, Expression)>,
    },
    LSet {
        target: Expression,
        value: Expression,
    },
    RSet {
        target: Expression,
        value: Expression,
    },
    Color {
        foreground: Option<Expression>,
        background: Option<Expression>,
    },
    Width {
        columns: Expression,
        rows: Option<Expression>,
    },
    View {
        coords: ((Expression, Expression), (Expression, Expression)),
        fill_color: Option<Expression>,
        border_color: Option<Expression>,
    },
    ViewPrint {
        top: Option<Expression>,
        bottom: Option<Expression>,
    },
    ViewReset,
    Window {
        coords: ((Expression, Expression), (Expression, Expression)),
    },
    WindowReset,
    Draw {
        commands: Expression,
    },
    Palette {
        attribute: Expression,
        color: Option<Expression>,
    },
    Key {
        key_num: Expression,
        key_string: Expression,
    },
    KeyOn,
    KeyOff,
    KeyList,
    OnTimer {
        interval: Expression,
        label: String,
    },
    OnGotoGosub {
        expression: Expression,
        targets: Vec<GotoTarget>,
        is_gosub: bool,
    },
    TimerOn,
    TimerOff,
    TimerStop,
    InputFile {
        file_number: Expression,
        variables: Vec<Expression>,
    },
    LineInput {
        prompt: Option<Expression>,
        variable: Expression,
    },
    LineInputFile {
        file_number: Expression,
        variable: Expression,
    },
    WriteFile {
        file_number: Expression,
        expressions: Vec<Expression>,
    },
    Seek {
        file_number: Expression,
        position: Expression,
    },
    OptionBase {
        base: i16,
    },
    Declare {
        name: String,
        is_function: bool,
    },
    DefFn {
        name: String,
        params: Vec<String>,
        body: Expression,
    },
    DefSeg {
        segment: Option<Box<Expression>>,
    },
}

#[derive(Debug, Clone)]
pub enum ExitType {
    For,
    Do,
    Function,
    Sub,
}

#[derive(Debug, Clone)]
pub enum GotoTarget {
    LineNumber(u16),
    Label(String),
}

#[derive(Debug, Clone)]
pub enum OpenMode {
    Append,
    Binary,
    Input,
    Output,
    Random,
}

#[derive(Debug, Clone)]
pub enum OpenAccess {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone)]
pub enum OpenLock {
    Shared,
    LockRead,
    LockWrite,
    LockReadWrite,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(QType),
    Variable(Variable),
    ArrayAccess {
        name: String,
        indices: Vec<Expression>,
        type_suffix: Option<char>,
    },
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },
    BinaryOp {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expression>,
    },
    FunctionCall(FunctionCall),
    TypeCast {
        target_type: QType,
        expression: Box<Expression>,
    },
    CaseRange {
        start: Box<Expression>,
        end: Box<Expression>,
    },
    CaseIs {
        op: BinaryOp,
        value: Box<Expression>,
    },
    CaseElse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
    Modulo,
    Power,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub type_suffix: Option<char>,
    pub indices: Vec<Expression>,
}

impl Variable {
    pub fn new(name: String) -> Self {
        Self {
            name,
            type_suffix: None,
            indices: Vec::new(),
        }
    }

    pub fn with_suffix(mut self, suffix: char) -> Self {
        self.type_suffix = Some(suffix);
        self
    }

    pub fn get_default_type(&self, def_types: &DefTypeMap) -> QType {
        if let Some(suffix) = self.type_suffix {
            return match suffix {
                '%' => QType::Integer(0),
                '&' => QType::Long(0),
                '!' => QType::Single(0.0),
                '#' => QType::Double(0.0),
                '$' => QType::String(String::new()),
                _ => QType::Empty,
            };
        }

        let first_char = self.name.chars().next().unwrap_or('a').to_ascii_lowercase();
        def_types.get_type(first_char)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<Expression>,
    pub type_suffix: Option<char>,
}

impl FunctionCall {
    pub fn new(name: String) -> Self {
        Self {
            name,
            args: Vec::new(),
            type_suffix: None,
        }
    }

    pub fn with_args(mut self, args: Vec<Expression>) -> Self {
        self.args = args;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct DefTypeMap {
    pub ranges: Vec<((char, char), QType)>,
}

impl DefTypeMap {
    pub fn new() -> Self {
        let mut map = Self::default();
        map.ranges.push((('a', 'i'), QType::Integer(0)));
        map.ranges.push((('j', 'z'), QType::Single(0.0)));
        map
    }

    pub fn get_type(&self, first_char: char) -> QType {
        for ((start, end), qtype) in &self.ranges {
            if first_char >= *start && first_char <= *end {
                return qtype.clone();
            }
        }
        QType::String(String::new())
    }

    pub fn set_range(&mut self, start: char, end: char, qtype: QType) {
        self.ranges
            .retain(|(range, _)| !(range.0 <= start && range.1 >= end));
        self.ranges.push(((start, end), qtype));
    }
}

pub type LineNumber = u16;
pub type Label = String;

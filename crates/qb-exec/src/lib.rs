//! `qb-exec` — R11 spike: executes QIR, the IR the BASIC compiler emits with
//! `--ir` (see docs/rust-port/r11-emitter-design.md for the format).
//!
//! The program is a flat instruction list. Labels are resolved to indices at
//! load time, so GOTO/GOSUB/RESUME/ON ERROR are plain jumps — the constructs
//! that make a Rust source backend hard (no `goto` in Rust) are trivial here.
//!
//! Numbers keep the QBasic type tag of the expression (INTEGER, LONG, SINGLE,
//! DOUBLE). SINGLE values are held at double precision until they are stored
//! or printed, matching what the C++ backend does (`y# = 1 / 3` keeps 53 bits).
//! Formatting and rounding come from `qb-runtime`, the same code the C++ path links.
//!
//! Storage: frame locals live in `cells` (a stack, truncated when a SUB
//! returns); `DIM SHARED`, STATIC locals and anything created while an error
//! handler looks at module level live in `heap`; arrays live in `arrays`.

mod builtins;

/// `qb-runtime` is linked as a whole: its string engine (R3) refers to symbols that
/// libqb.cpp/qbx.cpp define for C++ programs. qb-exec never calls that code, so
/// it only needs the symbols to exist.
#[allow(non_upper_case_globals)]
mod libqb_host {
    #[no_mangle]
    pub static mut cmem_sp: u32 = 0;
    #[no_mangle]
    pub static mut dblock: isize = 0;
    #[no_mangle]
    pub static mut cmem: [u8; 1] = [0];
    #[no_mangle]
    pub extern "C" fn qb_host_error(_n: i32) {
        std::process::abort()
    }
    #[no_mangle]
    pub extern "C" fn qb_host_field_free(_s: *mut core::ffi::c_void) {
        std::process::abort()
    }
}

use std::collections::HashMap;
use std::io::Write;

/// Runtime error raised by a QIR program (QBasic error code + source line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunError {
    pub code: i32,
    pub line: u32,
    pub msg: String,
}

impl RunError {
    fn load(msg: impl Into<String>) -> Self {
        RunError { code: -1, line: 0, msg: msg.into() }
    }
}

/// Parses and runs a QIR program, writing console output to `out`.
pub fn run_ir(src: &str, out: &mut dyn Write) -> Result<(), RunError> {
    let prog = Program::parse(src)?;
    let mut vm = Vm::new(&prog, out);
    vm.run()
}

// ---------------------------------------------------------------- values

/// Where a variable or array element lives.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Place {
    Cell(usize),
    Heap(usize),
    Elem(usize, usize),
}

const NO_ARRAY: usize = usize::MAX;

#[derive(Debug, Clone, PartialEq)]
enum Val {
    I(i16),
    L(i32),
    /// SINGLE, carried at double precision (see module docs).
    F(f64),
    D(f64),
    S(Vec<u8>),
    /// Reference (BYREF argument, SWAP operand).
    R(Place),
    /// Array handle (index into `Vm::arrays`, `NO_ARRAY` before DIM).
    A(usize),
}

impl Val {
    fn rank(&self) -> u8 {
        match self {
            Val::I(_) => 0,
            Val::L(_) => 1,
            Val::F(_) => 2,
            Val::D(_) => 3,
            _ => 9,
        }
    }
    fn is_int(&self) -> bool {
        matches!(self, Val::I(_) | Val::L(_))
    }
    fn f(&self) -> Result<f64, i32> {
        match self {
            Val::I(v) => Ok(f64::from(*v)),
            Val::L(v) => Ok(f64::from(*v)),
            Val::F(v) | Val::D(v) => Ok(*v),
            _ => Err(13),
        }
    }
    /// Integer value; floats round half-even like QBasic (`qb-runtime` rounding).
    fn i(&self) -> Result<i64, i32> {
        match self {
            Val::I(v) => Ok(i64::from(*v)),
            Val::L(v) => Ok(i64::from(*v)),
            Val::F(v) | Val::D(v) => Ok(qb_runtime::round::qb_qbr_f64(*v)),
            _ => Err(13),
        }
    }
    fn s(self) -> Result<Vec<u8>, i32> {
        match self {
            Val::S(s) => Ok(s),
            _ => Err(13),
        }
    }
    fn truth(&self) -> Result<bool, i32> {
        Ok(self.f()? != 0.0)
    }
}

fn bool_val(b: bool) -> Val {
    Val::I(if b { -1 } else { 0 })
}

/// Integer result typed INTEGER or LONG by operand rank (C cast semantics).
fn int_result(v: i64, rank: u8) -> Val {
    if rank == 0 {
        Val::I(v as i16)
    } else {
        Val::L(v as i32)
    }
}

fn float_result(v: f64, rank: u8) -> Val {
    if rank >= 3 {
        Val::D(v)
    } else {
        Val::F(v)
    }
}

/// Default value of a variable from its name suffix; `None` = untyped temp.
/// Array names end in `()` and default to an unallocated handle.
fn suffix_default(name: &str) -> Option<Val> {
    if name.ends_with("()") {
        return Some(Val::A(NO_ARRAY));
    }
    scalar_default(name)
}

fn scalar_default(name: &str) -> Option<Val> {
    match name.trim_end_matches("()").as_bytes().last()? {
        b'%' => Some(Val::I(0)),
        b'&' => Some(Val::L(0)),
        b'!' => Some(Val::F(0.0)),
        b'#' => Some(Val::D(0.0)),
        b'$' => Some(Val::S(Vec::new())),
        _ => None,
    }
}

/// Converts `v` for storage into something typed like `proto`.
fn convert_like(proto: &Val, v: Val) -> Result<Val, i32> {
    Ok(match proto {
        Val::I(_) => {
            let x = v.i()?;
            if v.is_int() {
                Val::I(x as i16)
            } else {
                Val::I(i16::try_from(x).map_err(|_| 6)?)
            }
        }
        Val::L(_) => {
            let x = v.i()?;
            if v.is_int() {
                Val::L(x as i32)
            } else {
                Val::L(i32::try_from(x).map_err(|_| 6)?)
            }
        }
        Val::F(_) => Val::F(f64::from(v.f()? as f32)),
        Val::D(_) => Val::D(v.f()?),
        Val::S(_) => Val::S(v.s()?),
        Val::R(_) | Val::A(_) => v,
    })
}

fn str_of(v: &Val) -> Result<Vec<u8>, i32> {
    Ok(match v {
        Val::I(x) => qb_runtime::numfmt::str_i64(i64::from(*x)),
        Val::L(x) => qb_runtime::numfmt::str_i64(i64::from(*x)),
        Val::F(x) => qb_runtime::numfmt::str_f32(*x as f32),
        Val::D(x) => qb_runtime::numfmt::str_f64(*x),
        _ => return Err(13),
    })
}

// ---------------------------------------------------------------- program

#[derive(Debug, Clone)]
enum Op {
    Line(u32),
    Push(Val),
    Load(u32),
    Store(u32),
    Ref(u32),
    /// array id, number of indices
    LoadE(u32, usize),
    StoreE(u32, usize),
    RefE(u32, usize),
    DimA(u32, usize),
    /// array id, dims, preserve
    RedimA(u32, usize, bool),
    /// array id, upper bound? (dimension number on the stack)
    Bound(u32, bool),
    Swap,
    Bin(BinOp),
    Un(UnOp),
    Fn(String, usize),
    Print,
    PrintZone,
    PrintNl,
    Jmp(usize),
    Jz(usize),
    Jnz(usize),
    Gosub(usize),
    Return,
    ForTest,
    /// sub index, argc
    Call(usize, usize),
    EndSub,
    OnError(Option<usize>),
    Resume(ResumeKind),
    Error,
    /// READ: push the next DATA item as a string (true) or its VAL (false)
    Read(bool),
    /// RESTORE [label]: data index
    Restore(usize),
    /// RANDOMIZE [USING] seed
    Randomize(bool),
    End,
}

#[derive(Debug, Clone, Copy)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    IDiv,
    Mod,
    Pow,
    And,
    Or,
    Xor,
    Eqv,
    Imp,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl BinOp {
    fn parse(s: &str) -> Option<BinOp> {
        use BinOp::*;
        Some(match s {
            "ADD" => Add,
            "SUB" => Sub,
            "MUL" => Mul,
            "DIV" => Div,
            "IDIV" => IDiv,
            "MOD" => Mod,
            "POW" => Pow,
            "AND" => And,
            "OR" => Or,
            "XOR" => Xor,
            "EQV" => Eqv,
            "IMP" => Imp,
            "EQ" => Eq,
            "NE" => Ne,
            "LT" => Lt,
            "LE" => Le,
            "GT" => Gt,
            "GE" => Ge,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
enum ResumeKind {
    Same,
    Next,
    Label(usize),
}

struct SubDef {
    entry: usize,
    params: Vec<u32>,
    /// FUNCTION: variable holding the return value (the function's name)
    ret: Option<u32>,
    is_static: bool,
}

/// Loaded program. Variable names are interned to dense ids at load time, so
/// LOAD/STORE index a per-frame slot table instead of hashing names.
struct Program {
    code: Vec<Op>,
    subs: Vec<SubDef>,
    shared: Vec<u32>,
    names: Vec<String>,
    /// DATA items (raw text) in program order
    data: Vec<Vec<u8>>,
}

#[derive(Default)]
struct Interner {
    ids: HashMap<String, u32>,
    names: Vec<String>,
}

impl Interner {
    fn id(&mut self, name: &str) -> u32 {
        if let Some(&i) = self.ids.get(name) {
            return i;
        }
        let i = self.names.len() as u32;
        self.names.push(name.to_string());
        self.ids.insert(name.to_string(), i);
        i
    }
}

fn parse_val(ty: &str, text: &str) -> Result<Val, RunError> {
    let bad = || RunError::load(format!("bad literal {ty} {text}"));
    Ok(match ty {
        "I" => Val::I(text.parse().map_err(|_| bad())?),
        "L" => Val::L(text.parse().map_err(|_| bad())?),
        "F" => Val::F(f64::from(text.parse::<f32>().map_err(|_| bad())?)),
        "D" => Val::D(text.parse().map_err(|_| bad())?),
        "S" => {
            if text.len() % 2 != 0 {
                return Err(bad());
            }
            let mut s = Vec::with_capacity(text.len() / 2);
            for i in (0..text.len()).step_by(2) {
                s.push(u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| bad())?);
            }
            Val::S(s)
        }
        _ => return Err(bad()),
    })
}

/// SUB/FUNCTION header: `SUB NAME n P1..Pn [STATIC]`.
struct Header {
    name: String,
    params: Vec<String>,
    is_func: bool,
    is_static: bool,
}

impl Program {
    fn parse(src: &str) -> Result<Program, RunError> {
        let mut lines = src.lines().map(str::trim).filter(|l| !l.is_empty());
        match lines.next() {
            Some(h) if h.starts_with("QIR ") => {}
            _ => return Err(RunError::load("missing QIR header")),
        }
        // Raw instructions with unresolved label operands, split into the main
        // stream and one stream per SUB; SUB bodies are appended after main.
        type Raw = (String, Vec<String>);
        let mut main: Vec<Raw> = Vec::new();
        let mut subs: Vec<(Header, Vec<Raw>)> = Vec::new();
        let mut shared = Vec::new();
        let mut in_sub = false;
        for line in lines {
            let mut parts = line.split(' ').filter(|p| !p.is_empty());
            let op = parts.next().unwrap().to_string();
            let args: Vec<String> = parts.map(str::to_string).collect();
            match op.as_str() {
                "SUB" | "FUNCTION" => {
                    if in_sub || args.len() < 2 {
                        return Err(RunError::load(format!("bad SUB: {line}")));
                    }
                    let n: usize = args[1].parse().map_err(|_| RunError::load(line.to_string()))?;
                    let rest = &args[2..];
                    let is_static = rest.len() == n + 1 && rest[n] == "STATIC";
                    if rest.len() != n && !is_static {
                        return Err(RunError::load(format!("bad SUB params: {line}")));
                    }
                    let h = Header {
                        name: args[0].clone(),
                        params: rest[..n].to_vec(),
                        is_func: op == "FUNCTION",
                        is_static,
                    };
                    subs.push((h, Vec::new()));
                    in_sub = true;
                }
                "ENDSUB" => {
                    if !in_sub {
                        return Err(RunError::load("ENDSUB outside SUB"));
                    }
                    subs.last_mut().unwrap().1.push((op, args));
                    in_sub = false;
                }
                "SHARED" => shared.extend(args),
                _ => {
                    if in_sub {
                        subs.last_mut().unwrap().1.push((op, args));
                    } else {
                        main.push((op, args));
                    }
                }
            }
        }
        if in_sub {
            return Err(RunError::load("SUB without ENDSUB"));
        }
        main.push(("END".into(), vec![]));
        let mut raw = main;
        let mut vars = Interner::default();
        let shared = shared.iter().map(|n: &String| vars.id(n)).collect();
        let mut sub_index = HashMap::new();
        let mut sub_entries = Vec::new();
        for (h, body) in subs {
            if sub_index.insert(h.name.clone(), sub_entries.len()).is_some() {
                return Err(RunError::load(format!("duplicate SUB {}", h.name)));
            }
            let params: Vec<u32> = h.params.iter().map(|p| vars.id(p)).collect();
            let ret = h.is_func.then(|| vars.id(&h.name));
            sub_entries.push((params, ret, h.is_static, raw.len()));
            raw.extend(body);
        }
        let pseudo = |op: &str| op == "LABEL" || op == "DATA";
        let mut data = Vec::new();
        let mut data_labels = HashMap::new();
        for (op, args) in &raw {
            match op.as_str() {
                "DATA" => match parse_val("S", args.first().map_or("", String::as_str))? {
                    Val::S(s) => data.push(s),
                    _ => unreachable!(),
                },
                "LABEL" => {
                    if let Some(n) = args.first() {
                        data_labels.insert(n.clone(), data.len());
                    }
                }
                _ => {}
            }
        }
        let mut labels = HashMap::new();
        let mut code_index = 0usize;
        for (op, args) in &raw {
            if op == "DATA" {
                continue;
            }
            if op == "LABEL" {
                let name = args.first().ok_or_else(|| RunError::load("LABEL without name"))?;
                if labels.insert(name.clone(), code_index).is_some() {
                    return Err(RunError::load(format!("duplicate label {name}")));
                }
            } else {
                code_index += 1;
            }
        }
        // Entry indices must skip LABEL pseudo-ops too.
        let mut remap = Vec::with_capacity(raw.len() + 1);
        let mut idx = 0usize;
        for (op, _) in &raw {
            remap.push(idx);
            if !pseudo(op) {
                idx += 1;
            }
        }
        remap.push(idx);
        let label = |a: &[String]| -> Result<usize, RunError> {
            let n = a.first().ok_or_else(|| RunError::load("missing label operand"))?;
            labels.get(n).copied().ok_or_else(|| RunError::load(format!("unknown label {n}")))
        };
        let arg = |a: &[String], i: usize| -> Result<String, RunError> {
            a.get(i).cloned().ok_or_else(|| RunError::load("missing operand"))
        };
        let num = |a: &[String], i: usize| -> Result<usize, RunError> {
            arg(a, i)?.parse().map_err(|_| RunError::load("bad count"))
        };
        let mut code = Vec::with_capacity(idx);
        for (op, a) in &raw {
            code.push(match op.as_str() {
                "LABEL" | "DATA" => continue,
                "READ" => Op::Read(num(a, 0)? != 0),
                "RESTORE" => Op::Restore(match a.first() {
                    None => 0,
                    Some(n) => *data_labels.get(n).ok_or_else(|| RunError::load(format!("unknown label {n}")))?,
                }),
                "RANDOMIZE" => Op::Randomize(num(a, 0)? != 0),
                "LINE" => Op::Line(arg(a, 0)?.parse().map_err(|_| RunError::load("bad LINE"))?),
                "PUSH" => Op::Push(parse_val(&arg(a, 0)?, a.get(1).map_or("", String::as_str))?),
                "LOAD" => Op::Load(vars.id(&arg(a, 0)?)),
                "STORE" => Op::Store(vars.id(&arg(a, 0)?)),
                "REF" => Op::Ref(vars.id(&arg(a, 0)?)),
                "LOADE" => Op::LoadE(vars.id(&arg(a, 0)?), num(a, 1)?),
                "STOREE" => Op::StoreE(vars.id(&arg(a, 0)?), num(a, 1)?),
                "REFE" => Op::RefE(vars.id(&arg(a, 0)?), num(a, 1)?),
                "DIMA" => Op::DimA(vars.id(&arg(a, 0)?), num(a, 1)?),
                "REDIMA" => Op::RedimA(vars.id(&arg(a, 0)?), num(a, 1)?, num(a, 2)? != 0),
                "BOUND" => Op::Bound(vars.id(&arg(a, 0)?), arg(a, 1)? == "U"),
                "SWAP" => Op::Swap,
                "OP" => Op::Bin(BinOp::parse(&arg(a, 0)?).ok_or_else(|| RunError::load("unknown OP"))?),
                "UN" => Op::Un(match arg(a, 0)?.as_str() {
                    "NEG" => UnOp::Neg,
                    "NOT" => UnOp::Not,
                    _ => return Err(RunError::load("unknown UN")),
                }),
                "FN" => Op::Fn(arg(a, 0)?, num(a, 1)?),
                "PRINT" => Op::Print,
                "PRINTZONE" => Op::PrintZone,
                "PRINTNL" => Op::PrintNl,
                "JMP" => Op::Jmp(label(a)?),
                "JZ" => Op::Jz(label(a)?),
                "JNZ" => Op::Jnz(label(a)?),
                "GOSUB" => Op::Gosub(label(a)?),
                "RETURN" => Op::Return,
                "FORTEST" => Op::ForTest,
                "CALL" => {
                    let n = arg(a, 0)?;
                    let i = *sub_index.get(&n).ok_or_else(|| RunError::load(format!("unknown SUB {n}")))?;
                    Op::Call(i, num(a, 1)?)
                }
                "ENDSUB" => Op::EndSub,
                "ONERROR" => Op::OnError(if arg(a, 0)? == "0" { None } else { Some(label(a)?) }),
                "RESUME" => Op::Resume(match arg(a, 0)?.as_str() {
                    "0" => ResumeKind::Same,
                    "NEXT" => ResumeKind::Next,
                    _ => ResumeKind::Label(label(a)?),
                }),
                "ERROR" => Op::Error,
                "END" => Op::End,
                other => return Err(RunError::load(format!("unknown op {other}"))),
            });
        }
        let subs = sub_entries
            .into_iter()
            .map(|(params, ret, is_static, raw_entry)| SubDef { entry: remap[raw_entry], params, ret, is_static })
            .collect();
        Ok(Program { code, subs, shared, names: vars.names, data })
    }
}

// ---------------------------------------------------------------- machine

struct Frame {
    /// variable id -> place (None until first use)
    vars: Vec<Option<Place>>,
    ret: usize,
    /// cells.len() on entry; locals above it are freed on return
    cell_base: usize,
    sub: Option<usize>,
    /// stack height at the current statement's LINE (error recovery)
    stmt_sp: usize,
}

struct Arr {
    dims: Vec<(i64, i64)>,
    data: Vec<Val>,
}

struct Vm<'a> {
    prog: &'a Program,
    out: &'a mut dyn Write,
    stack: Vec<Val>,
    cells: Vec<Val>,
    heap: Vec<Val>,
    arrays: Vec<Arr>,
    globals: Vec<Option<Place>>,
    statics: Vec<Option<Vec<Option<Place>>>>,
    frames: Vec<Frame>,
    /// frame whose variables LOAD/STORE see (module level while a handler runs)
    view: usize,
    gosub: Vec<(usize, usize)>,
    pc: usize,
    line: u32,
    /// pc of the current statement's LINE marker (RESUME target).
    stmt_pc: usize,
    col: usize,
    on_error: Option<usize>,
    /// Some((pc of failing statement, line, view)) while an error handler runs.
    in_handler: Option<(usize, u32, usize)>,
    err: i32,
    rnd: qb_runtime::rnd::RndState,
    data_ptr: usize,
}

enum Flow {
    Next,
    Stop,
}

impl<'a> Vm<'a> {
    fn new(prog: &'a Program, out: &'a mut dyn Write) -> Self {
        let n = prog.names.len();
        Vm {
            prog,
            out,
            stack: Vec::new(),
            cells: Vec::new(),
            heap: Vec::new(),
            arrays: Vec::new(),
            globals: vec![None; n],
            statics: (0..prog.subs.len()).map(|_| None).collect(),
            frames: vec![Frame { vars: vec![None; n], ret: 0, cell_base: 0, sub: None, stmt_sp: 0 }],
            view: 0,
            gosub: Vec::new(),
            pc: 0,
            line: 0,
            stmt_pc: 0,
            col: 1,
            on_error: None,
            in_handler: None,
            err: 0,
            rnd: qb_runtime::rnd::RndState::default(),
            data_ptr: 0,
        }
    }

    fn run(&mut self) -> Result<(), RunError> {
        let prog = self.prog;
        for &id in &prog.shared {
            let p = self.alloc_heap(id);
            self.globals[id as usize] = Some(p);
        }
        while let Some(op) = prog.code.get(self.pc) {
            self.pc += 1;
            match self.step(op) {
                Ok(Flow::Next) => {}
                Ok(Flow::Stop) => break,
                Err(code) => self.raise(code)?,
            }
        }
        self.out.flush().map_err(|e| RunError { code: 57, line: self.line, msg: e.to_string() })
    }

    /// Error dispatch: jump to the ON ERROR handler or stop the program.
    /// The handler sees module-level variables (QBasic), without unwinding
    /// SUB frames, so RESUME can return into the SUB.
    fn raise(&mut self, code: i32) -> Result<(), RunError> {
        match (self.on_error, self.in_handler) {
            (Some(h), None) => {
                self.err = code;
                self.in_handler = Some((self.stmt_pc, self.line, self.view));
                let sp = self.frames[self.view].stmt_sp;
                self.stack.truncate(sp);
                self.view = 0;
                self.pc = h;
                Ok(())
            }
            _ => Err(RunError { code, line: self.line, msg: format!("error {code}") }),
        }
    }

    fn name(&self, id: u32) -> &'a str {
        let prog: &'a Program = self.prog;
        &prog.names[id as usize]
    }

    fn default_of(&self, id: u32) -> Val {
        suffix_default(self.name(id)).unwrap_or(Val::F(0.0))
    }

    fn alloc_heap(&mut self, id: u32) -> Place {
        self.heap.push(self.default_of(id));
        Place::Heap(self.heap.len() - 1)
    }

    /// New local of the viewed frame; frames that are not the top of the
    /// stack (module level during an error handler) or are STATIC use the heap.
    fn alloc_local(&mut self, id: u32) -> Place {
        let f = &self.frames[self.view];
        let is_static = f.sub.is_some_and(|s| self.prog.subs[s].is_static);
        if is_static || self.view + 1 != self.frames.len() {
            return self.alloc_heap(id);
        }
        self.cells.push(self.default_of(id));
        Place::Cell(self.cells.len() - 1)
    }

    #[inline]
    fn place(&mut self, id: u32) -> Place {
        if let Some(p) = self.frames[self.view].vars[id as usize] {
            return p;
        }
        if let Some(g) = self.globals[id as usize] {
            return g;
        }
        let p = self.alloc_local(id);
        self.frames[self.view].vars[id as usize] = Some(p);
        p
    }

    fn get(&self, p: Place) -> &Val {
        match p {
            Place::Cell(i) => &self.cells[i],
            Place::Heap(i) => &self.heap[i],
            Place::Elem(a, i) => &self.arrays[a].data[i],
        }
    }

    fn get_mut(&mut self, p: Place) -> &mut Val {
        match p {
            Place::Cell(i) => &mut self.cells[i],
            Place::Heap(i) => &mut self.heap[i],
            Place::Elem(a, i) => &mut self.arrays[a].data[i],
        }
    }

    /// Stores with conversion to the type already held at the place.
    fn set(&mut self, p: Place, v: Val) -> Result<(), i32> {
        let slot = self.get_mut(p);
        let nv = convert_like(slot, v)?;
        *slot = nv;
        Ok(())
    }

    fn pop(&mut self) -> Result<Val, i32> {
        self.stack.pop().ok_or(51) // internal error
    }

    fn pop_n(&mut self, n: usize) -> Result<Vec<Val>, i32> {
        if self.stack.len() < n {
            return Err(51);
        }
        Ok(self.stack.split_off(self.stack.len() - n))
    }

    fn emit(&mut self, bytes: &[u8]) -> Result<(), i32> {
        self.out.write_all(bytes).map_err(|_| 57)?;
        for &b in bytes {
            if b == b'\n' {
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------ arrays

    fn array_handle(&mut self, id: u32) -> usize {
        let p = self.place(id);
        match self.get(p) {
            Val::A(h) => *h,
            _ => NO_ARRAY,
        }
    }

    fn new_array(&mut self, id: u32, dims: Vec<(i64, i64)>) -> Result<usize, i32> {
        let mut count: usize = 1;
        for &(lb, ub) in &dims {
            if ub < lb {
                return Err(9);
            }
            count = count.checked_mul((ub - lb + 1) as usize).ok_or(7)?;
        }
        let proto = scalar_default(self.name(id)).unwrap_or(Val::F(0.0));
        self.arrays.push(Arr { dims, data: vec![proto; count] });
        Ok(self.arrays.len() - 1)
    }

    fn pop_dims(&mut self, n: usize) -> Result<Vec<(i64, i64)>, i32> {
        let v = self.pop_n(n * 2)?;
        v.chunks(2).map(|c| Ok((c[0].i()?, c[1].i()?))).collect()
    }

    /// Element place for `n` indices on the stack. Undeclared arrays get
    /// QBasic's implicit `DIM name(10)` per dimension.
    fn elem(&mut self, id: u32, n: usize) -> Result<Place, i32> {
        let idx = self.pop_n(n)?;
        let mut h = self.array_handle(id);
        if h == NO_ARRAY {
            h = self.new_array(id, vec![(0, 10); n])?;
            let p = self.place(id);
            *self.get_mut(p) = Val::A(h);
        }
        let arr = &self.arrays[h];
        if arr.dims.len() != n {
            return Err(9);
        }
        let mut off: usize = 0;
        for (v, &(lb, ub)) in idx.iter().zip(&arr.dims) {
            let i = v.i()?;
            if i < lb || i > ub {
                return Err(9);
            }
            off = off * (ub - lb + 1) as usize + (i - lb) as usize;
        }
        Ok(Place::Elem(h, off))
    }

    fn step(&mut self, op: &Op) -> Result<Flow, i32> {
        match *op {
            Op::Line(n) => {
                self.line = n;
                self.stmt_pc = self.pc - 1;
                let sp = self.stack.len();
                self.frames[self.view].stmt_sp = sp;
            }
            Op::Push(ref v) => self.stack.push(v.clone()),
            Op::Load(id) => {
                let p = self.place(id);
                self.stack.push(self.get(p).clone());
            }
            Op::Store(id) => {
                let v = self.pop()?;
                let p = self.place(id);
                if suffix_default(self.name(id)).is_none() {
                    // untyped compiler temporary: keeps the value's own type
                    *self.get_mut(p) = v;
                } else {
                    self.set(p, v)?;
                }
            }
            Op::Ref(id) => {
                let p = self.place(id);
                self.stack.push(Val::R(p));
            }
            Op::LoadE(id, n) => {
                let p = self.elem(id, n)?;
                self.stack.push(self.get(p).clone());
            }
            Op::StoreE(id, n) => {
                let v = self.pop()?;
                let p = self.elem(id, n)?;
                self.set(p, v)?;
            }
            Op::RefE(id, n) => {
                let p = self.elem(id, n)?;
                self.stack.push(Val::R(p));
            }
            Op::DimA(id, n) => {
                let dims = self.pop_dims(n)?;
                if self.array_handle(id) != NO_ARRAY {
                    return Err(10); // duplicate definition
                }
                let h = self.new_array(id, dims)?;
                let p = self.place(id);
                *self.get_mut(p) = Val::A(h);
            }
            Op::RedimA(id, n, preserve) => {
                let dims = self.pop_dims(n)?;
                let old = self.array_handle(id);
                let h = self.new_array(id, dims)?;
                if preserve && old != NO_ARRAY {
                    // QB64 _PRESERVE keeps elements by linear position
                    let old_data = std::mem::take(&mut self.arrays[old].data);
                    let new = &mut self.arrays[h].data;
                    for (d, s) in new.iter_mut().zip(old_data) {
                        *d = s;
                    }
                }
                let p = self.place(id);
                *self.get_mut(p) = Val::A(h);
            }
            Op::Bound(id, upper) => {
                let d = self.pop()?.i()?;
                let h = self.array_handle(id);
                if h == NO_ARRAY {
                    return Err(9);
                }
                let dims = &self.arrays[h].dims;
                if d < 1 || d as usize > dims.len() {
                    return Err(9);
                }
                let (lb, ub) = dims[d as usize - 1];
                self.stack.push(Val::L(if upper { ub } else { lb } as i32));
            }
            Op::Swap => {
                let (Val::R(b), Val::R(a)) = (self.pop()?, self.pop()?) else { return Err(51) };
                let va = self.get(a).clone();
                let vb = self.get(b).clone();
                if std::mem::discriminant(&va) != std::mem::discriminant(&vb) {
                    return Err(13);
                }
                *self.get_mut(a) = vb;
                *self.get_mut(b) = va;
            }
            Op::Bin(o) => {
                let b = self.pop()?;
                let a = self.pop()?;
                let r = binop(o, a, b)?;
                self.stack.push(r);
            }
            Op::Un(o) => {
                let a = self.pop()?;
                self.stack.push(match o {
                    UnOp::Neg => match a {
                        Val::I(v) => Val::I(v.wrapping_neg()),
                        Val::L(v) => Val::L(v.wrapping_neg()),
                        Val::F(v) => Val::F(-v),
                        Val::D(v) => Val::D(-v),
                        _ => return Err(13),
                    },
                    UnOp::Not => int_result(!a.i()?, a.rank().min(1)),
                });
            }
            Op::Fn(ref name, argc) => {
                let args = self.pop_n(argc)?;
                let r = match name.as_str() {
                    "ERR" => Val::L(self.err),
                    "ERL" => Val::L(0),
                    "RND" => {
                        let n = match args.first() {
                            Some(v) => Some(v.f()? as f32),
                            None => None,
                        };
                        Val::F(f64::from(self.rnd.rnd(n)))
                    }
                    _ => builtins::call(name, args)?,
                };
                self.stack.push(r);
            }
            Op::Print => {
                let v = self.pop()?;
                match v {
                    Val::S(s) => self.emit(&s)?,
                    other => {
                        let mut t = str_of(&other)?;
                        t.push(b' ');
                        self.emit(&t)?;
                    }
                }
            }
            Op::PrintZone => {
                let next = ((self.col - 1) / 14 + 1) * 14 + 1;
                let pad = vec![b' '; next - self.col];
                self.emit(&pad)?;
            }
            Op::PrintNl => self.emit(b"\n")?,
            Op::Jmp(t) => self.pc = t,
            Op::Jz(t) => {
                if !self.pop()?.truth()? {
                    self.pc = t;
                }
            }
            Op::Jnz(t) => {
                if self.pop()?.truth()? {
                    self.pc = t;
                }
            }
            Op::Gosub(t) => {
                self.gosub.push((self.pc, self.frames.len()));
                self.pc = t;
            }
            Op::Return => match self.gosub.last() {
                Some(&(ret, depth)) if depth == self.frames.len() => {
                    self.gosub.pop();
                    self.pc = ret;
                }
                _ => return Err(3),
            },
            Op::ForTest => {
                let step = self.pop()?.f()?;
                let lim = self.pop()?.f()?;
                let v = self.pop()?.f()?;
                let go = if step < 0.0 { v >= lim } else { v <= lim };
                self.stack.push(bool_val(go));
            }
            Op::Call(index, argc) => self.call(index, argc)?,
            Op::EndSub => {
                if self.frames.len() < 2 {
                    return Err(51);
                }
                let f = self.frames.pop().ok_or(51)?;
                let sub = &self.prog.subs[f.sub.ok_or(51)?];
                let result = match sub.ret {
                    Some(r) => Some(match f.vars[r as usize] {
                        Some(p) => self.get(p).clone(),
                        None => self.default_of(r),
                    }),
                    None => None,
                };
                if sub.is_static {
                    let mut keep = f.vars;
                    for &p in &sub.params {
                        keep[p as usize] = None;
                    }
                    if let Some(r) = sub.ret {
                        keep[r as usize] = None;
                    }
                    self.statics[f.sub.unwrap()] = Some(keep);
                }
                self.cells.truncate(f.cell_base);
                // GOSUBs left open inside the SUB die with it
                self.gosub.retain(|&(_, d)| d <= self.frames.len());
                self.view = self.frames.len() - 1;
                self.pc = f.ret;
                if let Some(v) = result {
                    self.stack.push(v);
                }
            }
            Op::OnError(h) => self.on_error = h,
            Op::Resume(ref kind) => {
                let (pc, line, view) = self.in_handler.take().ok_or(20)?; // RESUME without error
                self.line = line;
                self.view = view;
                self.pc = match kind {
                    ResumeKind::Same => pc,
                    ResumeKind::Label(t) => {
                        self.view = 0;
                        *t
                    }
                    ResumeKind::Next => self.next_statement(pc),
                };
            }
            Op::Error => {
                let code = self.pop()?.i()?;
                if !(1..=255).contains(&code) {
                    return Err(5);
                }
                return Err(code as i32);
            }
            Op::Read(is_str) => {
                let item = self.prog.data.get(self.data_ptr).ok_or(4)?.clone(); // out of DATA
                self.data_ptr += 1;
                self.stack.push(if is_str {
                    Val::S(item)
                } else {
                    Val::D(qb_runtime::val::f64_from_f80(qb_runtime::val::val(&item)?))
                });
            }
            Op::Restore(i) => self.data_ptr = i,
            Op::Randomize(using) => {
                let seed = self.pop()?.f()?;
                if using {
                    self.rnd.randomize_using(seed);
                } else {
                    self.rnd.randomize(seed);
                }
            }
            Op::End => return Ok(Flow::Stop),
        }
        Ok(Flow::Next)
    }

    fn call(&mut self, index: usize, argc: usize) -> Result<(), i32> {
        let prog: &'a Program = self.prog;
        let sub = &prog.subs[index];
        if sub.params.len() != argc {
            return Err(450); // wrong number of arguments
        }
        let args = self.pop_n(argc)?;
        let cell_base = self.cells.len();
        let vars = match (sub.is_static, self.statics[index].take()) {
            (true, Some(v)) => v,
            _ => vec![None; prog.names.len()],
        };
        self.frames.push(Frame { vars, ret: self.pc, cell_base, sub: Some(index), stmt_sp: self.stack.len() });
        self.view = self.frames.len() - 1;
        for (&p, a) in sub.params.iter().zip(args) {
            let place = match a {
                Val::R(r) => r,
                v => {
                    let place = self.alloc_local(p);
                    self.set(place, v)?;
                    place
                }
            };
            self.frames[self.view].vars[p as usize] = Some(place);
        }
        if let Some(r) = sub.ret {
            let place = self.alloc_local(r);
            self.frames[self.view].vars[r as usize] = Some(place);
        }
        self.pc = sub.entry;
        Ok(())
    }

    /// pc of the LINE marker after the statement starting at `pc`.
    fn next_statement(&self, pc: usize) -> usize {
        let code = &self.prog.code;
        (pc + 1..code.len()).find(|&i| matches!(code[i], Op::Line(_))).unwrap_or(code.len())
    }
}

fn binop(o: BinOp, a: Val, b: Val) -> Result<Val, i32> {
    use BinOp::*;
    if let (Val::S(x), Val::S(y)) = (&a, &b) {
        return Ok(match o {
            Add => Val::S([x.as_slice(), y.as_slice()].concat()),
            Eq => bool_val(x == y),
            Ne => bool_val(x != y),
            Lt => bool_val(x < y),
            Le => bool_val(x <= y),
            Gt => bool_val(x > y),
            Ge => bool_val(x >= y),
            _ => return Err(13),
        });
    }
    if a.rank() > 3 || b.rank() > 3 {
        return Err(13);
    }
    let rank = a.rank().max(b.rank());
    let ints = a.is_int() && b.is_int();
    Ok(match o {
        Add | Sub | Mul if ints => {
            let (x, y) = (a.i()?, b.i()?);
            int_result(
                match o {
                    Add => x + y,
                    Sub => x - y,
                    _ => x * y,
                },
                rank,
            )
        }
        Add => float_result(a.f()? + b.f()?, rank),
        Sub => float_result(a.f()? - b.f()?, rank),
        Mul => float_result(a.f()? * b.f()?, rank),
        Div => {
            let y = b.f()?;
            if y == 0.0 {
                return Err(11);
            }
            float_result(a.f()? / y, rank)
        }
        Pow => float_result(a.f()?.powf(b.f()?), rank),
        IDiv | Mod => {
            let (x, y) = (a.i()?, b.i()?);
            if y == 0 {
                return Err(11);
            }
            let r = if matches!(o, IDiv) { x / y } else { x % y };
            int_result(r, rank.min(1))
        }
        And => int_result(a.i()? & b.i()?, rank.min(1)),
        Or => int_result(a.i()? | b.i()?, rank.min(1)),
        Xor => int_result(a.i()? ^ b.i()?, rank.min(1)),
        Eqv => int_result(!(a.i()? ^ b.i()?), rank.min(1)),
        Imp => int_result(!a.i()? | b.i()?, rank.min(1)),
        Eq => bool_val(a.f()? == b.f()?),
        Ne => bool_val(a.f()? != b.f()?),
        Lt => bool_val(a.f()? < b.f()?),
        Le => bool_val(a.f()? <= b.f()?),
        Gt => bool_val(a.f()? > b.f()?),
        Ge => bool_val(a.f()? >= b.f()?),
    })
}

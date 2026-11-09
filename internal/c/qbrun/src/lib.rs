//! `qbrun` — R11 spike: executes QIR, the IR the BASIC compiler emits with
//! `--ir` (see docs/rust-port/r11-emitter-design.md for the format).
//!
//! The program is a flat instruction list. Labels are resolved to indices at
//! load time, so GOTO/GOSUB/RESUME/ON ERROR are plain jumps — the constructs
//! that make a Rust source backend hard (no `goto` in Rust) are trivial here.
//!
//! Numbers keep the QBasic type tag of the expression (INTEGER, LONG, SINGLE,
//! DOUBLE). SINGLE values are held at double precision until they are stored
//! or printed, matching what the C++ backend does (`y# = 1 / 3` keeps 53 bits).
//! Formatting and rounding come from `qbrt`, the same code the C++ path links.

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

#[derive(Debug, Clone, PartialEq)]
enum Val {
    I(i16),
    L(i32),
    /// SINGLE, carried at double precision (see module docs).
    F(f64),
    D(f64),
    S(Vec<u8>),
    /// Reference to a variable cell (BYREF argument).
    R(usize),
}

impl Val {
    fn rank(&self) -> u8 {
        match self {
            Val::I(_) => 0,
            Val::L(_) => 1,
            Val::F(_) => 2,
            Val::D(_) => 3,
            Val::S(_) | Val::R(_) => 9,
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
    /// Integer value; floats round half-even like QBasic (`qbrt` rounding).
    fn i(&self) -> Result<i64, i32> {
        match self {
            Val::I(v) => Ok(i64::from(*v)),
            Val::L(v) => Ok(i64::from(*v)),
            Val::F(v) | Val::D(v) => Ok(qbrt::round::qbrt_qbr_f64(*v)),
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

/// Type a variable gets from its name suffix; `None` = untyped compiler temp.
fn suffix_default(name: &str) -> Option<Val> {
    match name.as_bytes().last()? {
        b'%' => Some(Val::I(0)),
        b'&' => Some(Val::L(0)),
        b'!' => Some(Val::F(0.0)),
        b'#' => Some(Val::D(0.0)),
        b'$' => Some(Val::S(Vec::new())),
        _ => None,
    }
}

/// Converts `v` for storage into a variable with the given name.
fn convert_for(name: &str, v: Val) -> Result<Val, i32> {
    let Some(proto) = suffix_default(name) else { return Ok(v) };
    Ok(match proto {
        Val::I(_) => Val::I(v.i()? as i16),
        Val::L(_) => Val::L(v.i()? as i32),
        Val::F(_) => Val::F(f64::from(v.f()? as f32)),
        Val::D(_) => Val::D(v.f()?),
        Val::S(_) => Val::S(v.s()?),
        Val::R(_) => unreachable!(),
    })
}

fn str_of(v: &Val) -> Result<Vec<u8>, i32> {
    Ok(match v {
        Val::I(x) => qbrt::numfmt::str_i64(i64::from(*x)),
        Val::L(x) => qbrt::numfmt::str_i64(i64::from(*x)),
        Val::F(x) => qbrt::numfmt::str_f32(*x as f32),
        Val::D(x) => qbrt::numfmt::str_f64(*x),
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
    Bin(BinOp),
    Un(UnOp),
    Fn(String, usize),
    Print,
    PrintZone,
    PrintNl,
    Jmp(usize),
    Jz(usize),
    Gosub(usize),
    Return,
    ForTest,
    /// sub index, argc
    Call(usize, usize),
    EndSub,
    OnError(Option<usize>),
    Resume(ResumeKind),
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
}

/// Loaded program. Variable names are interned to dense ids at load time, so
/// LOAD/STORE index a per-frame slot table instead of hashing names.
struct Program {
    code: Vec<Op>,
    subs: Vec<SubDef>,
    shared: Vec<u32>,
    names: Vec<String>,
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
        let mut subs: Vec<(String, Vec<String>, Vec<Raw>)> = Vec::new();
        let mut shared = Vec::new();
        let mut in_sub = false;
        for line in lines {
            let mut parts = line.split(' ').filter(|p| !p.is_empty());
            let op = parts.next().unwrap().to_string();
            let args: Vec<String> = parts.map(str::to_string).collect();
            match op.as_str() {
                "SUB" => {
                    if in_sub || args.len() < 2 {
                        return Err(RunError::load(format!("bad SUB: {line}")));
                    }
                    let n: usize = args[1].parse().map_err(|_| RunError::load(line.to_string()))?;
                    if args.len() != 2 + n {
                        return Err(RunError::load(format!("bad SUB params: {line}")));
                    }
                    subs.push((args[0].clone(), args[2..].to_vec(), Vec::new()));
                    in_sub = true;
                }
                "ENDSUB" => {
                    if !in_sub {
                        return Err(RunError::load("ENDSUB outside SUB"));
                    }
                    subs.last_mut().unwrap().2.push((op, args));
                    in_sub = false;
                }
                "SHARED" => shared.extend(args),
                _ => {
                    if in_sub {
                        subs.last_mut().unwrap().2.push((op, args));
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
        for (name, params, body) in subs {
            if sub_index.insert(name.clone(), sub_entries.len()).is_some() {
                return Err(RunError::load(format!("duplicate SUB {name}")));
            }
            let params: Vec<u32> = params.iter().map(|p| vars.id(p)).collect();
            sub_entries.push((params, raw.len()));
            raw.extend(body);
        }
        let mut labels = HashMap::new();
        let mut code_index = 0usize;
        for (op, args) in &raw {
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
            if op != "LABEL" {
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
                "LABEL" => continue,
                "LINE" => Op::Line(arg(a, 0)?.parse().map_err(|_| RunError::load("bad LINE"))?),
                "PUSH" => Op::Push(parse_val(&arg(a, 0)?, a.get(1).map_or("", String::as_str))?),
                "LOAD" => Op::Load(vars.id(&arg(a, 0)?)),
                "STORE" => Op::Store(vars.id(&arg(a, 0)?)),
                "REF" => Op::Ref(vars.id(&arg(a, 0)?)),
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
                "END" => Op::End,
                other => return Err(RunError::load(format!("unknown op {other}"))),
            });
        }
        let subs = sub_entries
            .into_iter()
            .map(|(params, raw_entry)| SubDef { entry: remap[raw_entry], params })
            .collect();
        Ok(Program { code, subs, shared, names: vars.names })
    }
}

// ---------------------------------------------------------------- machine

const NONE: usize = usize::MAX;

struct Frame {
    /// variable id -> cell index (NONE until first use)
    vars: Vec<usize>,
    ret: usize,
    /// cells.len() on entry; locals above it are freed on return
    cell_base: usize,
}

struct Vm<'a> {
    prog: &'a Program,
    out: &'a mut dyn Write,
    stack: Vec<Val>,
    cells: Vec<Val>,
    globals: Vec<usize>,
    frames: Vec<Frame>,
    gosub: Vec<(usize, usize)>,
    pc: usize,
    line: u32,
    /// pc of the current statement's LINE marker (RESUME target).
    stmt_pc: usize,
    col: usize,
    on_error: Option<usize>,
    /// Some((pc of failing statement, line)) while an error handler runs.
    in_handler: Option<(usize, u32)>,
    err: i32,
}

enum Flow {
    Next,
    Stop,
}

impl<'a> Vm<'a> {
    fn new(prog: &'a Program, out: &'a mut dyn Write) -> Self {
        Vm {
            prog,
            out,
            stack: Vec::new(),
            cells: Vec::new(),
            globals: vec![NONE; prog.names.len()],
            frames: vec![Frame { vars: vec![NONE; prog.names.len()], ret: 0, cell_base: 0 }],
            gosub: Vec::new(),
            pc: 0,
            line: 0,
            stmt_pc: 0,
            col: 1,
            on_error: None,
            in_handler: None,
            err: 0,
        }
    }

    fn run(&mut self) -> Result<(), RunError> {
        let prog = self.prog;
        for &id in &prog.shared {
            let c = self.alloc(id);
            self.globals[id as usize] = c;
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
    fn raise(&mut self, code: i32) -> Result<(), RunError> {
        match (self.on_error, self.in_handler) {
            (Some(h), None) => {
                self.err = code;
                self.in_handler = Some((self.stmt_pc, self.line));
                self.stack.clear();
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

    fn alloc(&mut self, id: u32) -> usize {
        self.cells.push(suffix_default(self.name(id)).unwrap_or(Val::F(0.0)));
        self.cells.len() - 1
    }

    #[inline]
    fn cell(&mut self, id: u32) -> usize {
        let c = self.frames.last().unwrap().vars[id as usize];
        if c != NONE {
            return c;
        }
        let g = self.globals[id as usize];
        if g != NONE {
            return g;
        }
        let c = self.alloc(id);
        self.frames.last_mut().unwrap().vars[id as usize] = c;
        c
    }

    fn pop(&mut self) -> Result<Val, i32> {
        self.stack.pop().ok_or(51) // internal error
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

    fn step(&mut self, op: &Op) -> Result<Flow, i32> {
        match *op {
            Op::Line(n) => {
                self.line = n;
                self.stmt_pc = self.pc - 1;
            }
            Op::Push(ref v) => self.stack.push(v.clone()),
            Op::Load(id) => {
                let c = self.cell(id);
                self.stack.push(self.cells[c].clone());
            }
            Op::Store(id) => {
                let v = self.pop()?;
                let c = self.cell(id);
                self.cells[c] = convert_for(self.name(id), v)?;
            }
            Op::Ref(id) => {
                let c = self.cell(id);
                self.stack.push(Val::R(c));
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
                if self.stack.len() < argc {
                    return Err(51);
                }
                let args = self.stack.split_off(self.stack.len() - argc);
                let r = self.builtin(name, args)?;
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
            Op::Call(index, argc) => {
                let prog: &'a Program = self.prog;
                let sub = &prog.subs[index];
                if sub.params.len() != argc || self.stack.len() < argc {
                    return Err(450); // wrong number of arguments
                }
                let args = self.stack.split_off(self.stack.len() - argc);
                let cell_base = self.cells.len();
                let mut vars = vec![NONE; prog.names.len()];
                for (&p, a) in sub.params.iter().zip(args) {
                    let c = match a {
                        Val::R(c) => c,
                        v => {
                            let c = self.alloc(p);
                            self.cells[c] = convert_for(self.name(p), v)?;
                            c
                        }
                    };
                    vars[p as usize] = c;
                }
                self.frames.push(Frame { vars, ret: self.pc, cell_base });
                self.pc = sub.entry;
            }
            Op::EndSub => {
                if self.frames.len() < 2 {
                    return Err(51);
                }
                let f = self.frames.pop().ok_or(51)?;
                self.cells.truncate(f.cell_base);
                // GOSUBs left open inside the SUB die with it
                self.gosub.retain(|&(_, d)| d <= self.frames.len());
                self.pc = f.ret;
            }
            Op::OnError(h) => self.on_error = h,
            Op::Resume(ref kind) => {
                let (pc, line) = self.in_handler.take().ok_or(20)?; // RESUME without error
                self.line = line;
                self.pc = match kind {
                    ResumeKind::Same => pc,
                    ResumeKind::Label(t) => *t,
                    ResumeKind::Next => self.next_statement(pc),
                };
            }
            Op::End => return Ok(Flow::Stop),
        }
        Ok(Flow::Next)
    }

    /// pc of the LINE marker after the statement starting at `pc`.
    fn next_statement(&self, pc: usize) -> usize {
        let code = &self.prog.code;
        (pc + 1..code.len()).find(|&i| matches!(code[i], Op::Line(_))).unwrap_or(code.len())
    }

    fn builtin(&mut self, name: &str, mut a: Vec<Val>) -> Result<Val, i32> {
        let argc = a.len();
        let len_of = |v: &Val| -> Result<usize, i32> {
            let n = v.i()?;
            if n < 0 {
                Err(5)
            } else {
                Ok(n as usize)
            }
        };
        Ok(match (name, argc) {
            ("LEN", 1) => Val::L(a.pop().unwrap().s()?.len() as i32),
            ("LEFT$", 2) => {
                let n = len_of(&a[1])?;
                let s = a.swap_remove(0).s()?;
                Val::S(s[..n.min(s.len())].to_vec())
            }
            ("RIGHT$", 2) => {
                let n = len_of(&a[1])?;
                let s = a.swap_remove(0).s()?;
                Val::S(s[s.len() - n.min(s.len())..].to_vec())
            }
            ("MID$", 2) | ("MID$", 3) => {
                let start = a[1].i()?;
                if start < 1 {
                    return Err(5);
                }
                let n = if argc == 3 { Some(len_of(&a[2])?) } else { None };
                let s = a.swap_remove(0).s()?;
                let from = (start as usize - 1).min(s.len());
                let to = n.map_or(s.len(), |n| (from + n).min(s.len()));
                Val::S(s[from..to].to_vec())
            }
            ("UCASE$", 1) => Val::S(a.pop().unwrap().s()?.to_ascii_uppercase()),
            ("LCASE$", 1) => Val::S(a.pop().unwrap().s()?.to_ascii_lowercase()),
            ("STR$", 1) => Val::S(str_of(&a[0])?),
            ("CHR$", 1) => {
                let c = a[0].i()?;
                if !(0..=255).contains(&c) {
                    return Err(5);
                }
                Val::S(vec![c as u8])
            }
            ("ASC", 1) => {
                let s = a.pop().unwrap().s()?;
                Val::L(i32::from(*s.first().ok_or(5)?))
            }
            ("ERR", 0) => Val::L(self.err),
            ("ERL", 0) => Val::L(0),
            _ => return Err(51),
        })
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
    if matches!(a, Val::S(_)) || matches!(b, Val::S(_)) {
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
        Eq => bool_val(a.f()? == b.f()?),
        Ne => bool_val(a.f()? != b.f()?),
        Lt => bool_val(a.f()? < b.f()?),
        Le => bool_val(a.f()? <= b.f()?),
        Gt => bool_val(a.f()? > b.f()?),
        Ge => bool_val(a.f()? >= b.f()?),
    })
}

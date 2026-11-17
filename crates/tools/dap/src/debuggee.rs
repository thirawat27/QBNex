//! The debuggee: a child process (`qb-dap --debuggee <program.qir> <port>`)
//! that runs the program on the qb-exec VM. Its stdin/stdout/stderr belong to
//! the program; it talks to the adapter over a loopback TCP connection with
//! one JSON object per line:
//!
//! - adapter -> debuggee: `{"t":"bp","at":[[file,line],..]}`, `{"t":"pause"}`,
//!   `{"t":"resume","mode":"continue"|"in"|"over"|"out"}`
//! - debuggee -> adapter: `{"t":"stopped", ...Snapshot}`
//!
//! A place is a file tag ([`crate::snapshot::FileTag`]: the program's own file
//! or one of its `$INCLUDE` files) and a line in that file.
//!
//! The first line the debuggee sends is the token the adapter put in its
//! environment ([`crate::adapter::TOKEN_VAR`]): the adapter takes no other
//! connection for the debuggee.

use std::collections::HashSet;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

use qb_exec::debug::{Debugger, Stop};
use serde_json::{Value, json};

use crate::snapshot::{FileTag, Frame, MAIN_FILE, Snapshot, Var};

/// A place in the source: (file, line in that file).
pub type Loc = (FileTag, u32);

/// Where each statement of the program is.
#[derive(Clone, Debug, Default)]
pub struct Places {
    /// (pc, line as the VM counts it, place), sorted by pc
    stmts: Vec<(u32, u32, Loc)>,
}

impl Places {
    /// From the bytecode's statement table and its `$INCLUDE` entries
    /// (`Module::stmts`, `Module::incls`). The VM counts the lines of an
    /// included statement as the line of its `$INCLUDE`; the include entry
    /// names the file and the line there.
    pub fn new(stmts: &[(u32, u32)], incls: &[(u32, u32, u32)]) -> Places {
        let mut v: Vec<(u32, u32, Loc)> = stmts.iter().map(|&(pc, line)| (pc, line, (MAIN_FILE, line))).collect();
        for &(k, line, file) in incls {
            if let Some(s) = v.get_mut(k as usize) {
                s.2 = (file.saturating_add(1), line);
            }
        }
        Places { stmts: v }
    }

    /// (line as the VM counts it, place) of the statement that starts at `pc`.
    /// Several statements can start at one pc (lines without code); the VM
    /// takes the last of them, and so does this.
    pub fn at(&self, pc: u32) -> Option<(u32, Loc)> {
        let i = self.stmts.partition_point(|s| s.0 <= pc).checked_sub(1)?;
        let s = self.stmts[i];
        (s.0 == pc).then_some((s.1, s.2))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Run,
    /// stop at the next statement
    In { pc: u32, depth: usize },
    /// next statement in this frame or a caller
    Over { pc: u32, depth: usize },
    /// next statement in a caller
    Out { depth: usize },
}

/// Decides where to stop; separate from I/O so it can be unit-tested.
#[derive(Debug)]
pub struct Stepper {
    pub mode: Mode,
    pub stop_on_entry: bool,
    prev: Option<(Loc, usize)>,
}

impl Stepper {
    pub fn new(stop_on_entry: bool) -> Self {
        Stepper { mode: Mode::Run, stop_on_entry, prev: None }
    }

    /// Reason to stop at this statement (at place `at`), if any.
    pub fn check(&mut self, pc: u32, at: Loc, depth: usize, pause: bool, breakpoints: &HashSet<Loc>) -> Option<&'static str> {
        let first = self.prev.is_none();
        let moved = self.prev != Some((at, depth));
        self.prev = Some((at, depth));
        if pause {
            return Some("pause");
        }
        if first && self.stop_on_entry {
            return Some("entry");
        }
        let step = match self.mode {
            Mode::Run => false,
            Mode::In { pc: p, depth: d } => pc != p || depth != d,
            Mode::Over { pc: p, depth: d } => depth < d || (depth == d && pc != p),
            Mode::Out { depth: d } => depth < d,
        };
        if step {
            return Some("step");
        }
        if moved && breakpoints.contains(&at) {
            return Some("breakpoint");
        }
        None
    }

    pub fn resume(&mut self, mode: &str, pc: u32, depth: usize) {
        self.mode = match mode {
            "in" => Mode::In { pc, depth },
            "over" => Mode::Over { pc, depth },
            "out" => Mode::Out { depth },
            _ => Mode::Run,
        };
    }
}

/// Statement start of no statement (a frame whose first statement has not run).
const NO_PC: u32 = u32::MAX;

/// Keeps, for each frame on the call stack, the start of the statement it
/// last began. The VM tells only the innermost frame's statement; a caller's
/// is the statement that made the call, which began in that frame before.
#[derive(Debug, Default)]
pub struct Trail {
    /// index = depth - 1
    pcs: Vec<u32>,
}

impl Trail {
    /// A statement at `pc` begins in the frame at `depth` (1 = module level).
    pub fn enter(&mut self, pc: u32, depth: usize) {
        self.pcs.truncate(depth);
        self.pcs.resize(depth, NO_PC);
        if let Some(last) = self.pcs.last_mut() {
            *last = pc;
        }
    }

    /// Place of the current statement of the frame at `depth`, when it is the
    /// statement at `vm_line` (the line the VM gives for that frame); else
    /// that line in the program's own file.
    pub fn place(&self, places: &Places, depth: usize, vm_line: u32) -> Loc {
        depth
            .checked_sub(1)
            .and_then(|i| self.pcs.get(i))
            .filter(|&&pc| pc != NO_PC)
            .and_then(|&pc| places.at(pc))
            .filter(|(line, _)| *line == vm_line)
            .map_or((MAIN_FILE, vm_line), |(_, at)| at)
    }
}

struct Hook {
    stepper: Stepper,
    breakpoints: Arc<Mutex<HashSet<Loc>>>,
    pause: Arc<AtomicBool>,
    resume: Receiver<String>,
    out: TcpStream,
    places: Places,
    trail: Trail,
}

impl Debugger for Hook {
    fn statement(&mut self, stop: &mut Stop<'_, '_, '_>) {
        let depth = stop.depth();
        self.trail.enter(stop.pc, depth);
        let at = self.places.at(stop.pc).map_or((MAIN_FILE, stop.line), |(_, at)| at);
        let pause = self.pause.swap(false, Ordering::Relaxed);
        // (the set is looked at under its lock: a copy per statement made every
        // statement of a program with a breakpoint cost an allocation)
        let reason = self.stepper.check(stop.pc, at, depth, pause, &self.breakpoints.lock().unwrap());
        let Some(reason) = reason else { return };
        self.stepper.mode = Mode::Run;
        stop.flush();
        let _ = std::io::stdout().flush();
        let stack = stop.stack();
        let mut frames = Vec::with_capacity(stack.len());
        for (i, f) in stack.iter().enumerate() {
            // stack entry i (innermost first) is the frame at depth `depth - i`
            let (file, line) = if i == 0 { at } else { self.trail.place(&self.places, depth - i, f.line) };
            frames.push(Frame { func: f.func.clone(), line, file, locals: stop.locals(i).into_iter().map(Var::from).collect() });
        }
        let snap = Snapshot {
            reason: reason.into(),
            line: at.1,
            file: at.0,
            frames,
            globals: stop.globals().into_iter().map(Var::from).collect(),
        };
        let mut msg = serde_json::to_value(&snap).unwrap();
        msg["t"] = json!("stopped");
        // a resume sent while the program was running does not apply to this stop
        while self.resume.try_recv().is_ok() {}
        if writeln!(self.out, "{msg}").is_err() {
            std::process::exit(0);
        }
        match self.resume.recv() {
            Ok(mode) => self.stepper.resume(&mode, stop.pc, depth),
            // the adapter went away
            Err(_) => std::process::exit(0),
        }
    }
}

/// How the adapter starts the debuggee.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub qir: PathBuf,
    pub port: u16,
    pub token: String,
    pub no_debug: bool,
    pub stop_on_entry: bool,
    /// The program's argv as BASIC sees it (COMMAND$, _COMMANDCOUNT):
    /// [program, args...]. Without it the program would see the debuggee's
    /// own command line.
    pub program_args: Option<Vec<OsString>>,
    /// Where the program's exit code is written for the adapter: a process
    /// exit status cannot carry every value `SYSTEM n` can (on Unix it keeps
    /// only the low 8 bits).
    pub exit_file: Option<PathBuf>,
}

/// Entry point of the debuggee process. Returns the exit code.
pub fn main(o: &Options) -> i32 {
    // before anything reads the arguments: the first setting is the one kept
    if let Some(a) = &o.program_args {
        qb_exec::set_program_args(a.clone());
    }
    let code = run(o);
    if let Some(f) = &o.exit_file {
        let _ = std::fs::write(f, code.to_string());
    }
    code
}

fn run(o: &Options) -> i32 {
    let (port, token, no_debug, stop_on_entry) = (o.port, o.token.as_str(), o.no_debug, o.stop_on_entry);
    let qir = match std::fs::read_to_string(&o.qir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("qb-dap: cannot read {}: {e}", o.qir.display());
            return 2;
        }
    };
    let src = match qb_ir::parse(&qir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("qb-dap: {e}");
            return 2;
        }
    };
    let module = match qb_exec::lower(&src) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("qb-dap: {}", e.msg);
            return 2;
        }
    };
    let mut stdout = std::io::stdout();
    let mut vm = qb_exec::Vm::new(&module, &mut stdout);
    if !no_debug {
        let Ok(mut conn) = TcpStream::connect(("127.0.0.1", port)) else {
            eprintln!("qb-dap: cannot reach the adapter on port {port}");
            return 2;
        };
        if writeln!(conn, "{token}").is_err() {
            eprintln!("qb-dap: cannot reach the adapter on port {port}");
            return 2;
        }
        let breakpoints = Arc::new(Mutex::new(HashSet::new()));
        let pause = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        // Breakpoints arrive before the program starts: read until "go".
        let mut reader = BufReader::new(conn.try_clone().unwrap());
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            let msg: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
            line.clear();
            match msg["t"].as_str() {
                Some("bp") => set_breakpoints(&breakpoints, &msg),
                Some("go") => break,
                _ => {}
            }
        }
        {
            let (breakpoints, pause) = (breakpoints.clone(), pause.clone());
            std::thread::spawn(move || {
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap_or(0) > 0 {
                    let msg: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
                    line.clear();
                    match msg["t"].as_str() {
                        Some("bp") => set_breakpoints(&breakpoints, &msg),
                        Some("pause") => pause.store(true, Ordering::Relaxed),
                        Some("resume") => {
                            let _ = tx.send(msg["mode"].as_str().unwrap_or("continue").to_string());
                        }
                        _ => {}
                    }
                }
                // adapter closed the connection
                std::process::exit(0);
            });
        }
        let places = Places::new(&module.stmts, &module.incls);
        let hook = Hook { stepper: Stepper::new(stop_on_entry), breakpoints, pause, resume: rx, out: conn, places, trail: Trail::default() };
        if let Err(e) = vm.attach_debugger(&src, Box::new(hook)) {
            eprintln!("qb-dap: {e}");
            return 2;
        }
    }
    let r = vm.run_status();
    drop(vm);
    let _ = stdout.flush();
    match r {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Unhandled error #{} on line {}", e.code, e.line);
            1
        }
    }
}

/// `{"t":"bp","at":[[file,line],..]}`: the whole set of breakpoint places.
fn set_breakpoints(bps: &Mutex<HashSet<Loc>>, msg: &Value) {
    *bps.lock().unwrap() = breakpoint_places(msg);
}

fn breakpoint_places(msg: &Value) -> HashSet<Loc> {
    let place = |p: &Value| Some((u32::try_from(p[0].as_u64()?).ok()?, u32::try_from(p[1].as_u64()?).ok()?));
    msg["at"].as_array().map(|a| a.iter().filter_map(place).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: FileTag = MAIN_FILE;

    #[test]
    fn stepping_rules() {
        let none = HashSet::new();
        let bp: HashSet<Loc> = [(M, 5)].into();
        let mut s = Stepper::new(true);
        assert_eq!(s.check(0, (M, 1), 1, false, &none), Some("entry"));
        assert_eq!(s.check(1, (M, 2), 1, false, &none), None);
        // breakpoint once per arrival on the line
        assert_eq!(s.check(2, (M, 5), 1, false, &bp), Some("breakpoint"));
        assert_eq!(s.check(3, (M, 5), 1, false, &bp), None);
        // over: skips deeper frames, stops back in this frame
        s.resume("over", 3, 1);
        assert_eq!(s.check(10, (M, 20), 2, false, &none), None);
        assert_eq!(s.check(4, (M, 6), 1, false, &none), Some("step"));
        s.resume("in", 4, 1);
        assert_eq!(s.check(10, (M, 20), 2, false, &none), Some("step"));
        s.resume("out", 10, 2);
        assert_eq!(s.check(11, (M, 21), 2, false, &none), None);
        assert_eq!(s.check(5, (M, 7), 1, false, &none), Some("step"));
        s.resume("continue", 5, 1);
        assert_eq!(s.check(6, (M, 8), 1, true, &none), Some("pause"));
    }

    /// The statements of an `$INCLUDE` file all carry the `$INCLUDE` line for
    /// the VM; their own file and line tell them apart, so a breakpoint in the
    /// included file stops on its line, and one on the `$INCLUDE` line does not.
    #[test]
    fn breakpoints_in_included_files() {
        let inc: FileTag = 1;
        let bp: HashSet<Loc> = [(inc, 3)].into();
        let mut s = Stepper::new(false);
        assert_eq!(s.check(0, (M, 1), 1, false, &bp), None);
        assert_eq!(s.check(1, (inc, 2), 1, false, &bp), None);
        assert_eq!(s.check(2, (inc, 3), 1, false, &bp), Some("breakpoint"));
        let on_include_line: HashSet<Loc> = [(M, 2)].into();
        let mut s = Stepper::new(false);
        assert_eq!(s.check(1, (inc, 2), 1, false, &on_include_line), None);
    }

    #[test]
    fn places_of_statements() {
        // pc 0: line 1; pc 4: line 2 (`$INCLUDE` line) = inc.bi line 7; pc 4 again:
        // a second statement at the same pc (the VM takes the last one)
        let stmts = [(0, 1), (4, 2), (4, 2), (9, 3)];
        let incls = [(2, 7, 0)];
        let p = Places::new(&stmts, &incls);
        assert_eq!(p.at(0), Some((1, (M, 1))));
        assert_eq!(p.at(4), Some((2, (1, 7))));
        assert_eq!(p.at(9), Some((3, (M, 3))));
        assert_eq!(p.at(5), None);
        assert_eq!(p.at(100), None);
    }

    /// A caller frame's place is the statement that made the call, from the
    /// trail; when the trail does not match the VM's line, the VM's line in the
    /// program's file.
    #[test]
    fn caller_frames_from_the_trail() {
        let stmts = [(0, 1), (4, 2), (20, 30)];
        let incls = [(1, 5, 0)];
        let p = Places::new(&stmts, &incls);
        let mut t = Trail::default();
        t.enter(0, 1);
        t.enter(4, 1);
        // a SUB called from the included statement at pc 4
        t.enter(20, 2);
        assert_eq!(t.place(&p, 1, 2), (1, 5));
        assert_eq!(t.place(&p, 1, 9), (M, 9));
        assert_eq!(t.place(&p, 3, 9), (M, 9));
        // back in the module: the SUB's entry is gone
        t.enter(0, 1);
        assert_eq!(t.place(&p, 2, 30), (M, 30));
    }

    #[test]
    fn breakpoint_message() {
        let m = json!({"t": "bp", "at": [[0, 5], [2, 7], ["x", 1], [1], 3]});
        let set = breakpoint_places(&m);
        assert_eq!(set, HashSet::from([(0, 5), (2, 7)]));
        assert!(breakpoint_places(&json!({"t": "bp"})).is_empty());
    }

    #[test]
    fn exit_code_goes_to_the_exit_file() {
        let dir = std::env::temp_dir().join(format!("qb-dap-exitfile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let qir = dir.join("p.qir");
        std::fs::write(&qir, "QIR 1\nmeta console only\nfunc main\nline 1\nconst.i32 %0 300\ncvt.i64.i32 %1 %0\nsystem %1\nend\nendfunc\n").unwrap();
        let exit_file = dir.join("exit-code");
        let o = Options { qir, no_debug: true, exit_file: Some(exit_file.clone()), ..Options::default() };
        assert_eq!(main(&o), 300);
        assert_eq!(std::fs::read_to_string(&exit_file).unwrap(), "300");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

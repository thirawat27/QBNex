//! The debuggee: a child process (`qb-dap --debuggee <program.qir> <port>`)
//! that runs the program on the qb-exec VM. Its stdin/stdout/stderr belong to
//! the program; it talks to the adapter over a loopback TCP connection with
//! one JSON object per line:
//!
//! - adapter -> debuggee: `{"t":"bp","lines":[..]}`, `{"t":"pause"}`,
//!   `{"t":"resume","mode":"continue"|"in"|"over"|"out"}`
//! - debuggee -> adapter: `{"t":"stopped", ...Snapshot}`

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

use qb_exec::debug::{Debugger, Stop};
use serde_json::{Value, json};

use crate::snapshot::{Frame, Snapshot, Var};

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
    prev: Option<(u32, usize)>,
}

impl Stepper {
    pub fn new(stop_on_entry: bool) -> Self {
        Stepper { mode: Mode::Run, stop_on_entry, prev: None }
    }

    /// Reason to stop at this statement, if any.
    pub fn check(&mut self, pc: u32, line: u32, depth: usize, pause: bool, breakpoints: &HashSet<u32>) -> Option<&'static str> {
        let first = self.prev.is_none();
        let moved = self.prev != Some((line, depth));
        self.prev = Some((line, depth));
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
        if moved && breakpoints.contains(&line) {
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

struct Hook {
    stepper: Stepper,
    breakpoints: Arc<Mutex<HashSet<u32>>>,
    pause: Arc<AtomicBool>,
    resume: Receiver<String>,
    out: TcpStream,
}

impl Debugger for Hook {
    fn statement(&mut self, stop: &mut Stop<'_, '_, '_>) {
        let depth = stop.depth();
        let pause = self.pause.swap(false, Ordering::Relaxed);
        let bps = self.breakpoints.lock().unwrap().clone();
        let Some(reason) = self.stepper.check(stop.pc, stop.line, depth, pause, &bps) else { return };
        self.stepper.mode = Mode::Run;
        stop.flush();
        let _ = std::io::stdout().flush();
        let stack = stop.stack();
        let frames = stack
            .iter()
            .enumerate()
            .map(|(i, f)| Frame { func: f.func.clone(), line: f.line, locals: stop.locals(i).into_iter().map(Var::from).collect() })
            .collect();
        let snap = Snapshot {
            reason: reason.into(),
            line: stop.line,
            frames,
            globals: stop.globals().into_iter().map(Var::from).collect(),
        };
        let mut msg = serde_json::to_value(&snap).unwrap();
        msg["t"] = json!("stopped");
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

/// Entry point of the debuggee process. Returns the exit code.
pub fn main(qir_path: &str, port: u16, no_debug: bool, stop_on_entry: bool) -> i32 {
    let qir = match std::fs::read_to_string(qir_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("qb-dap: cannot read {qir_path}: {e}");
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
        let Ok(conn) = TcpStream::connect(("127.0.0.1", port)) else {
            eprintln!("qb-dap: cannot reach the adapter on port {port}");
            return 2;
        };
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
        let hook = Hook { stepper: Stepper::new(stop_on_entry), breakpoints, pause, resume: rx, out: conn };
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

fn set_breakpoints(bps: &Mutex<HashSet<u32>>, msg: &Value) {
    let lines = msg["lines"].as_array().map(|a| a.iter().filter_map(|l| l.as_u64()).map(|l| l as u32).collect()).unwrap_or_default();
    *bps.lock().unwrap() = lines;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_rules() {
        let none = HashSet::new();
        let bp: HashSet<u32> = [5].into();
        let mut s = Stepper::new(true);
        assert_eq!(s.check(0, 1, 1, false, &none), Some("entry"));
        assert_eq!(s.check(1, 2, 1, false, &none), None);
        // breakpoint once per arrival on the line
        assert_eq!(s.check(2, 5, 1, false, &bp), Some("breakpoint"));
        assert_eq!(s.check(3, 5, 1, false, &bp), None);
        // over: skips deeper frames, stops back in this frame
        s.resume("over", 3, 1);
        assert_eq!(s.check(10, 20, 2, false, &none), None);
        assert_eq!(s.check(4, 6, 1, false, &none), Some("step"));
        s.resume("in", 4, 1);
        assert_eq!(s.check(10, 20, 2, false, &none), Some("step"));
        s.resume("out", 10, 2);
        assert_eq!(s.check(11, 21, 2, false, &none), None);
        assert_eq!(s.check(5, 7, 1, false, &none), Some("step"));
        s.resume("continue", 5, 1);
        assert_eq!(s.check(6, 8, 1, true, &none), Some("pause"));
    }
}

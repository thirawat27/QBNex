//! The DAP side: talks to the client on stdio and drives one debuggee process.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::debuggee::Loc;
use crate::names::{Casing, func_name, var_name};
use crate::protocol::{read_frame, write_message};
use crate::snapshot::{FileTag, MAIN_FILE, Snapshot, Val, text};

enum In {
    Client(Value),
    /// a client message whose body is not JSON (the framing around it was fine)
    Malformed(String),
    ClientGone,
    Child(Value),
    Output(&'static str, String),
    /// The debuggee ended. `from_program`: the program itself gave `code`
    /// (through the exit file); else it comes from the process status.
    Exited { code: i64, from_program: bool },
}

#[derive(Clone, Debug, Default)]
struct Launch {
    /// absolute path of the `.bas` file
    program: PathBuf,
    qir_file: PathBuf,
    cwd: PathBuf,
    stop_on_entry: bool,
    no_debug: bool,
    input: Option<PathBuf>,
    /// the program's arguments (COMMAND$)
    args: Vec<String>,
    /// where the debuggee writes the program's exit code
    exit_file: PathBuf,
}

/// The statement lines of the program, per file, from the QIR line table.
#[derive(Clone, Debug, Default, PartialEq)]
struct Lines {
    /// file -> lines that start a statement
    stmts: BTreeMap<FileTag, BTreeSet<u32>>,
    /// QIR `srcfile` names (`$INCLUDE` files), index = file tag - 1
    srcfiles: Vec<String>,
}

/// A breakpoint as the client set it.
#[derive(Clone, Debug)]
struct Breakpoint {
    id: u64,
    /// the line asked for
    line: u32,
}

const NOT_OURS: &str = "breakpoints work in the launched program and its $INCLUDE files only";
const NOT_LAUNCHED: &str = "the program is not launched yet";
const NO_STATEMENT: &str = "no statement at or after this line";

struct Adapter {
    seq: i64,
    out: Box<dyn Write>,
    tx: Sender<In>,
    launch: Option<Launch>,
    configured: bool,
    started: bool,
    /// statement lines per file; `None` until the program is compiled
    lines: Option<Lines>,
    /// file tag -> path: the program, then its `$INCLUDE` files
    files: Vec<PathBuf>,
    /// breakpoints as the client set them, per source file (a request for one
    /// file leaves the others alone)
    sources: Vec<(PathBuf, Vec<Breakpoint>)>,
    next_bp: u64,
    casing: Casing,
    conn: Option<TcpStream>,
    child: Option<Arc<Mutex<Child>>>,
    snapshot: Option<Snapshot>,
    /// variablesReference - 1 -> (display name, value)
    refs: Vec<Vec<(String, Val)>>,
    temp: Option<PathBuf>,
    /// a `terminated` event went out (the client gets one only)
    terminated: bool,
    /// the client asked to terminate: the debuggee's end is not the program's own
    terminating: bool,
}

/// Runs the adapter on stdin/stdout; returns the process exit code.
pub fn run_stdio() -> i32 {
    let (tx, rx) = channel();
    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mut r = BufReader::new(std::io::stdin());
            loop {
                // Only broken framing or the end of input ends the session: a
                // body that is not JSON is reported and the next message read.
                let m = match read_frame(&mut r) {
                    Ok(Some(body)) => match serde_json::from_slice(&body) {
                        Ok(v) => In::Client(v),
                        Err(e) => In::Malformed(e.to_string()),
                    },
                    _ => {
                        let _ = tx.send(In::ClientGone);
                        break;
                    }
                };
                if tx.send(m).is_err() {
                    break;
                }
            }
        });
    }
    let mut a = Adapter::new(Box::new(std::io::stdout()), tx);
    for msg in rx {
        if !a.handle(msg) {
            break;
        }
    }
    a.finish();
    0
}

/// `p` made absolute against `base` (an absolute `p` stays as it is).
fn absolute_in(base: &Path, p: impl AsRef<Path>) -> PathBuf {
    let joined = base.join(p);
    std::path::absolute(&joined).unwrap_or(joined)
}

/// The launch configuration's `args`: a list of strings, or one string that
/// is one argument (it is not split: no shell rules are guessed).
fn program_args(v: &Value) -> Result<Vec<String>, String> {
    const BAD: &str = "launch: \"args\" must be a list of strings";
    match v {
        Value::Null => Ok(Vec::new()),
        Value::String(s) => Ok(vec![s.clone()]),
        Value::Array(a) => a.iter().map(|x| x.as_str().map(str::to_string).ok_or_else(|| BAD.to_string())).collect(),
        _ => Err(BAD.to_string()),
    }
}

/// Bytes of a QIR hex literal (`x41 42` written `x4142`).
fn unhex(h: &str) -> Option<Vec<u8>> {
    let h = h.strip_prefix('x')?;
    if !h.len().is_multiple_of(2) || !h.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).ok()).collect()
}

/// Statement lines per file from QIR text: `line N` starts a statement of the
/// program's file, unless an `incline L F` follows it (the statement is line
/// `L` of `srcfile F`; the `N` of such a statement is its `$INCLUDE` line).
fn line_table(qir: &str) -> Lines {
    fn add(t: &mut Lines, file: FileTag, line: u32) {
        t.stmts.entry(file).or_default().insert(line);
    }
    let mut t = Lines::default();
    let mut pending: Option<u32> = None;
    for l in qir.lines() {
        let mut w = l.split_ascii_whitespace();
        match w.next() {
            Some("line") => {
                if let Some(n) = pending.take() {
                    add(&mut t, MAIN_FILE, n);
                }
                pending = w.next().and_then(|n| n.parse().ok());
            }
            Some("incline") => {
                let line = w.next().and_then(|n| n.parse::<u32>().ok());
                let file = w.next().and_then(|n| n.parse::<u32>().ok());
                if let (Some(line), Some(file)) = (line, file)
                    && pending.take().is_some()
                {
                    add(&mut t, file.saturating_add(1), line);
                }
            }
            Some("srcfile") => {
                let k = w.next().and_then(|n| n.parse::<usize>().ok());
                let name = w.next().and_then(unhex);
                if let (Some(k), Some(name)) = (k, name)
                    && k < 1 << 16
                {
                    if t.srcfiles.len() <= k {
                        t.srcfiles.resize(k + 1, String::new());
                    }
                    t.srcfiles[k] = text(&name);
                }
            }
            _ => {}
        }
    }
    if let Some(n) = pending {
        add(&mut t, MAIN_FILE, n);
    }
    t
}

/// Path of an `$INCLUDE` file named in the QIR: the compiler writes it
/// relative to the main source's folder when it lies below it.
fn include_path(program_dir: &Path, name: &str) -> PathBuf {
    let p = Path::new(name);
    if p.is_absolute() { p.to_path_buf() } else { absolute_in(program_dir, p) }
}

/// Exit code of a process status; a Unix signal as a shell reports it (128 + n).
fn status_code(s: ExitStatus) -> i64 {
    if let Some(c) = s.code() {
        return i64::from(c);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(n) = s.signal() {
            return 128 + i64::from(n);
        }
    }
    1
}

/// Command line of the debuggee for a launch.
fn debuggee_args(l: &Launch, port: u16) -> Vec<OsString> {
    let mut a: Vec<OsString> = vec!["--debuggee".into(), l.qir_file.clone().into(), port.to_string().into()];
    // DECLARE DYNAMIC LIBRARY "name" looks in the program's folder, not next to qb-dap
    // (the program path is absolute: the debuggee runs in the launch's cwd)
    if let Some(dir) = l.program.parent().filter(|d| !d.as_os_str().is_empty()) {
        a.push("--program-dir".into());
        a.push(dir.as_os_str().to_os_string());
    }
    a.push("--exit-file".into());
    a.push(l.exit_file.clone().into());
    if l.no_debug {
        a.push("--no-debug".into());
    }
    if l.stop_on_entry {
        a.push("--stop-on-entry".into());
    }
    // The program's argv as a normal run gives it (COMMAND$, _COMMANDCOUNT):
    // COMMAND$(0) is the executable a build writes next to the source.
    a.push("--".into());
    a.push(l.program.with_extension(std::env::consts::EXE_EXTENSION).into());
    a.extend(l.args.iter().map(OsString::from));
    a
}

/// A new directory `<prefix>-<pid>-<random>` in the system temp dir. It is
/// created with `create_dir` semantics (fails on an existing path or
/// symlink, so another user cannot pre-plant it) and is private on Unix.
fn fresh_temp_dir(prefix: &str) -> std::io::Result<PathBuf> {
    use std::hash::{BuildHasher, Hasher};
    let base = std::env::temp_dir();
    let mut last = std::io::Error::other("no fresh temp dir");
    for attempt in 0..16u32 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u32(std::process::id());
        h.write_u32(attempt);
        if let Ok(t) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            h.write_u128(t.as_nanos());
        }
        let dir = base.join(format!("{prefix}-{}-{:016x}", std::process::id(), h.finish()));
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        match builder.create(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

/// QIR payload appended to an executable by `qb --ir` (see qb-exec's main.rs).
fn qir_payload(exe: &[u8]) -> Option<&[u8]> {
    let n = exe.len();
    let le = |b: &[u8]| Some(u64::from_le_bytes(b.try_into().ok()?) as usize);
    if n >= 24 && &exe[n - 8..] == b"QBNEXAO1" {
        let (ql, il) = (le(&exe[n - 24..n - 16])?, le(&exe[n - 16..n - 8])?);
        let end = n - 24;
        return exe.get(end.checked_sub(il)?.checked_sub(ql)?..end - il);
    }
    if n < 16 || &exe[n - 8..] != b"QBNEXIR0" {
        return None;
    }
    let len = le(&exe[n - 16..n - 8])?;
    exe.get((n - 16).checked_sub(len)?..n - 16)
}

/// Environment variable that hands the debuggee the token it must send first.
pub const TOKEN_VAR: &str = "QBNEX_DAP_TOKEN";

/// 128 random bits from the operating system, as hex.
fn new_token() -> Result<String, String> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|e| format!("no random numbers for the debuggee token: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Reads one line of at most `max` bytes before `until`, byte by byte
/// (nothing after the newline is consumed: the connection then carries the
/// protocol). A peer that sends a byte now and then cannot stretch the wait.
fn read_first_line(c: &mut TcpStream, max: usize, until: Instant) -> Option<String> {
    let mut line = Vec::new();
    let mut b = [0u8; 1];
    loop {
        let left = until.checked_duration_since(Instant::now()).filter(|d| !d.is_zero())?;
        c.set_read_timeout(Some(left)).ok()?;
        match c.read(&mut b) {
            Ok(1) if b[0] == b'\n' => return String::from_utf8(line).ok(),
            Ok(1) if line.len() < max => line.push(b[0]),
            _ => return None,
        }
    }
}

fn same_token(a: &str, b: &str) -> bool {
    // compared in full, so the time taken does not tell how much was right
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// Waits for the debuggee on `listener`. The loopback port can be reached by
/// any local process (other users on the machine too): a connection counts
/// only once its first line is `token`. Others are closed and the wait goes
/// on, each one given `handshake` to send its line, all within `deadline`.
/// `check` is called while waiting (the debuggee may have exited).
fn accept_debuggee(listener: &TcpListener, token: &str, deadline: Duration, handshake: Duration, mut check: impl FnMut() -> Result<(), String>) -> Result<TcpStream, String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok((mut c, _)) => {
                c.set_nonblocking(false).map_err(|e| e.to_string())?;
                let left = deadline.saturating_sub(started.elapsed()).max(Duration::from_millis(10));
                if read_first_line(&mut c, token.len() + 2, Instant::now() + handshake.min(left)).is_some_and(|l| same_token(l.trim_end_matches('\r'), token)) {
                    c.set_read_timeout(None).map_err(|e| e.to_string())?;
                    return Ok(c);
                }
                // not the debuggee: drop it and keep listening
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(format!("debuggee did not connect: {e}")),
        }
        if started.elapsed() >= deadline {
            return Err("debuggee did not connect in time".to_string());
        }
        check()?;
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let (a, b) = (canon(a), canon(b));
    // (file names are case-insensitive on Windows and, by default, on macOS)
    if cfg!(any(windows, target_os = "macos")) { a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase() } else { a == b }
}

impl Adapter {
    fn new(out: Box<dyn Write>, tx: Sender<In>) -> Adapter {
        Adapter {
            seq: 0,
            out,
            tx,
            launch: None,
            configured: false,
            started: false,
            lines: None,
            files: Vec::new(),
            sources: Vec::new(),
            next_bp: 0,
            casing: Casing::default(),
            conn: None,
            child: None,
            snapshot: None,
            refs: Vec::new(),
            temp: None,
            terminated: false,
            terminating: false,
        }
    }

    /// Handles one input; `false` ends the session.
    fn handle(&mut self, msg: In) -> bool {
        match msg {
            In::Client(m) => return self.client(m),
            In::Malformed(e) => {
                self.event("output", json!({"category": "console", "output": format!("qb-dap: ignored a message that is not JSON: {e}\n")}));
            }
            In::ClientGone => return false,
            In::Child(m) => self.child_msg(m),
            In::Output(cat, s) => self.event("output", json!({"category": cat, "output": s})),
            In::Exited { code, from_program } => {
                // After a terminate request the end is not the program's: no exit
                // code is made up for it (one the program gave is still told).
                if from_program || !self.terminating {
                    self.event("exited", json!({"exitCode": code}));
                }
                self.terminated();
            }
        }
        true
    }

    /// Stops the debuggee and removes the work directory.
    fn finish(&mut self) {
        self.kill();
        if let Some(t) = self.temp.take() {
            let _ = std::fs::remove_dir_all(t);
        }
    }

    /// The `terminated` event, once per session.
    fn terminated(&mut self) {
        if !self.terminated {
            self.terminated = true;
            self.event("terminated", json!({}));
        }
    }

    fn send(&mut self, mut msg: Value) {
        self.seq += 1;
        msg["seq"] = json!(self.seq);
        let _ = write_message(&mut self.out, &msg);
    }

    fn event(&mut self, event: &str, body: Value) {
        self.send(json!({"type": "event", "event": event, "body": body}));
    }

    fn respond(&mut self, req: &Value, body: Value) {
        self.send(json!({"type": "response", "request_seq": req["seq"], "success": true, "command": req["command"], "body": body}));
    }

    fn fail(&mut self, req: &Value, message: &str) {
        self.send(json!({
            "type": "response", "request_seq": req["seq"], "success": false, "command": req["command"],
            "message": message, "body": {"error": {"id": 1, "format": message}}
        }));
    }

    fn send_child(&mut self, msg: Value) {
        if let Some(c) = self.conn.as_mut() {
            let _ = writeln!(c, "{msg}");
        }
    }

    fn kill(&mut self) {
        self.conn = None;
        if let Some(c) = self.child.take() {
            let _ = c.lock().unwrap().kill();
        }
    }

    /// Handles a client message; `false` ends the session.
    fn client(&mut self, req: Value) -> bool {
        if !req["type"].is_string() {
            self.event("output", json!({"category": "console", "output": "qb-dap: ignored a message that is not a DAP message\n"}));
            return true;
        }
        if req["type"] != "request" {
            return true;
        }
        let args = req["arguments"].clone();
        match req["command"].as_str().unwrap_or("") {
            "initialize" => {
                self.respond(
                    &req,
                    json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsEvaluateForHovers": true,
                        "supportsTerminateRequest": true,
                        "supportsSingleThreadExecutionRequests": false,
                    }),
                );
                self.event("initialized", json!({}));
            }
            "launch" => match self.prepare(&args) {
                Ok(l) => {
                    self.launch = Some(l);
                    self.respond(&req, json!({}));
                    // breakpoints set before the launch now know their lines
                    self.breakpoints_changed();
                    self.try_start();
                }
                Err(e) => self.fail(&req, &e),
            },
            "setBreakpoints" => {
                let path = args["source"]["path"].as_str().map(PathBuf::from);
                let asked: Vec<u32> = args["breakpoints"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|b| b["line"].as_u64()).map(|l| u32::try_from(l).unwrap_or(u32::MAX)).collect())
                    .unwrap_or_default();
                let mut bps = Vec::with_capacity(asked.len());
                for line in asked {
                    self.next_bp += 1;
                    bps.push(Breakpoint { id: self.next_bp, line });
                }
                let body: Vec<Value> = bps.iter().map(|b| self.breakpoint_json(path.as_deref(), b)).collect();
                if let Some(p) = path {
                    match self.sources.iter_mut().find(|(s, _)| same_file(s, &p)) {
                        Some(entry) => entry.1 = bps,
                        None => self.sources.push((p, bps)),
                    }
                    self.sync_breakpoints();
                }
                self.respond(&req, json!({"breakpoints": body}));
            }
            "setExceptionBreakpoints" | "setFunctionBreakpoints" => self.respond(&req, json!({"breakpoints": []})),
            "configurationDone" => {
                self.configured = true;
                self.respond(&req, json!({}));
                self.try_start();
            }
            "threads" => self.respond(&req, json!({"threads": [{"id": 1, "name": "main"}]})),
            "stackTrace" => {
                let frames: Vec<Value> = self
                    .snapshot
                    .as_ref()
                    .map(|s| s.frames.as_slice())
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        // a statement of an $INCLUDE file is shown in that file, at its line there
                        let path = self.source_path(f.file);
                        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        json!({
                            "id": i, "name": func_name(&f.func, &self.casing), "line": f.line, "column": 1,
                            "source": {"name": name, "path": path.to_string_lossy()},
                        })
                    })
                    .collect();
                let n = frames.len();
                self.respond(&req, json!({"stackFrames": frames, "totalFrames": n}));
            }
            "scopes" => {
                let i = args["frameId"].as_u64().unwrap_or(0) as usize;
                let Some(s) = self.snapshot.clone() else { return self.fail_ret(&req, "not stopped") };
                let Some(f) = s.frames.get(i) else { return self.fail_ret(&req, "no such frame") };
                let locals: Vec<(String, Val)> = f.locals.iter().map(|v| (var_name(&v.qir, &f.func, &self.casing), v.val.clone())).collect();
                let globals: Vec<(String, Val)> = s.globals.iter().map(|v| (var_name(&v.qir, "main", &self.casing), v.val.clone())).collect();
                let (nl, ng) = (locals.len(), globals.len());
                let lref = self.new_ref(locals);
                let gref = self.new_ref(globals);
                let local_name = if f.func == "main" { "Module" } else { "Locals" };
                self.respond(
                    &req,
                    json!({"scopes": [
                        {"name": local_name, "presentationHint": "locals", "variablesReference": lref, "namedVariables": nl, "expensive": false},
                        {"name": "Globals", "variablesReference": gref, "namedVariables": ng, "expensive": false},
                    ]}),
                );
            }
            "variables" => {
                let r = args["variablesReference"].as_u64().unwrap_or(0) as usize;
                let Some(items) = r.checked_sub(1).and_then(|i| self.refs.get(i)).cloned() else {
                    return self.fail_ret(&req, "unknown variablesReference");
                };
                let vars: Vec<Value> = items.into_iter().map(|(name, v)| self.var_json(name, &v)).collect();
                self.respond(&req, json!({"variables": vars}));
            }
            "evaluate" => {
                let expr = args["expression"].as_str().unwrap_or("").trim().to_string();
                let frame = args["frameId"].as_u64().unwrap_or(0) as usize;
                match self.evaluate(&expr, frame) {
                    Some(v) => {
                        let r = self.child_ref(&v);
                        self.respond(&req, json!({"result": v.display(), "type": v.type_name(), "variablesReference": r}));
                    }
                    None => self.fail(&req, &format!("cannot evaluate {expr}")),
                }
            }
            cmd @ ("continue" | "next" | "stepIn" | "stepOut") => {
                let mode = match cmd {
                    "next" => "over",
                    "stepIn" => "in",
                    "stepOut" => "out",
                    _ => "continue",
                };
                self.snapshot = None;
                self.refs.clear();
                self.send_child(json!({"t": "resume", "mode": mode}));
                self.respond(&req, json!({"allThreadsContinued": true}));
            }
            "pause" => {
                self.send_child(json!({"t": "pause"}));
                self.respond(&req, json!({}));
            }
            "terminate" => {
                self.terminating = true;
                self.kill();
                self.respond(&req, json!({}));
                self.terminated();
            }
            "disconnect" => {
                self.kill();
                self.respond(&req, json!({}));
                return false;
            }
            other => {
                let m = format!("unsupported request {other}");
                self.fail(&req, &m);
            }
        }
        true
    }

    /// Path of a file tag; the program's for one the QIR did not name.
    fn source_path(&self, file: FileTag) -> PathBuf {
        match self.files.get(file as usize) {
            Some(p) => p.clone(),
            None => self.launch.as_ref().map(|l| l.program.clone()).unwrap_or_default(),
        }
    }

    /// Where a breakpoint asked at `line` of `path` goes: the line it is moved
    /// to (the next statement) and its places, or why it cannot be set.
    fn resolve(&self, path: Option<&Path>, line: u32) -> Result<(u32, Vec<Loc>), &'static str> {
        let Some(lines) = &self.lines else { return Err(NOT_LAUNCHED) };
        let Some(path) = path else { return Err(NOT_OURS) };
        // (one file can be several tags: the program named again as an include)
        let tags: Vec<FileTag> = self.files.iter().enumerate().filter(|(_, f)| same_file(path, f)).map(|(i, _)| i as FileTag).collect();
        if tags.is_empty() {
            return Err(NOT_OURS);
        }
        if lines.stmts.values().all(BTreeSet::is_empty) {
            // a QIR without a line table: taken as written
            return Ok((line, tags.iter().map(|&t| (t, line)).collect()));
        }
        let at = tags.iter().filter_map(|t| lines.stmts.get(t)?.range(line..).next().copied()).min().ok_or(NO_STATEMENT)?;
        let places = tags.iter().filter(|t| lines.stmts.get(t).is_some_and(|s| s.contains(&at))).map(|&t| (t, at)).collect();
        Ok((at, places))
    }

    fn breakpoint_json(&self, path: Option<&Path>, b: &Breakpoint) -> Value {
        match self.resolve(path, b.line) {
            Ok((line, _)) => json!({"id": b.id, "verified": true, "line": line}),
            Err(why) => json!({"id": b.id, "verified": false, "line": b.line, "message": why}),
        }
    }

    /// Every place with a breakpoint, over all source files.
    fn breakpoint_places(&self) -> Vec<Loc> {
        let mut set = BTreeSet::new();
        for (path, bps) in &self.sources {
            for b in bps {
                if let Ok((_, places)) = self.resolve(Some(path.as_path()), b.line) {
                    set.extend(places);
                }
            }
        }
        set.into_iter().collect()
    }

    /// Hands the debuggee the whole set of breakpoint places.
    fn sync_breakpoints(&mut self) {
        let at: Vec<[u32; 2]> = self.breakpoint_places().into_iter().map(|(f, l)| [f, l]).collect();
        self.send_child(json!({"t": "bp", "at": at}));
    }

    /// Tells the client what became of its breakpoints once the program's
    /// lines are known.
    fn breakpoints_changed(&mut self) {
        let mut changed = Vec::new();
        for (path, bps) in &self.sources {
            for b in bps {
                changed.push(self.breakpoint_json(Some(path.as_path()), b));
            }
        }
        for b in changed {
            self.event("breakpoint", json!({"reason": "changed", "breakpoint": b}));
        }
    }

    fn fail_ret(&mut self, req: &Value, m: &str) -> bool {
        self.fail(req, m);
        true
    }

    fn new_ref(&mut self, items: Vec<(String, Val)>) -> usize {
        self.refs.push(items);
        self.refs.len()
    }

    fn child_ref(&mut self, v: &Val) -> usize {
        if !v.is_composite() {
            return 0;
        }
        let kids = v
            .children()
            .into_iter()
            .map(|(n, c)| (if n.starts_with('(') { n } else { self.casing.spell(&n) }, c))
            .collect();
        self.new_ref(kids)
    }

    fn var_json(&mut self, name: String, v: &Val) -> Value {
        let r = self.child_ref(v);
        let mut j = json!({"name": name, "value": v.display(), "type": v.type_name(), "variablesReference": r});
        if let Val::Array { total, .. } = v {
            j["indexedVariables"] = json!(total);
        }
        j
    }

    /// `name`, `name.field`, `name(1, 2).field` against the stopped frame.
    fn evaluate(&self, expr: &str, frame: usize) -> Option<Val> {
        let s = self.snapshot.as_ref()?;
        let f = s.frames.get(frame)?;
        let head_len = expr.find(['.', '(']).unwrap_or(expr.len());
        let head = expr[..head_len].trim();
        let candidates: Vec<(String, &Val)> = f
            .locals
            .iter()
            .map(|v| (var_name(&v.qir, &f.func, &self.casing), &v.val))
            .chain(s.globals.iter().map(|v| (var_name(&v.qir, "main", &self.casing), &v.val)))
            .collect();
        let strip = |n: &str| n.trim_end_matches(['%', '&', '!', '#', '$', '~', '`']).to_ascii_uppercase();
        let mut val = candidates
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(head))
            .or_else(|| candidates.iter().find(|(n, _)| strip(n) == strip(head)))
            .map(|(_, v)| (*v).clone())?;
        let mut rest = expr[head_len..].trim();
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix('.') {
                let end = r.find(['.', '(']).unwrap_or(r.len());
                val = val.field(r[..end].trim())?.clone();
                rest = r[end..].trim();
            } else {
                let r = rest.strip_prefix('(')?;
                let end = r.find(')')?;
                let idx: Option<Vec<i64>> = r[..end].split(',').map(|x| x.trim().parse().ok()).collect();
                val = val.element(&idx?)?.clone();
                rest = r[end + 1..].trim();
            }
        }
        Some(val)
    }

    fn child_msg(&mut self, m: Value) {
        if m["t"] == "stopped"
            && let Ok(s) = serde_json::from_value::<Snapshot>(m)
        {
            let reason = s.reason.clone();
            self.snapshot = Some(s);
            self.refs.clear();
            self.event("stopped", json!({"reason": reason, "threadId": 1, "allThreadsStopped": true}));
        }
    }

    /// Validates the launch arguments and produces the QIR file.
    fn prepare(&mut self, args: &Value) -> Result<Launch, String> {
        let program = args["program"].as_str().ok_or("launch: \"program\" (the .bas file) is required")?;
        let here = std::env::current_dir().map_err(|e| format!("cannot read the current folder: {e}"))?;
        // relative paths of the launch configuration are taken from its cwd,
        // else from the folder qb-dap was started in
        let cwd = args["cwd"].as_str().map(|c| absolute_in(&here, c));
        let base = cwd.clone().unwrap_or(here);
        let program = absolute_in(&base, program);
        let dir = program.parent().map(Path::to_path_buf).unwrap_or_default();
        let program_args = program_args(&args["args"])?;
        if let Ok(bytes) = std::fs::read(&program) {
            self.casing = Casing::from_source(&qb_syntax::encoding::decode(&bytes).0);
        }
        let temp = match &self.temp {
            Some(t) => t.clone(),
            None => fresh_temp_dir("qb-dap").map_err(|e| format!("cannot create a work directory: {e}"))?,
        };
        self.temp = Some(temp.clone());
        let qir_text = match args["qir"].as_str() {
            Some(q) => {
                let q = absolute_in(&base, q);
                std::fs::read_to_string(&q).map_err(|e| format!("cannot read {}: {e}", q.display()))?
            }
            None => {
                let compiler = args["compiler"].as_str().unwrap_or("qb");
                let exe = temp.join(if cfg!(windows) { "program.exe" } else { "program" });
                let out = Command::new(compiler)
                    .arg(&program)
                    .arg("--ir")
                    .arg("-o")
                    .arg(&exe)
                    .current_dir(&dir)
                    .stdin(Stdio::null())
                    .output()
                    .map_err(|e| format!("cannot run the compiler \"{compiler}\": {e}"))?;
                let log = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
                if !out.status.success() {
                    return Err(format!("compile failed:\n{log}"));
                }
                let bytes = std::fs::read(&exe).map_err(|e| format!("{}: {e}", exe.display()))?;
                let qir = qir_payload(&bytes).ok_or("the compiler output carries no QIR (needs a qb with --ir)")?;
                String::from_utf8_lossy(qir).into_owned()
            }
        };
        let lines = line_table(&qir_text);
        let qir_file = temp.join("program.qir");
        std::fs::write(&qir_file, &qir_text).map_err(|e| e.to_string())?;
        self.files = std::iter::once(program.clone()).chain(lines.srcfiles.iter().map(|n| include_path(&dir, n))).collect();
        self.lines = Some(lines);
        Ok(Launch {
            program,
            qir_file,
            cwd: cwd.unwrap_or(dir),
            stop_on_entry: args["stopOnEntry"].as_bool().unwrap_or(false),
            no_debug: args["noDebug"].as_bool().unwrap_or(false),
            input: args["input"].as_str().map(|p| absolute_in(&base, p)),
            args: program_args,
            exit_file: temp.join("exit-code"),
        })
    }

    fn try_start(&mut self) {
        if self.started || !self.configured {
            return;
        }
        let Some(l) = self.launch.clone() else { return };
        self.started = true;
        if let Err(e) = self.start(&l) {
            self.event("output", json!({"category": "stderr", "output": format!("qb-dap: {e}\n")}));
            self.terminated();
        }
    }

    fn start(&mut self, l: &Launch) -> Result<(), String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let token = new_token()?;
        // a code left from before is not this run's
        let _ = std::fs::remove_file(&l.exit_file);
        let mut cmd = Command::new(exe);
        // (the token goes in the environment, not on the command line, which other
        // users can read; the debuggee removes it before the program runs)
        cmd.args(debuggee_args(l, port)).env(TOKEN_VAR, &token);
        let stdin = match &l.input {
            Some(p) => Stdio::from(std::fs::File::open(p).map_err(|e| format!("{}: {e}", p.display()))?),
            None => Stdio::null(),
        };
        let mut child = cmd.current_dir(&l.cwd).stdin(stdin).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
        for (pipe, cat) in [(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>), "stdout"), (child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>), "stderr")] {
            let Some(mut pipe) = pipe else { continue };
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                // A read can end inside a multi-byte character (Thai is 3 bytes): the
                // incomplete tail waits for the next read, or `text` would take the
                // whole chunk for a legacy code page.
                let mut pend: Vec<u8> = Vec::new();
                while let Ok(n) = pipe.read(&mut buf) {
                    pend.extend_from_slice(&buf[..n]);
                    let cut = match std::str::from_utf8(&pend) {
                        Err(e) if e.error_len().is_none() && n != 0 => e.valid_up_to(),
                        _ => pend.len(),
                    };
                    let rest = pend.split_off(cut);
                    if (!pend.is_empty() && tx.send(In::Output(cat, text(&pend))).is_err()) || n == 0 {
                        break;
                    }
                    pend = rest;
                }
            });
        }
        let child = Arc::new(Mutex::new(child));
        self.child = Some(child.clone());
        if !l.no_debug {
            let conn = accept_debuggee(&listener, &token, Duration::from_secs(20), Duration::from_secs(5), || match child.lock().unwrap().try_wait() {
                Ok(Some(_)) => Err("the debuggee exited before connecting".to_string()),
                _ => Ok(()),
            })?;
            drop(listener);
            let reader = conn.try_clone().map_err(|e| e.to_string())?;
            self.conn = Some(conn);
            self.sync_breakpoints();
            self.send_child(json!({"t": "go"}));
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let mut r = BufReader::new(reader);
                let mut line = String::new();
                while r.read_line(&mut line).unwrap_or(0) > 0 {
                    if let Ok(v) = serde_json::from_str(&line) {
                        let _ = tx.send(In::Child(v));
                    }
                    line.clear();
                }
            });
        }
        let tx = self.tx.clone();
        let exit_file = l.exit_file.clone();
        std::thread::spawn(move || {
            loop {
                let status = child.lock().unwrap().try_wait();
                match status {
                    Ok(Some(s)) => {
                        // let the output threads drain first
                        std::thread::sleep(Duration::from_millis(50));
                        // The program's own code (SYSTEM -1, SYSTEM 300) from the exit
                        // file: the process status keeps only part of it on Unix.
                        let given = std::fs::read_to_string(&exit_file).ok().and_then(|t| t.trim().parse::<i64>().ok());
                        let _ = tx.send(In::Exited { code: given.unwrap_or_else(|| status_code(s)), from_program: given.is_some() });
                        break;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                    Err(_) => break,
                }
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connect(port: u16, first: Option<&str>) -> TcpStream {
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        if let Some(f) = first {
            c.write_all(f.as_bytes()).unwrap();
        }
        c
    }

    #[test]
    fn only_the_connection_with_the_token_is_the_debuggee() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let token = new_token().unwrap();
        assert_eq!(token.len(), 32);
        assert_ne!(token, new_token().unwrap());
        let good = token.clone();
        let t = std::thread::spawn(move || {
            // wrong token, a line too long, silence: all turned away first
            let _a = connect(port, Some("0123456789abcdef0123456789abcdef\n"));
            let _b = connect(port, Some(&"x".repeat(200)));
            let _c = connect(port, None);
            std::thread::sleep(Duration::from_millis(400));
            let mut d = connect(port, Some(&format!("{good}\n")));
            d.write_all(b"after\n").unwrap();
            d
        });
        let conn = accept_debuggee(&listener, &token, Duration::from_secs(20), Duration::from_millis(300), || Ok(())).expect("debuggee accepted");
        let _d = t.join().unwrap();
        // the line after the token is left for the protocol
        let mut r = BufReader::new(conn);
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        assert_eq!(line, "after\n");
    }

    #[test]
    fn waiting_for_the_debuggee_ends() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let e = accept_debuggee(&listener, "t", Duration::from_millis(200), Duration::from_millis(50), || Ok(())).unwrap_err();
        assert!(e.contains("in time"), "{e}");
        let e = accept_debuggee(&listener, "t", Duration::from_secs(20), Duration::from_millis(50), || Err("exited".into())).unwrap_err();
        assert_eq!(e, "exited");
        assert!(same_token("abc", "abc") && !same_token("abc", "abd") && !same_token("abc", "ab"));
    }

    /// Output of an adapter under test.
    #[derive(Clone, Default)]
    struct Buf(Arc<Mutex<Vec<u8>>>);

    impl Write for Buf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Buf {
        /// The messages written since the last call.
        fn take(&self) -> Vec<Value> {
            let data = std::mem::take(&mut *self.0.lock().unwrap());
            let mut r = BufReader::new(&data[..]);
            let mut v = Vec::new();
            while let Ok(Some(m)) = crate::protocol::read_message(&mut r) {
                v.push(m);
            }
            v
        }
    }

    fn adapter() -> (Adapter, Buf) {
        let buf = Buf::default();
        let (tx, _rx) = channel();
        (Adapter::new(Box::new(buf.clone()), tx), buf)
    }

    fn request(a: &mut Adapter, seq: i64, command: &str, arguments: Value) -> bool {
        a.handle(In::Client(json!({"seq": seq, "type": "request", "command": command, "arguments": arguments})))
    }

    fn events<'a>(msgs: &'a [Value], name: &str) -> Vec<&'a Value> {
        msgs.iter().filter(|m| m["event"] == name).collect()
    }

    fn response<'a>(msgs: &'a [Value], command: &str) -> &'a Value {
        msgs.iter().find(|m| m["type"] == "response" && m["command"] == command).unwrap_or_else(|| panic!("no {command} response in {msgs:?}"))
    }

    fn work_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qb-dap-unit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn hex(s: &str) -> String {
        s.bytes().map(|b| format!("{b:02X}")).collect()
    }

    /// A program with lines 1 and 5, and `$INCLUDE: 'inc.bi'` on line 2 whose
    /// statements are lines 3 and 4 of inc.bi.
    fn program_with_include(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
        let bas = dir.join("prog.bas");
        let inc = dir.join("inc.bi");
        let qir = dir.join("prog.qir");
        std::fs::write(&bas, "PRINT 1\n'$INCLUDE: 'inc.bi'\n\n\nPRINT 5\n").unwrap();
        std::fs::write(&inc, "\n\nPRINT 3\nPRINT 4\n").unwrap();
        let text = format!("QIR 1\nsrcfile 0 x{}\nfunc main\nline 1\nline 2\nincline 3 0\nline 2\nincline 4 0\nline 5\nend\nendfunc\n", hex("inc.bi"));
        std::fs::write(&qir, text).unwrap();
        (bas, inc, qir)
    }

    #[test]
    fn line_table_per_file() {
        let qir = format!("QIR 1\nsrcfile 0 x{}\nsrcfile 1 x{}\nfunc main\nline 1\nline 2\nincline 3 0\n  line 2\n  incline 9 1\nconst.i32 %0 1\nline 7\nendfunc\n", hex("a.bi"), hex("sub/b.bi"));
        let t = line_table(&qir);
        assert_eq!(t.srcfiles, vec!["a.bi".to_string(), "sub/b.bi".to_string()]);
        assert_eq!(t.stmts[&MAIN_FILE], BTreeSet::from([1, 7]));
        assert_eq!(t.stmts[&1], BTreeSet::from([3]));
        assert_eq!(t.stmts[&2], BTreeSet::from([9]));
        assert_eq!(unhex("x4142"), Some(b"AB".to_vec()));
        assert_eq!(unhex("x414"), None);
        assert_eq!(unhex("x+f"), None);
    }

    /// Breakpoints sent before the launch wait for the program's lines: then
    /// they are moved to statements and reported again. Each file keeps its
    /// own; one outside the program is refused and does not replace the
    /// program's.
    #[test]
    fn breakpoints_before_launch_and_per_file() {
        let dir = work_dir("bps");
        let (bas, inc, qir) = program_with_include(&dir);
        let other = dir.join("other.bas");
        let (mut a, out) = adapter();
        request(&mut a, 1, "initialize", json!({}));
        out.take();
        request(&mut a, 2, "setBreakpoints", json!({"source": {"path": bas}, "breakpoints": [{"line": 1}, {"line": 3}]}));
        let r = out.take();
        let bps = &response(&r, "setBreakpoints")["body"]["breakpoints"];
        assert_eq!(bps[0]["verified"], false, "{bps}");
        assert_eq!(bps[0]["message"], NOT_LAUNCHED);
        request(&mut a, 3, "setBreakpoints", json!({"source": {"path": other}, "breakpoints": [{"line": 1}]}));
        out.take();

        request(&mut a, 4, "launch", json!({"program": bas, "qir": qir}));
        let r = out.take();
        assert_eq!(response(&r, "launch")["success"], true, "{r:?}");
        let changed: Vec<&Value> = events(&r, "breakpoint").into_iter().map(|e| &e["body"]["breakpoint"]).collect();
        assert_eq!(changed.len(), 3, "{changed:?}");
        assert_eq!((changed[0]["verified"].clone(), changed[0]["line"].clone()), (json!(true), json!(1)));
        // line 3 of prog.bas holds no statement of its own: moved to line 5
        assert_eq!((changed[1]["verified"].clone(), changed[1]["line"].clone()), (json!(true), json!(5)));
        assert_eq!(changed[2]["verified"], false);
        assert_eq!(changed[2]["message"], NOT_OURS);
        assert_eq!(a.breakpoint_places(), vec![(MAIN_FILE, 1), (MAIN_FILE, 5)]);

        // inc.bi line 3 is a breakpoint of its own; the program's stay
        request(&mut a, 5, "setBreakpoints", json!({"source": {"path": inc}, "breakpoints": [{"line": 3}]}));
        let r = out.take();
        let bps = &response(&r, "setBreakpoints")["body"]["breakpoints"];
        assert_eq!((bps[0]["verified"].clone(), bps[0]["line"].clone()), (json!(true), json!(3)), "{bps}");
        request(&mut a, 6, "setBreakpoints", json!({"source": {"path": other}, "breakpoints": [{"line": 2}]}));
        assert_eq!(a.breakpoint_places(), vec![(MAIN_FILE, 1), (MAIN_FILE, 5), (1, 3)]);
        // clearing the program's file keeps inc.bi's
        request(&mut a, 7, "setBreakpoints", json!({"source": {"path": bas}, "breakpoints": []}));
        assert_eq!(a.breakpoint_places(), vec![(1, 3)]);
        // past the last statement of a file
        out.take();
        request(&mut a, 8, "setBreakpoints", json!({"source": {"path": inc}, "breakpoints": [{"line": 40}]}));
        let r = out.take();
        assert_eq!(response(&r, "setBreakpoints")["body"]["breakpoints"][0]["message"], NO_STATEMENT);
        a.finish();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A stop inside an included file shows that file and its line.
    #[test]
    fn stack_frames_name_included_files() {
        let dir = work_dir("stack");
        let (bas, inc, qir) = program_with_include(&dir);
        let (mut a, out) = adapter();
        request(&mut a, 1, "launch", json!({"program": bas, "qir": qir}));
        let stop = json!({"t": "stopped", "reason": "breakpoint", "line": 4, "file": 1, "globals": [],
            "frames": [{"func": "SUB_S", "line": 4, "file": 1, "locals": []}, {"func": "main", "line": 5, "locals": []}]});
        a.handle(In::Child(stop));
        out.take();
        request(&mut a, 2, "stackTrace", json!({"threadId": 1}));
        let r = out.take();
        let frames = &response(&r, "stackTrace")["body"]["stackFrames"];
        assert_eq!(frames[0]["line"], 4);
        assert!(same_file(Path::new(frames[0]["source"]["path"].as_str().unwrap()), &inc), "{frames}");
        assert_eq!(frames[0]["source"]["name"], "inc.bi");
        assert_eq!(frames[1]["line"], 5);
        assert!(same_file(Path::new(frames[1]["source"]["path"].as_str().unwrap()), &bas), "{frames}");
        a.finish();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// One `terminated` per session; after a terminate request the killed
    /// debuggee gets no made-up exit code.
    #[test]
    fn terminate_sends_one_terminated_event() {
        let (mut a, out) = adapter();
        request(&mut a, 1, "initialize", json!({}));
        assert!(request(&mut a, 2, "terminate", json!({})));
        assert!(a.handle(In::Exited { code: 1, from_program: false }));
        let r = out.take();
        assert_eq!(events(&r, "terminated").len(), 1, "{r:?}");
        assert!(events(&r, "exited").is_empty(), "{r:?}");
    }

    /// The program's own code is reported as it is (SYSTEM -1, SYSTEM 300).
    #[test]
    fn exited_reports_the_programs_code() {
        for code in [-1i64, 300, 0] {
            let (mut a, out) = adapter();
            a.handle(In::Exited { code, from_program: true });
            let r = out.take();
            let exited = events(&r, "exited");
            assert_eq!(exited.len(), 1);
            assert_eq!(exited[0]["body"]["exitCode"], code);
            assert_eq!(events(&r, "terminated").len(), 1);
        }
        // a code the program gave before a terminate request took effect is still told
        let (mut a, out) = adapter();
        request(&mut a, 1, "terminate", json!({}));
        a.handle(In::Exited { code: 7, from_program: true });
        let r = out.take();
        assert_eq!(events(&r, "exited")[0]["body"]["exitCode"], 7);
        assert_eq!(events(&r, "terminated").len(), 1);
    }

    /// A message that is not JSON, or not a DAP message, is reported and the
    /// session goes on.
    #[test]
    fn malformed_messages_do_not_end_the_session() {
        let (mut a, out) = adapter();
        assert!(a.handle(In::Malformed("expected value at line 1".into())));
        assert!(a.handle(In::Client(json!(42))));
        assert!(a.handle(In::Client(json!({"seq": 1, "type": "request"}))));
        assert!(request(&mut a, 2, "threads", json!({})));
        let r = out.take();
        assert_eq!(events(&r, "output").len(), 2, "{r:?}");
        assert_eq!(r.iter().filter(|m| m["type"] == "response" && m["success"] == false).count(), 1, "{r:?}");
        assert_eq!(response(&r, "threads")["success"], true);
    }

    /// `program`, `qir` and `input` are taken from the launch's cwd; `args`
    /// is a list of strings (one string is one argument).
    #[test]
    fn relative_paths_and_program_arguments() {
        let dir = work_dir("launch");
        program_with_include(&dir);
        let (mut a, _out) = adapter();
        let l = a.prepare(&json!({"program": "prog.bas", "qir": "prog.qir", "cwd": dir, "input": "in.txt", "args": ["a", "b c"]})).unwrap();
        assert!(l.program.is_absolute());
        assert!(same_file(&l.program, &dir.join("prog.bas")), "{}", l.program.display());
        assert!(same_file(&l.cwd, &dir));
        assert_eq!(l.input, Some(absolute_in(&dir, "in.txt")));
        assert_eq!(l.args, vec!["a".to_string(), "b c".to_string()]);
        assert!(same_file(&a.files[1], &dir.join("inc.bi")));

        let cmd = debuggee_args(&l, 4000);
        let sep = cmd.iter().position(|a| a.to_str() == Some("--")).unwrap();
        assert_eq!(cmd[sep + 1], OsString::from(l.program.with_extension(std::env::consts::EXE_EXTENSION)));
        assert_eq!(cmd[sep + 2..], [OsString::from("a"), OsString::from("b c")]);
        assert!(cmd[..sep].iter().any(|a| a.to_str() == Some("--exit-file")));

        assert_eq!(program_args(&json!("one two")).unwrap(), vec!["one two".to_string()]);
        assert!(program_args(&Value::Null).unwrap().is_empty());
        assert!(program_args(&json!(5)).is_err());
        assert!(program_args(&json!(["x", 1])).is_err());
        a.finish();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn work_dirs_are_fresh_and_distinct() {
        let a = fresh_temp_dir("qb-dap-unit").unwrap();
        let b = fresh_temp_dir("qb-dap-unit").unwrap();
        assert_ne!(a, b);
        assert!(a.is_dir() && b.is_dir());
        assert_eq!(std::fs::read_dir(&a).unwrap().count(), 0);
        let _ = std::fs::remove_dir(&a);
        let _ = std::fs::remove_dir(&b);
    }
}

//! The DAP side: talks to the client on stdio and drives one debuggee process.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::names::{Casing, func_name, var_name};
use crate::protocol::{read_message, write_message};
use crate::snapshot::{Snapshot, Val, text};

enum In {
    Client(Value),
    ClientGone,
    Child(Value),
    Output(&'static str, String),
    Exited(i32),
}

#[derive(Clone, Debug, Default)]
struct Launch {
    program: PathBuf,
    qir_file: PathBuf,
    cwd: PathBuf,
    stop_on_entry: bool,
    no_debug: bool,
    input: Option<PathBuf>,
}

struct Adapter {
    seq: i64,
    out: std::io::Stdout,
    tx: Sender<In>,
    launch: Option<Launch>,
    configured: bool,
    started: bool,
    /// lines that start a statement (from the QIR line table)
    stmt_lines: BTreeSet<u32>,
    breakpoints: Vec<u32>,
    casing: Casing,
    conn: Option<TcpStream>,
    child: Option<Arc<Mutex<Child>>>,
    snapshot: Option<Snapshot>,
    /// variablesReference - 1 -> (display name, value)
    refs: Vec<Vec<(String, Val)>>,
    temp: Option<PathBuf>,
}

/// Runs the adapter on stdin/stdout; returns the process exit code.
pub fn run_stdio() -> i32 {
    let (tx, rx) = channel();
    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mut r = BufReader::new(std::io::stdin());
            loop {
                match read_message(&mut r) {
                    Ok(Some(m)) => {
                        if tx.send(In::Client(m)).is_err() {
                            break;
                        }
                    }
                    _ => {
                        let _ = tx.send(In::ClientGone);
                        break;
                    }
                }
            }
        });
    }
    let mut a = Adapter {
        seq: 0,
        out: std::io::stdout(),
        tx,
        launch: None,
        configured: false,
        started: false,
        stmt_lines: BTreeSet::new(),
        breakpoints: Vec::new(),
        casing: Casing::default(),
        conn: None,
        child: None,
        snapshot: None,
        refs: Vec::new(),
        temp: None,
    };
    for msg in rx {
        let go_on = match msg {
            In::Client(m) => a.client(m),
            In::ClientGone => false,
            In::Child(m) => {
                a.child_msg(m);
                true
            }
            In::Output(cat, s) => {
                a.event("output", json!({"category": cat, "output": s}));
                true
            }
            In::Exited(code) => {
                a.event("exited", json!({"exitCode": code}));
                a.event("terminated", json!({}));
                true
            }
        };
        if !go_on {
            break;
        }
    }
    a.kill();
    if let Some(t) = a.temp.take() {
        let _ = std::fs::remove_dir_all(t);
    }
    0
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

fn same_file(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let (a, b) = (canon(a), canon(b));
    if cfg!(windows) { a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy()) } else { a == b }
}

impl Adapter {
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

    fn to_child(&mut self, msg: Value) {
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
                    self.try_start();
                }
                Err(e) => self.fail(&req, &e),
            },
            "setBreakpoints" => {
                let path = args["source"]["path"].as_str().map(PathBuf::from);
                let lines: Vec<u64> = args["breakpoints"].as_array().map(|a| a.iter().filter_map(|b| b["line"].as_u64()).collect()).unwrap_or_default();
                let ours = match (&path, &self.launch) {
                    (Some(p), Some(l)) => same_file(p, &l.program),
                    // before launch: assume the program file
                    (Some(_), None) => true,
                    _ => false,
                };
                let mut actual = Vec::new();
                let bps: Vec<Value> = lines
                    .iter()
                    .map(|&l| {
                        let l = l as u32;
                        let found = if !ours {
                            None
                        } else if self.stmt_lines.is_empty() {
                            Some(l)
                        } else {
                            self.stmt_lines.range(l..).next().copied()
                        };
                        match found {
                            Some(line) => {
                                actual.push(line);
                                json!({"verified": true, "line": line})
                            }
                            None => json!({"verified": false, "line": l, "message": if ours { "no statement at or after this line" } else { "breakpoints work in the launched program file only" }}),
                        }
                    })
                    .collect();
                if ours {
                    self.breakpoints = actual.clone();
                    self.to_child(json!({"t": "bp", "lines": actual}));
                }
                self.respond(&req, json!({"breakpoints": bps}));
            }
            "setExceptionBreakpoints" | "setFunctionBreakpoints" => self.respond(&req, json!({"breakpoints": []})),
            "configurationDone" => {
                self.configured = true;
                self.respond(&req, json!({}));
                self.try_start();
            }
            "threads" => self.respond(&req, json!({"threads": [{"id": 1, "name": "main"}]})),
            "stackTrace" => {
                let program = self.launch.as_ref().map(|l| l.program.clone()).unwrap_or_default();
                let name = program.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                let frames: Vec<Value> = self
                    .snapshot
                    .as_ref()
                    .map(|s| s.frames.as_slice())
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        json!({
                            "id": i, "name": func_name(&f.func, &self.casing), "line": f.line, "column": 1,
                            "source": {"name": name, "path": program.to_string_lossy()},
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
                self.to_child(json!({"t": "resume", "mode": mode}));
                self.respond(&req, json!({"allThreadsContinued": true}));
            }
            "pause" => {
                self.to_child(json!({"t": "pause"}));
                self.respond(&req, json!({}));
            }
            "terminate" => {
                self.kill();
                self.respond(&req, json!({}));
                self.event("terminated", json!({}));
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
            } else if let Some(r) = rest.strip_prefix('(') {
                let end = r.find(')')?;
                let idx: Option<Vec<i64>> = r[..end].split(',').map(|x| x.trim().parse().ok()).collect();
                val = val.element(&idx?)?.clone();
                rest = r[end + 1..].trim();
            } else {
                return None;
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
        let program = PathBuf::from(args["program"].as_str().ok_or("launch: \"program\" (the .bas file) is required")?);
        let dir = program.parent().map(Path::to_path_buf).unwrap_or_default();
        if let Ok(bytes) = std::fs::read(&program) {
            self.casing = Casing::from_source(&qb_syntax::encoding::decode(&bytes).0);
        }
        let temp = std::env::temp_dir().join(format!("qb-dap-{}", std::process::id()));
        std::fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
        self.temp = Some(temp.clone());
        let qir_text = match args["qir"].as_str() {
            Some(q) => std::fs::read_to_string(q).map_err(|e| format!("cannot read {q}: {e}"))?,
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
        self.stmt_lines = qir_text.lines().filter_map(|l| l.strip_prefix("line ")).filter_map(|n| n.trim().parse().ok()).collect();
        let qir_file = temp.join("program.qir");
        std::fs::write(&qir_file, &qir_text).map_err(|e| e.to_string())?;
        Ok(Launch {
            program,
            qir_file,
            cwd: args["cwd"].as_str().map(PathBuf::from).unwrap_or(dir),
            stop_on_entry: args["stopOnEntry"].as_bool().unwrap_or(false),
            no_debug: args["noDebug"].as_bool().unwrap_or(false),
            input: args["input"].as_str().map(PathBuf::from),
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
            self.event("terminated", json!({}));
        }
    }

    fn start(&mut self, l: &Launch) -> Result<(), String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut cmd = Command::new(exe);
        cmd.arg("--debuggee").arg(&l.qir_file).arg(port.to_string());
        if l.no_debug {
            cmd.arg("--no-debug");
        }
        if l.stop_on_entry {
            cmd.arg("--stop-on-entry");
        }
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
                while let Ok(n) = pipe.read(&mut buf) {
                    if n == 0 || tx.send(In::Output(cat, text(&buf[..n]))).is_err() {
                        break;
                    }
                }
            });
        }
        let child = Arc::new(Mutex::new(child));
        self.child = Some(child.clone());
        if !l.no_debug {
            listener.set_nonblocking(true).map_err(|e| e.to_string())?;
            let started = Instant::now();
            let conn = loop {
                match listener.accept() {
                    Ok((c, _)) => break c,
                    Err(_) if started.elapsed() < Duration::from_secs(20) => {
                        if let Ok(Some(_)) = child.lock().unwrap().try_wait() {
                            break Err("the debuggee exited before connecting".to_string())?;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => return Err(format!("debuggee did not connect: {e}")),
                }
            };
            conn.set_nonblocking(false).map_err(|e| e.to_string())?;
            let reader = conn.try_clone().map_err(|e| e.to_string())?;
            self.conn = Some(conn);
            let bps = self.breakpoints.clone();
            self.to_child(json!({"t": "bp", "lines": bps}));
            self.to_child(json!({"t": "go"}));
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
        std::thread::spawn(move || {
            loop {
                let status = child.lock().unwrap().try_wait();
                match status {
                    Ok(Some(s)) => {
                        // let the output threads drain first
                        std::thread::sleep(Duration::from_millis(50));
                        let _ = tx.send(In::Exited(s.code().unwrap_or(1)));
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

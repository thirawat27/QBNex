//! Drives the `qb-dap` binary over stdio with DAP framing, on QIR fixtures
//! of qb-exec (so the tests need no compiler).

use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Duration;

use serde_json::{Value, json};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(case: &str, qir: &str) -> (PathBuf, PathBuf) {
    let r = repo();
    (r.join("tests/conformance/cases").join(case), r.join("crates/qb-exec/tests/fixtures/corpus").join(qir))
}

struct Dap {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Value>,
    seq: i64,
    /// messages read while waiting for something else
    pending: Vec<Value>,
    output: String,
}

impl Dap {
    fn start() -> Dap {
        let mut child = Command::new(env!("CARGO_BIN_EXE_qb-dap"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start qb-dap");
        let stdin = child.stdin.take().unwrap();
        let stdout: ChildStdout = child.stdout.take().unwrap();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let mut r = BufReader::new(stdout);
            while let Ok(Some(m)) = qb_dap::protocol::read_message(&mut r) {
                if tx.send(m).is_err() {
                    break;
                }
            }
        });
        Dap { child, stdin, rx, seq: 0, pending: Vec::new(), output: String::new() }
    }

    fn next(&mut self) -> Value {
        match self.rx.recv_timeout(Duration::from_secs(30)) {
            Ok(m) => {
                if m["event"] == "output" {
                    self.output.push_str(m["body"]["output"].as_str().unwrap_or(""));
                }
                m
            }
            Err(RecvTimeoutError::Timeout) => panic!("timeout; output so far: {}", self.output),
            Err(e) => panic!("adapter closed: {e}; output: {}", self.output),
        }
    }

    fn request(&mut self, command: &str, arguments: Value) -> Value {
        self.seq += 1;
        let seq = self.seq;
        let msg = json!({"seq": seq, "type": "request", "command": command, "arguments": arguments});
        qb_dap::protocol::write_message(&mut self.stdin, &msg).unwrap();
        loop {
            let m = self.next();
            if m["type"] == "response" && m["request_seq"] == seq {
                assert_eq!(m["success"], true, "{command} failed: {m}");
                return m["body"].clone();
            }
            self.pending.push(m);
        }
    }

    fn event(&mut self, name: &str) -> Value {
        if let Some(i) = self.pending.iter().position(|m| m["event"] == name) {
            return self.pending.remove(i);
        }
        loop {
            let m = self.next();
            if m["event"] == name {
                return m;
            }
            self.pending.push(m);
        }
    }

    fn stopped(&mut self) -> String {
        self.event("stopped")["body"]["reason"].as_str().unwrap().to_string()
    }

    /// (frame names, lines)
    fn stack(&mut self) -> Vec<(String, u64)> {
        let st = self.request("stackTrace", json!({"threadId": 1}));
        st["stackFrames"].as_array().unwrap().iter().map(|f| (f["name"].as_str().unwrap().to_string(), f["line"].as_u64().unwrap())).collect()
    }

    /// name -> (value, variablesReference) of a scope of a frame ("Locals"/"Module", "Globals").
    fn scope(&mut self, frame: u64, which: usize) -> Vec<(String, String, u64)> {
        let sc = self.request("scopes", json!({"frameId": frame}));
        let r = sc["scopes"][which]["variablesReference"].as_u64().unwrap();
        self.vars(r)
    }

    fn vars(&mut self, r: u64) -> Vec<(String, String, u64)> {
        let v = self.request("variables", json!({"variablesReference": r}));
        v["variables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| (x["name"].as_str().unwrap().to_string(), x["value"].as_str().unwrap().to_string(), x["variablesReference"].as_u64().unwrap()))
            .collect()
    }

    fn eval(&mut self, expr: &str) -> String {
        self.eval_in(expr, 0)
    }

    fn eval_in(&mut self, expr: &str, frame: u64) -> String {
        self.request("evaluate", json!({"expression": expr, "frameId": frame, "context": "watch"}))["result"].as_str().unwrap().to_string()
    }

    fn launch(&mut self, bas: &Path, qir: &Path, extra: Value, breakpoints: &[u64]) {
        let init = self.request("initialize", json!({"adapterID": "qbnex", "linesStartAt1": true, "columnsStartAt1": true}));
        assert_eq!(init["supportsConfigurationDoneRequest"], true);
        self.event("initialized");
        let mut args = json!({"program": bas, "qir": qir});
        for (k, v) in extra.as_object().unwrap() {
            args[k] = v.clone();
        }
        self.request("launch", args);
        let bps: Vec<Value> = breakpoints.iter().map(|l| json!({"line": l})).collect();
        let r = self.request("setBreakpoints", json!({"source": {"path": bas}, "breakpoints": bps}));
        for b in r["breakpoints"].as_array().unwrap() {
            assert_eq!(b["verified"], true, "{r}");
        }
        self.request("configurationDone", json!({}));
    }

    fn finish(mut self) -> String {
        let code = self.event("exited")["body"]["exitCode"].as_i64().unwrap();
        assert_eq!(code, 0, "output: {}", self.output);
        self.event("terminated");
        self.request("disconnect", json!({}));
        let _ = self.stdin.flush();
        for _ in 0..200 {
            if self.child.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(self.child.try_wait().unwrap().is_some(), "adapter did not exit");
        // drain the remaining output events
        while let Ok(m) = self.rx.try_recv() {
            if m["event"] == "output" {
                self.output.push_str(m["body"]["output"].as_str().unwrap_or(""));
            }
        }
        self.output
    }
}

fn find<'a>(vars: &'a [(String, String, u64)], name: &str) -> &'a (String, String, u64) {
    vars.iter().find(|v| v.0 == name).unwrap_or_else(|| panic!("{name} not in {vars:?}"))
}

#[test]
fn breakpoints_steps_stack_and_variables() {
    let (bas, qir) = fixture("control/subs_functions.bas", "control_subs_functions.qir");
    let mut d = Dap::start();
    // line 16: `x = x + 1` in SUB Bump
    d.launch(&bas, &qir, json!({}), &[16]);

    assert_eq!(d.stopped(), "breakpoint");
    assert_eq!(d.stack(), vec![("Bump".to_string(), 16), ("<module>".to_string(), 5)]);
    let locals = d.scope(0, 0);
    assert_eq!(find(&locals, "x%").1, "5");
    let globals = d.scope(0, 1);
    assert_eq!(find(&globals, "a%").1, "5");
    assert_eq!(find(&globals, "total&").1, "0");
    assert_eq!(d.eval("x"), "5");
    assert_eq!(d.eval("A%"), "5");

    // step over leaves the SUB: next statement is line 6 in the module
    d.request("next", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "step");
    assert_eq!(d.stack(), vec![("<module>".to_string(), 6)]);

    // continue: second call, `Bump (a%)` passes a copy of a% = 6
    d.request("continue", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "breakpoint");
    assert_eq!(d.stack(), vec![("Bump".to_string(), 16), ("<module>".to_string(), 7)]);
    assert_eq!(d.eval("x%"), "6");

    // step out: back in the module after the call
    d.request("stepOut", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "step");
    assert_eq!(d.stack()[0], ("<module>".to_string(), 8));

    // step in at line 9 (`PRINT Fact&(10)`) enters the recursive FUNCTION
    d.request("setBreakpoints", json!({"source": {"path": bas}, "breakpoints": [{"line": 9}]}));
    d.request("continue", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "breakpoint");
    d.request("stepIn", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "step");
    assert_eq!(d.stack(), vec![("Fact".to_string(), 20), ("<module>".to_string(), 9)]);
    // Line 20 is one single-line IF holding several statements: step in
    // statement by statement until the recursive call is entered.
    let mut st = d.stack();
    for _ in 0..8 {
        if st.len() == 3 {
            break;
        }
        d.request("stepIn", json!({"threadId": 1}));
        assert_eq!(d.stopped(), "step");
        st = d.stack();
    }
    assert_eq!(st.len(), 3, "{st:?}");
    assert_eq!(st[0].0, "Fact");
    assert_eq!(d.eval("n"), "9");
    // step out returns to the outer Fact (depth 2)
    d.request("stepOut", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "step");
    assert_eq!(d.stack().len(), 2);

    // STATIC variable of SUB Counter, then run to the end
    d.request("setBreakpoints", json!({"source": {"path": bas}, "breakpoints": [{"line": 29}]}));
    d.request("continue", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "breakpoint");
    assert_eq!(d.eval("c"), "1");
    assert_eq!(d.eval("total"), "0");
    d.request("setBreakpoints", json!({"source": {"path": bas}, "breakpoints": []}));
    d.request("continue", json!({"threadId": 1}));
    let out = d.finish();
    assert!(out.contains("3628800"), "{out}");
    assert!(out.contains("abab"), "{out}");
}

#[test]
fn udt_and_array_variables() {
    let (bas, qir) = fixture("data/udt.bas", "data_udt.qir");
    let mut d = Dap::start();
    // line 20: `PRINT pts(1).x; pts(0).x`
    d.launch(&bas, &qir, json!({}), &[20]);
    assert_eq!(d.stopped(), "breakpoint");
    let globals = d.scope(0, 1);
    let b = find(&globals, "b").clone();
    assert_eq!(b.1, "{BOX}");
    let fields = d.vars(b.2);
    let lo = find(&fields, "lo").clone();
    let lo_fields = d.vars(lo.2);
    assert_eq!(find(&lo_fields, "x").1, "1");
    assert_eq!(find(&lo_fields, "y").1, "1.5");
    assert_eq!(find(&lo_fields, "tag").1, "\"lo  \"");
    assert_eq!(find(&fields, "id").1, "1234567890123");
    let pts = find(&globals, "pts").clone();
    assert_eq!(pts.1, "(0 TO 2) [3]");
    let elems = d.vars(pts.2);
    let p1 = find(&elems, "(1)").clone();
    assert_eq!(find(&d.vars(p1.2), "x").1, "42");
    assert_eq!(d.eval("pts(1).x"), "42");
    assert_eq!(d.eval("b.hi.tag"), "\"high\"");
    d.request("continue", json!({"threadId": 1}));
    let out = d.finish();
    assert!(out.contains("42"), "{out}");
}

#[test]
fn stop_on_entry_and_register_arrays() {
    let (bas, qir) = fixture("data/arrays.bas", "data_arrays.qir");
    let mut d = Dap::start();
    d.launch(&bas, &qir, json!({"stopOnEntry": true}), &[]);
    assert_eq!(d.stopped(), "entry");
    let first = d.stack()[0].1;
    assert!(first >= 1);
    d.request("next", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "step");
    assert!(d.stack()[0].1 >= first);
    d.request("continue", json!({"threadId": 1}));
    d.finish();
}

/// The real compiler: `QB_COMPILER`, else `qb(.exe)` at the repository root.
fn real_compiler() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("QB_COMPILER") {
        return Some(PathBuf::from(p));
    }
    let p = repo().join(if cfg!(windows) { "qb.exe" } else { "qb" });
    p.is_file().then_some(p)
}

const DEMO: &str = "TYPE Pt
    x AS INTEGER
    y AS INTEGER
END TYPE
DIM SHARED total AS LONG
DIM p AS Pt
DIM a(3) AS INTEGER
p.x = 3: p.y = 4
FOR i% = 1 TO 3
    a(i%) = i% * 10
    Add i%
NEXT
PRINT total; Hyp!(p)
SUB Add (n AS INTEGER)
    total = total + n
END SUB
FUNCTION Hyp! (q AS Pt)
    Hyp! = SQR(q.x * q.x + q.y * q.y)
END FUNCTION
";

/// End to end with `qb --ir`: the adapter compiles the program itself.
#[test]
fn real_compiler_end_to_end() {
    let Some(qb) = real_compiler() else {
        eprintln!("skipped: no qb compiler (set QB_COMPILER or build qb at the repository root)");
        return;
    };
    let dir = std::env::temp_dir().join(format!("qb-dap-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bas = dir.join("demo.bas");
    std::fs::write(&bas, DEMO).unwrap();

    let mut d = Dap::start();
    let init = d.request("initialize", json!({"adapterID": "qbnex"}));
    assert_eq!(init["supportsConfigurationDoneRequest"], true);
    d.event("initialized");
    d.request("launch", json!({"program": bas, "compiler": qb}));
    // line 15: `total = total + n` in SUB Add; line 11 has no breakpoint
    let r = d.request("setBreakpoints", json!({"source": {"path": bas}, "breakpoints": [{"line": 15}, {"line": 4}]}));
    assert_eq!(r["breakpoints"][0]["verified"], true, "{r}");
    // line 4 (END TYPE) holds no statement: moved to the next one
    assert!(r["breakpoints"][1]["line"].as_u64().unwrap() > 4, "{r}");
    d.request("setBreakpoints", json!({"source": {"path": bas}, "breakpoints": [{"line": 15}]}));
    d.request("configurationDone", json!({}));

    assert_eq!(d.stopped(), "breakpoint");
    assert_eq!(d.stack(), vec![("Add".to_string(), 15), ("<module>".to_string(), 11)]);
    assert_eq!(d.eval("n"), "1");
    assert_eq!(d.eval("total"), "0");
    assert_eq!(d.eval_in("p.x", 1), "3");
    assert_eq!(d.eval_in("a(1)", 1), "10");
    assert_eq!(d.eval_in("i%", 1), "1");

    d.request("continue", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "breakpoint");
    assert_eq!(d.eval("n"), "2");
    assert_eq!(d.eval("total"), "1");

    d.request("stepOut", json!({"threadId": 1}));
    assert_eq!(d.stopped(), "step");
    assert_eq!(d.stack().len(), 1);

    d.request("setBreakpoints", json!({"source": {"path": bas}, "breakpoints": []}));
    d.request("continue", json!({"threadId": 1}));
    let out = d.finish();
    assert!(out.contains(" 6  5"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}
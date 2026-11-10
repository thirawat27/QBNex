//! Drives the `qb-lsp` binary over stdio with real JSON-RPC framing.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
    notifications: Vec<Value>,
}

impl Client {
    fn start() -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_qb-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start qb-lsp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Client { child, stdin, stdout, next_id: 1, notifications: Vec::new() }
    }

    fn send(&mut self, msg: Value) {
        let body = serde_json::to_string(&msg).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.stdin.flush().unwrap();
    }

    fn read(&mut self) -> Value {
        let mut len = 0usize;
        loop {
            let mut line = String::new();
            assert!(self.stdout.read_line(&mut line).unwrap() > 0, "server closed stdout");
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(v) = line.strip_prefix("Content-Length: ") {
                len = v.parse().unwrap();
            }
        }
        let mut buf = vec![0; len];
        self.stdout.read_exact(&mut buf).unwrap();
        serde_json::from_slice(&buf).unwrap()
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let msg = self.read();
            if msg.get("id") == Some(&json!(id)) {
                assert!(msg.get("error").is_none(), "{method} failed: {msg}");
                return msg["result"].clone();
            }
            self.notifications.push(msg);
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    fn wait_notification(&mut self, method: &str) -> Value {
        if let Some(i) = self.notifications.iter().position(|n| n["method"] == method) {
            return self.notifications.remove(i);
        }
        loop {
            let msg = self.read();
            if msg["method"] == method {
                return msg;
            }
            self.notifications.push(msg);
        }
    }
}

fn workspace(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qb-lsp-test-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const INC: &str = "TYPE pt\n    x AS INTEGER\n    y AS INTEGER\nEND TYPE\nSUB helper (n)\n    PRINT n\nEND SUB\n";

const MAIN: &str = "'$INCLUDE: 'inc.bi'\n\
'$INCLUDE: 'missing.bi'\n\
CONST MAXN = 10\n\
DIM SHARED p AS pt\n\
start:\n\
helper MAXN\n\
p.x = MAXN\n\
GOTO start\n\
for i=1 to 3\n\
print i\n\
next\n\
END IF\n";

fn pos(line: u32, character: u32) -> Value {
    json!({"line": line, "character": character})
}

#[test]
fn full_session() {
    let dir = workspace("full");
    std::fs::write(dir.join("inc.bi"), INC).unwrap();
    let main_path = dir.join("main.bas");
    std::fs::write(&main_path, MAIN).unwrap();
    let uri = qb_lsp::path_to_uri(&main_path);
    let inc_uri = qb_lsp::path_to_uri(&dir.join("inc.bi"));

    let mut c = Client::start();
    let init = c.request("initialize", json!({"processId": null, "rootUri": null, "capabilities": {}}));
    let caps = &init["capabilities"];
    for cap in ["documentSymbolProvider", "definitionProvider", "referencesProvider", "hoverProvider", "completionProvider", "documentFormattingProvider"] {
        assert!(!caps[cap].is_null(), "missing {cap}: {caps}");
    }
    assert_eq!(init["serverInfo"]["name"], "qb-lsp");
    c.notify("initialized", json!({}));

    c.notify("textDocument/didOpen", json!({"textDocument": {"uri": uri, "languageId": "qbasic", "version": 1, "text": MAIN}}));
    let diags = c.wait_notification("textDocument/publishDiagnostics");
    let messages: Vec<&str> = diags["params"]["diagnostics"].as_array().unwrap().iter().map(|d| d["message"].as_str().unwrap()).collect();
    assert!(messages.contains(&"END IF without IF"), "{messages:?}");
    assert!(messages.contains(&"include file not found: missing.bi"), "{messages:?}");
    assert_eq!(messages.len(), 2, "{messages:?}");

    // Document symbols.
    let syms = c.request("textDocument/documentSymbol", json!({"textDocument": {"uri": uri}}));
    let names: Vec<&str> = syms.as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["MAXN", "start"], "{syms}");

    // Definition of `helper` (line 5) is in inc.bi, line 4.
    let def = c.request("textDocument/definition", json!({"textDocument": {"uri": uri}, "position": pos(5, 2)}));
    assert_eq!(def[0]["uri"], inc_uri.as_str(), "{def}");
    assert_eq!(def[0]["range"]["start"], pos(4, 4));

    // `p.x`: the member `x` resolves to the TYPE field in inc.bi.
    let def = c.request("textDocument/definition", json!({"textDocument": {"uri": uri}, "position": pos(6, 2)}));
    assert_eq!(def[0]["uri"], inc_uri.as_str(), "{def}");
    assert_eq!(def[0]["range"]["start"], pos(1, 4));

    // Label target of GOTO.
    let def = c.request("textDocument/definition", json!({"textDocument": {"uri": uri}, "position": pos(7, 6)}));
    assert_eq!(def[0]["range"]["start"], pos(4, 0), "{def}");

    // References to MAXN: declaration + 2 uses.
    let refs = c.request("textDocument/references", json!({"textDocument": {"uri": uri}, "position": pos(2, 7), "context": {"includeDeclaration": true}}));
    assert_eq!(refs.as_array().unwrap().len(), 3, "{refs}");
    let refs = c.request("textDocument/references", json!({"textDocument": {"uri": uri}, "position": pos(2, 7), "context": {"includeDeclaration": false}}));
    assert_eq!(refs.as_array().unwrap().len(), 2, "{refs}");

    // Hover: built-in keyword and user TYPE.
    let h = c.request("textDocument/hover", json!({"textDocument": {"uri": uri}, "position": pos(9, 1)}));
    let v = h["contents"]["value"].as_str().unwrap();
    assert!(v.contains("**PRINT**"), "{v}");
    let h = c.request("textDocument/hover", json!({"textDocument": {"uri": uri}, "position": pos(3, 17)}));
    let v = h["contents"]["value"].as_str().unwrap();
    assert!(v.contains("TYPE pt") && v.contains("inc.bi"), "{v}");

    // Completion: user symbols from the include and built-ins.
    let comp = c.request("textDocument/completion", json!({"textDocument": {"uri": uri}, "position": pos(9, 0)}));
    let labels: Vec<&str> = comp.as_array().unwrap().iter().map(|i| i["label"].as_str().unwrap()).collect();
    for want in ["helper", "pt", "MAXN", "LEFT$", "_RGB32", "PRINT"] {
        assert!(labels.contains(&want), "missing {want}");
    }

    // Formatting through qb-fmt.
    let edits = c.request("textDocument/formatting", json!({"textDocument": {"uri": uri}, "options": {"tabSize": 2, "insertSpaces": true}}));
    let text = edits[0]["newText"].as_str().unwrap();
    assert!(text.contains("FOR i = 1 TO 3\n  PRINT i\nNEXT\n"), "{text}");

    // Edit: fix the stray END IF; diagnostics update.
    let fixed = MAIN.replace("END IF\n", "");
    c.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": fixed}]}));
    let diags = c.wait_notification("textDocument/publishDiagnostics");
    assert_eq!(diags["params"]["version"], 2);
    assert_eq!(diags["params"]["diagnostics"].as_array().unwrap().len(), 1, "{diags}");

    c.notify("textDocument/didClose", json!({"textDocument": {"uri": uri}}));
    let diags = c.wait_notification("textDocument/publishDiagnostics");
    assert!(diags["params"]["diagnostics"].as_array().unwrap().is_empty());

    let r = c.request("shutdown", Value::Null);
    assert!(r.is_null());
    c.notify("exit", Value::Null);
    let status = wait(&mut c.child);
    assert!(status, "server did not exit cleanly");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unknown_method_and_garbage_text() {
    let mut c = Client::start();
    c.request("initialize", json!({"processId": null, "rootUri": null, "capabilities": {}}));
    c.notify("initialized", json!({}));
    let uri = "untitled:Untitled-1";
    let text = "\u{0}\"\u{1a}SUB\n_ _\n$\nEND SUB END SUB ((((\n#é";
    c.notify("textDocument/didOpen", json!({"textDocument": {"uri": uri, "languageId": "qbasic", "version": 1, "text": text}}));
    c.wait_notification("textDocument/publishDiagnostics");
    for line in 0..5 {
        for ch in 0..12 {
            c.request("textDocument/hover", json!({"textDocument": {"uri": uri}, "position": pos(line, ch)}));
            c.request("textDocument/definition", json!({"textDocument": {"uri": uri}, "position": pos(line, ch)}));
        }
    }
    c.request("textDocument/documentSymbol", json!({"textDocument": {"uri": uri}}));
    let id = 999;
    c.send(json!({"jsonrpc": "2.0", "id": id, "method": "workspace/unknownThing", "params": {}}));
    let resp = loop {
        let m = c.read();
        if m.get("id") == Some(&json!(id)) {
            break m;
        }
    };
    assert_eq!(resp["error"]["code"], -32601);
    c.request("shutdown", Value::Null);
    c.notify("exit", Value::Null);
    assert!(wait(&mut c.child));
}

fn wait(child: &mut Child) -> bool {
    for _ in 0..100 {
        if let Some(s) = child.try_wait().unwrap() {
            return s.success();
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    false
}

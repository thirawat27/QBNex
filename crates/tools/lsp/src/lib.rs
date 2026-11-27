//! Language server for QBasic / QB64 / QBNex over stdio.
//!
//! Features: diagnostics (syntax errors, block mismatches, missing include
//! files), document symbols (SUB/FUNCTION/TYPE with fields/CONST/labels/
//! referenced line numbers), go-to-definition and references across a file
//! and its `$INCLUDE`s, hover for built-in keywords, user symbols and library
//! procedures, completion (members after `x.`: a variable of TYPE `List`
//! offers `Add`, which method syntax turns into `List_Add`), and
//! whole-document formatting through `qb-fmt`.

pub mod analysis;
mod uri;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types as lt;
use qb_syntax::keywords::{self, KeywordKind};
use qb_syntax::{LineIndex, Parse, Severity, TokenKind};

use crate::analysis::{Analysis, Symbol, SymbolKind, analyze, base_name, dotted_before, dotted_span, word_at};
pub use crate::uri::{path_to_uri, uri_key, uri_to_path};

type BoxError = Box<dyn Error + Send + Sync>;

/// One analysed source file.
pub struct Doc {
    pub uri: String,
    pub path: Option<PathBuf>,
    pub parse: Parse,
    pub index: LineIndex,
    pub analysis: Analysis,
}

impl Doc {
    pub fn new(uri: String, text: &str) -> Self {
        let parse = qb_syntax::parse(text);
        let index = LineIndex::new(&parse.text);
        let analysis = analyze(&parse);
        let path = uri_to_path(&uri);
        Doc { uri, path, parse, index, analysis }
    }

    fn position(&self, offset: usize) -> lt::Position {
        let (line, character) = self.index.position(&self.parse.text, offset);
        lt::Position { line, character }
    }

    fn range(&self, r: &Range<usize>) -> lt::Range {
        lt::Range { start: self.position(r.start), end: self.position(r.end) }
    }

    fn offset(&self, p: lt::Position) -> usize {
        self.index.offset(&self.parse.text, p.line, p.character)
    }

    fn location(&self, r: &Range<usize>) -> Option<lt::Location> {
        Some(lt::Location { uri: self.uri.parse().ok()?, range: self.range(r) })
    }
}

/// The procedure a method call `x.Name` reaches.
enum Method {
    /// A SUB or FUNCTION in the file set.
    User(Rc<Doc>, Symbol),
    /// A standard library procedure.
    Library(&'static qb_syntax::library::LibraryProc),
}

/// Modification time and length of a file on disk.
type Stamp = (SystemTime, u64);

/// Server state.
pub struct Server {
    docs: HashMap<String, Rc<Doc>>,
    /// Files read from disk (includes that are not open), reused while their
    /// modification time and length are unchanged.
    disk: RefCell<HashMap<String, (Stamp, Rc<Doc>)>>,
    /// Workspace folders: the last place an `$INCLUDE` is looked for.
    roots: Vec<PathBuf>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

/// Largest message body accepted. A client-supplied `Content-Length` is not trusted
/// with an allocation: `Connection::stdio` reserved whatever the header said and
/// aborted the server on a wrong one.
pub const MAX_MESSAGE: usize = 64 * 1024 * 1024;

/// Reads one message; `Ok(None)` at end of input.
pub fn read_message(r: &mut impl std::io::BufRead) -> std::io::Result<Option<Message>> {
    use std::io::{Error, ErrorKind};
    match read_frame(r)? {
        Some(body) => serde_json::from_slice(&body).map(Some).map_err(|e| Error::new(ErrorKind::InvalidData, e)),
        None => Ok(None),
    }
}

/// Reads the body of one message without parsing it; `Ok(None)` at end of
/// input. An error here means the header framing is broken (the next message
/// cannot be found); a body that is not a message is answered by the caller.
pub fn read_frame(r: &mut impl std::io::BufRead) -> std::io::Result<Option<Vec<u8>>> {
    use std::io::{Error, ErrorKind};
    let mut len: Option<usize> = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if len.is_some() {
                break;
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(':')
            && k.trim().eq_ignore_ascii_case("Content-Length")
        {
            len = Some(v.trim().parse().map_err(|_| Error::new(ErrorKind::InvalidData, format!("bad Content-Length `{}`", v.trim())))?);
        }
    }
    let len = len.unwrap_or(0);
    if len > MAX_MESSAGE {
        return Err(Error::new(ErrorKind::InvalidData, format!("LSP message of {len} bytes is too large")));
    }
    let mut buf = vec![0; len];
    r.read_exact(&mut buf)?;
    Ok(Some(buf))
}

/// Parses a message body. One that is not a request, notification or
/// response gives instead the JSON-RPC error response to send back: -32700
/// (parse error) for text that is not JSON, -32600 (invalid request) for JSON
/// that is not a message. Its id is the message's when a request was meant
/// (a `method` and a usable id), else null.
pub fn parse_body(body: &[u8]) -> Result<Message, serde_json::Value> {
    use serde_json::Value;
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => return Err(error_response(Value::Null, -32700, &format!("parse error: {e}"))),
    };
    let id = match &v["id"] {
        id @ (Value::Number(_) | Value::String(_)) if !v["method"].is_null() => id.clone(),
        _ => Value::Null,
    };
    serde_json::from_value(v).map_err(|e| error_response(id, -32600, &format!("invalid request: {e}")))
}

fn error_response(id: serde_json::Value, code: i32, message: &str) -> serde_json::Value {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// Writes one message given as JSON.
fn write_raw(w: &mut impl std::io::Write, msg: &serde_json::Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(msg)?;
    write!(w, "Content-Length: {}\r\n\r\n", body.len())?;
    w.write_all(&body)?;
    w.flush()
}

/// `exit` came without a `shutdown` before it: the LSP specification has the
/// server end with exit code 1 then.
#[derive(Debug)]
pub struct ExitWithoutShutdown;

impl std::fmt::Display for ExitWithoutShutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("exit without shutdown")
    }
}

impl Error for ExitWithoutShutdown {}

/// Runs the server on stdio until the client exits.
pub fn run_stdio() -> Result<(), BoxError> {
    let (conn, client) = Connection::memory();
    let Connection { sender: to_server, receiver: from_server } = client;
    let reader = std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        loop {
            // Only broken header framing or the end of input ends the session:
            // a body that is not a message is answered and the next one read.
            match read_frame(&mut stdin) {
                Ok(Some(body)) => match parse_body(&body) {
                    Ok(msg) => {
                        let exit = matches!(&msg, Message::Notification(n) if n.method == "exit");
                        if to_server.send(msg).is_err() || exit {
                            break;
                        }
                    }
                    Err(reply) => {
                        // (the writer takes the lock for each whole message, so the
                        // two never interleave)
                        if write_raw(&mut std::io::stdout().lock(), &reply).is_err() {
                            break;
                        }
                    }
                },
                Ok(None) => break,
                Err(e) => {
                    eprintln!("qb-lsp: {e}");
                    break;
                }
            }
        }
    });
    let writer = std::thread::spawn(move || {
        let stdout = std::io::stdout();
        for msg in from_server {
            if msg.write(&mut stdout.lock()).is_err() {
                break;
            }
        }
    });
    let result = run(&conn);
    drop(conn);
    let _ = writer.join();
    // the reader may still wait for input the client never sends: not joined
    drop(reader);
    result
}

/// Folders of the workspace from the `initialize` parameters: the editor's
/// stand-in for the folder the compiler was started in.
fn workspace_roots(params: &serde_json::Value) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    let uris = params["workspaceFolders"].as_array().into_iter().flatten().map(|f| &f["uri"]).chain([&params["rootUri"]]);
    for p in uris.filter_map(|u| uri_to_path(u.as_str()?)) {
        if !roots.contains(&p) {
            roots.push(p);
        }
    }
    if roots.is_empty()
        && let Some(p) = params["rootPath"].as_str().filter(|p| !p.is_empty())
    {
        roots.push(PathBuf::from(p));
    }
    roots
}

pub fn capabilities() -> lt::ServerCapabilities {
    lt::ServerCapabilities {
        text_document_sync: Some(lt::TextDocumentSyncCapability::Kind(lt::TextDocumentSyncKind::FULL)),
        document_symbol_provider: Some(lt::OneOf::Left(true)),
        definition_provider: Some(lt::OneOf::Left(true)),
        references_provider: Some(lt::OneOf::Left(true)),
        hover_provider: Some(lt::HoverProviderCapability::Simple(true)),
        completion_provider: Some(lt::CompletionOptions { trigger_characters: Some(vec![".".into()]), ..Default::default() }),
        document_formatting_provider: Some(lt::OneOf::Left(true)),
        ..Default::default()
    }
}

/// Runs the protocol on a connection (initialize handshake included).
pub fn run(conn: &Connection) -> Result<(), BoxError> {
    let caps = serde_json::to_value(capabilities())?;
    let init = serde_json::json!({
        "capabilities": caps,
        "serverInfo": { "name": "qb-lsp", "version": env!("CARGO_PKG_VERSION") },
    });
    let (id, params) = conn.initialize_start()?;
    conn.sender.send(Message::Response(Response::new_ok(id, init)))?;
    let mut server = Server::new();
    server.set_roots(workspace_roots(&params));
    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => {
                if conn.handle_shutdown(&req)? {
                    return Ok(());
                }
                let resp = server.request(req);
                conn.sender.send(Message::Response(resp))?;
            }
            Message::Notification(n) if n.method == "exit" => return Err(Box::new(ExitWithoutShutdown)),
            Message::Notification(n) => {
                for out in server.notification(n) {
                    conn.sender.send(Message::Notification(out))?;
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn params<P: serde::de::DeserializeOwned>(v: serde_json::Value) -> Result<P, String> {
    serde_json::from_value(v).map_err(|e| e.to_string())
}

/// A path starting with two slashes of either kind: UNC (`\\host\share`,
/// `//host/share`) or a device path (`\\?\...`, `\\.\...`).
fn is_unc_like(written: &str) -> bool {
    let b = written.trim_start().as_bytes();
    b.len() >= 2 && matches!(b[0], b'/' | b'\\') && matches!(b[1], b'/' | b'\\')
}

impl Server {
    pub fn new() -> Self {
        Server { docs: HashMap::new(), disk: RefCell::new(HashMap::new()), roots: Vec::new() }
    }

    /// Sets the workspace folders (searched for `$INCLUDE` files after the
    /// including file's folder and the main file's folder).
    pub fn set_roots(&mut self, roots: Vec<PathBuf>) {
        self.roots = roots;
    }

    /// Handles a request and builds its response.
    pub fn request(&mut self, req: Request) -> Response {
        let id: RequestId = req.id.clone();
        let result: Result<serde_json::Value, String> = (|| {
            match req.method.as_str() {
                "textDocument/documentSymbol" => {
                    let p: lt::DocumentSymbolParams = params(req.params)?;
                    serde_json::to_value(self.document_symbols(p.text_document.uri.as_str()))
                }
                "textDocument/definition" => {
                    let p: lt::GotoDefinitionParams = params(req.params)?;
                    let tp = p.text_document_position_params;
                    serde_json::to_value(self.definition(tp.text_document.uri.as_str(), tp.position))
                }
                "textDocument/references" => {
                    let p: lt::ReferenceParams = params(req.params)?;
                    let tp = p.text_document_position;
                    serde_json::to_value(self.references(tp.text_document.uri.as_str(), tp.position, p.context.include_declaration))
                }
                "textDocument/hover" => {
                    let p: lt::HoverParams = params(req.params)?;
                    let tp = p.text_document_position_params;
                    serde_json::to_value(self.hover(tp.text_document.uri.as_str(), tp.position))
                }
                "textDocument/completion" => {
                    let p: lt::CompletionParams = params(req.params)?;
                    let tp = p.text_document_position;
                    let dot_typed = p.context.and_then(|c| c.trigger_character).is_some_and(|c| c == ".");
                    serde_json::to_value(self.completion(tp.text_document.uri.as_str(), tp.position, dot_typed))
                }
                "textDocument/formatting" => {
                    let p: lt::DocumentFormattingParams = params(req.params)?;
                    serde_json::to_value(self.formatting(p.text_document.uri.as_str(), &p.options))
                }
                other => return Err(format!("unsupported method {other}")),
            }
            .map_err(|e| e.to_string())
        })();
        match result {
            Ok(v) => Response::new_ok(id, v),
            Err(e) if e.starts_with("unsupported method") => {
                Response::new_err(id, lsp_server::ErrorCode::MethodNotFound as i32, e)
            }
            Err(e) => Response::new_err(id, lsp_server::ErrorCode::InvalidParams as i32, e),
        }
    }

    /// Handles a notification; returns notifications to send (diagnostics).
    pub fn notification(&mut self, n: Notification) -> Vec<Notification> {
        match n.method.as_str() {
            "textDocument/didOpen" => {
                let Ok(p) = params::<lt::DidOpenTextDocumentParams>(n.params) else { return vec![] };
                let uri = p.text_document.uri.as_str().to_string();
                self.docs.insert(uri_key(&uri), Rc::new(Doc::new(uri.clone(), &p.text_document.text)));
                vec![self.publish(&uri, Some(p.text_document.version))]
            }
            "textDocument/didChange" => {
                let Ok(p) = params::<lt::DidChangeTextDocumentParams>(n.params) else { return vec![] };
                let uri = p.text_document.uri.as_str().to_string();
                if p.content_changes.is_empty() {
                    return vec![];
                }
                let key = uri_key(&uri);
                // Changes apply in order, each to the text the one before left.
                // FULL sync is advertised, but a client may still send ranges.
                let mut text = self.docs.get(&key).map(|d| d.parse.text.clone()).unwrap_or_default();
                for c in &p.content_changes {
                    apply_change(&mut text, c.range, &c.text);
                }
                self.docs.insert(key, Rc::new(Doc::new(uri.clone(), &text)));
                vec![self.publish(&uri, Some(p.text_document.version))]
            }
            "textDocument/didSave" => {
                let Ok(p) = params::<lt::DidSaveTextDocumentParams>(n.params) else { return vec![] };
                let uri = p.text_document.uri.as_str().to_string();
                if self.docs.contains_key(&uri_key(&uri)) { vec![self.publish(&uri, None)] } else { vec![] }
            }
            "textDocument/didClose" => {
                let Ok(p) = params::<lt::DidCloseTextDocumentParams>(n.params) else { return vec![] };
                let uri = p.text_document.uri;
                self.docs.remove(&uri_key(uri.as_str()));
                let clear = lt::PublishDiagnosticsParams { uri, diagnostics: vec![], version: None };
                vec![Notification::new("textDocument/publishDiagnostics".into(), clear)]
            }
            _ => vec![],
        }
    }

    /// An open document, or a file read from disk (cached by mtime + length).
    fn load(&self, uri: &str) -> Option<Rc<Doc>> {
        let key = uri_key(uri);
        if let Some(d) = self.docs.get(&key) {
            return Some(d.clone());
        }
        let path = uri_to_path(uri)?;
        let stamp: Option<Stamp> = std::fs::metadata(&path).ok().and_then(|m| Some((m.modified().ok()?, m.len())));
        let cached = self.disk.borrow().get(&key).filter(|(s, d)| stamp == Some(*s) && d.uri == uri).map(|(_, d)| d.clone());
        if cached.is_some() {
            return cached;
        }
        let bytes = std::fs::read(&path).ok()?;
        let (text, _) = qb_syntax::encoding::decode(&bytes);
        let doc = Rc::new(Doc::new(uri.to_string(), &text));
        let mut disk = self.disk.borrow_mut();
        match stamp {
            Some(stamp) => {
                disk.insert(key, (stamp, doc.clone()));
            }
            None => {
                disk.remove(&key);
            }
        }
        Some(doc)
    }

    /// Resolves an include path written in `doc` as the compiler searches:
    /// the including file's folder, then the main source file's folder
    /// (`main`: the folder of the document the file set starts from), then
    /// the workspace folders (the editor's stand-in for the folder qb was
    /// started in). An open document is used even before it is saved.
    fn resolve_include(&self, doc: &Doc, main: Option<&Path>, written: &str) -> Option<String> {
        // UNC and device paths (`\\host\share\x`, `//host/x`, `\\?\`, `\\.\`)
        // are never touched: probing one on Windows opens an SMB connection
        // (leaking the user's NTLM hash to the host named in the source).
        if is_unc_like(written) {
            return None;
        }
        if Path::new(written).is_absolute() {
            return self.find_include(Path::new(written));
        }
        let rel: PathBuf = written.split(['/', '\\']).collect();
        let dirs = doc.path.as_deref().and_then(Path::parent).into_iter().chain(main).chain(self.roots.iter().map(PathBuf::as_path));
        let mut tried: Vec<&Path> = Vec::new();
        for dir in dirs {
            if tried.contains(&dir) {
                continue;
            }
            tried.push(dir);
            if let Some(u) = self.find_include(&dir.join(&rel)) {
                return Some(u);
            }
        }
        None
    }

    /// URI of the include file at `p`: an open document or a file on disk.
    /// Where file names tell letter case apart, a name in another case is
    /// taken when the exact one is missing (as the compiler's ResolvePathCase$).
    fn find_include(&self, p: &Path) -> Option<String> {
        let uri = path_to_uri(p);
        let key = uri_key(&uri);
        if self.docs.contains_key(&key) || p.is_file() {
            return Some(uri);
        }
        if cfg!(not(windows)) {
            let lower = key.to_lowercase();
            if let Some(d) = self.docs.iter().find(|(k, _)| k.to_lowercase() == lower).map(|(_, d)| d.uri.clone()) {
                return Some(d);
            }
            if let Some(found) = resolve_case(p) {
                return Some(path_to_uri(&found));
            }
        }
        None
    }

    /// The document plus every file it includes, transitively (depth <= 100).
    fn file_set(&self, uri: &str) -> Vec<Rc<Doc>> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![(uri.to_string(), 0)];
        // the folder of the document the set starts from: the main source file's
        let mut main: Option<PathBuf> = None;
        while let Some((u, depth)) = stack.pop() {
            if depth > 100 || !seen.insert(uri_key(&u)) {
                continue;
            }
            let Some(doc) = self.load(&u) else { continue };
            if depth == 0 {
                main = doc.path.as_deref().and_then(Path::parent).map(Path::to_path_buf);
            }
            for (inc, _) in doc.analysis.includes.iter().rev() {
                if let Some(iu) = self.resolve_include(&doc, main.as_deref(), inc) {
                    stack.push((iu, depth + 1));
                }
            }
            out.push(doc);
        }
        out
    }

    fn publish(&self, uri: &str, version: Option<i32>) -> Notification {
        let mut diagnostics = Vec::new();
        if let Some(doc) = self.docs.get(&uri_key(uri)) {
            for d in &doc.parse.diagnostics {
                diagnostics.push(lt::Diagnostic {
                    range: doc.range(&d.range),
                    severity: Some(match d.severity {
                        Severity::Error => lt::DiagnosticSeverity::ERROR,
                        Severity::Warning => lt::DiagnosticSeverity::WARNING,
                    }),
                    source: Some("qb".into()),
                    message: d.message.clone(),
                    ..Default::default()
                });
            }
            // (this document is the main file of its own set)
            let main = doc.path.as_deref().and_then(Path::parent);
            for (inc, r) in &doc.analysis.includes {
                if doc.path.is_some() && self.resolve_include(doc, main, inc).is_none() {
                    diagnostics.push(lt::Diagnostic {
                        range: doc.range(r),
                        severity: Some(lt::DiagnosticSeverity::WARNING),
                        source: Some("qb".into()),
                        message: format!("include file not found: {inc}"),
                        ..Default::default()
                    });
                }
            }
        }
        let params = lt::PublishDiagnosticsParams {
            uri: uri.parse().unwrap_or_else(|_| "file:///".parse().unwrap()),
            diagnostics,
            version,
        };
        Notification::new("textDocument/publishDiagnostics".into(), params)
    }

    pub fn document_symbols(&self, uri: &str) -> Vec<lt::DocumentSymbol> {
        let Some(doc) = self.load(uri) else { return vec![] };
        let mut referenced = HashMap::new();
        for r in doc.analysis.refs.iter().filter(|r| r.is_line_number) {
            *referenced.entry(r.name.as_str()).or_insert(0) += 1;
        }
        doc.analysis
            .symbols
            .iter()
            .filter(|s| match s.kind {
                SymbolKind::Variable => false,
                // A line number counts once as its own definition.
                SymbolKind::LineNumber => referenced.get(s.name.as_str()).copied().unwrap_or(0) > 1,
                _ => true,
            })
            .map(|s| to_document_symbol(&doc, s))
            .collect()
    }

    /// Definitions of `name` seen at `at` (the file and offset of the use).
    /// A parameter or local is seen only inside its procedure, where it hides
    /// main-module names.
    fn definitions_of(&self, files: &[Rc<Doc>], name: &str, member: bool, line_number: bool, at: (&Doc, usize)) -> Vec<(Rc<Doc>, Symbol)> {
        let mut exact = Vec::new();
        let mut loose = Vec::new();
        let up = name.to_ascii_uppercase();
        let base = base_name(name);
        for d in files {
            let here = same_doc(d, at.0);
            let mut consider = |s: &Symbol| {
                if line_number != (s.kind == SymbolKind::LineNumber) {
                    return;
                }
                if s.scope.is_some() && !(here && s.visible_at(at.1)) {
                    return;
                }
                // After a dot only TYPE fields match; elsewhere fields never do.
                if member != (s.kind == SymbolKind::Field) {
                    return;
                }
                if s.name.to_ascii_uppercase() == up {
                    exact.push((d.clone(), s.clone()));
                } else if base_name(&s.name) == base && !line_number {
                    loose.push((d.clone(), s.clone()));
                }
            };
            for s in &d.analysis.symbols {
                consider(s);
                for c in &s.children {
                    consider(c);
                }
            }
        }
        let found = if exact.is_empty() { loose } else { exact };
        if found.iter().any(|(_, s)| s.scope.is_some()) {
            found.into_iter().filter(|(_, s)| s.scope.is_some()).collect()
        } else {
            found
        }
    }

    /// Declared TYPE of a variable seen at `at` (the file and offset of the
    /// use): a parameter or local of the procedure there first, else the
    /// first main-module declaration with a type.
    fn var_type(files: &[Rc<Doc>], name: &str, at: (&Doc, usize)) -> Option<String> {
        let base = base_name(name);
        let typed = |s: &&Symbol| s.kind == SymbolKind::Variable && s.type_name.is_some() && base_name(&s.name) == base;
        let (doc, off) = at;
        doc.analysis
            .symbols
            .iter()
            .filter(typed)
            .find(|s| s.scope.is_some() && s.visible_at(off))
            .or_else(|| files.iter().flat_map(|d| d.analysis.symbols.iter()).filter(typed).find(|s| s.scope.is_none()))
            .and_then(|s| s.type_name.clone())
    }

    /// Fields of a TYPE defined in the file set; `None` when it is not defined
    /// there (a standard library TYPE such as `List`).
    fn type_fields(files: &[Rc<Doc>], ty: &str) -> Option<Vec<Symbol>> {
        files
            .iter()
            .flat_map(|d| d.analysis.symbols.iter())
            .find(|s| s.kind == SymbolKind::Type && s.name.eq_ignore_ascii_case(ty))
            .map(|s| s.children.clone())
    }

    /// TYPE of a dotted receiver: `p` or `p.items` (through TYPE fields).
    fn receiver_type(files: &[Rc<Doc>], chain: &[&str], at: (&Doc, usize)) -> Option<String> {
        let (first, rest) = chain.split_first()?;
        let mut ty = Self::var_type(files, first, at)?;
        for seg in rest {
            let base = base_name(seg);
            ty = Self::type_fields(files, &ty)?.into_iter().find(|f| base_name(&f.name) == base)?.type_name?;
        }
        Some(ty)
    }

    /// What `x.Name` calls under method syntax: the procedure `T_Name` for the
    /// declared TYPE `T` of `x`, when `T` has no field `Name` (the compiler's
    /// rule). `chain` is the dotted name ending with the method.
    fn method_of(&self, files: &[Rc<Doc>], chain: &[&str], at: (&Doc, usize)) -> Option<Method> {
        let (method, recv) = chain.split_last()?;
        if recv.is_empty() || method.is_empty() {
            return None;
        }
        let ty = Self::receiver_type(files, recv, at)?;
        let base = base_name(method);
        if Self::type_fields(files, &ty).is_some_and(|f| f.iter().any(|f| base_name(&f.name) == base)) {
            return None;
        }
        let full = format!("{ty}_{method}");
        let user = self
            .definitions_of(files, &full, false, false, at)
            .into_iter()
            .find(|(_, s)| matches!(s.kind, SymbolKind::Sub | SymbolKind::Function));
        match user {
            Some((d, s)) => Some(Method::User(d, s)),
            None => qb_syntax::library::lookup(&full).map(Method::Library),
        }
    }

    /// The dotted name under the cursor, up to the segment the cursor is on,
    /// and its source text (`boxes(i).Add`).
    fn chain_at(doc: &Doc, pos: lt::Position) -> (Vec<&str>, &str) {
        let off = doc.offset(pos);
        let text = &doc.parse.text;
        match word_at(&doc.parse, off) {
            Some((TokenKind::Ident, _, range, true)) => match dotted_span(text, range.end) {
                Some((start, parts)) => (parts, &text[start..range.end]),
                None => (Vec::new(), ""),
            },
            _ => (Vec::new(), ""),
        }
    }

    /// What the cursor is on: (name, is member after a dot, is line-number reference/definition, token kind).
    fn target(doc: &Doc, pos: lt::Position) -> Option<(String, bool, bool, TokenKind)> {
        let off = doc.offset(pos);
        let (kind, word, range, member) = word_at(&doc.parse, off)?;
        let is_line = match kind {
            TokenKind::LineNumber => true,
            TokenKind::Number => doc.analysis.refs.iter().any(|r| r.is_line_number && r.range == range),
            TokenKind::Ident | TokenKind::Label | TokenKind::Keyword => false,
            _ => return None,
        };
        Some((word, member, is_line, kind))
    }

    pub fn definition(&self, uri: &str, pos: lt::Position) -> Vec<lt::Location> {
        let Some(doc) = self.load(uri) else { return vec![] };
        let Some((name, member, is_line, _)) = Self::target(&doc, pos) else { return vec![] };
        let files = self.file_set(uri);
        let at = (doc.as_ref(), doc.offset(pos));
        if member && let Some(m) = self.method_of(&files, &Self::chain_at(&doc, pos).0, at) {
            return match m {
                Method::User(d, s) => d.location(&s.name_range).into_iter().collect(),
                Method::Library(_) => vec![],
            };
        }
        self.definitions_of(&files, &name, member, is_line, at)
            .iter()
            .filter_map(|(d, s)| d.location(&s.name_range))
            .collect()
    }

    pub fn references(&self, uri: &str, pos: lt::Position, include_declaration: bool) -> Vec<lt::Location> {
        let Some(doc) = self.load(uri) else { return vec![] };
        let Some((name, _, is_line, _)) = Self::target(&doc, pos) else { return vec![] };
        let files = self.file_set(uri);
        let up = name.to_ascii_uppercase();
        let base = base_name(&name);
        let has_suffix = base != up;
        let mut out = Vec::new();
        for d in &files {
            for r in &d.analysis.refs {
                if r.is_line_number != is_line {
                    continue;
                }
                // Match each dotted segment.
                for (seg, seg_range) in segments(&r.name, r.range.start) {
                    let su = seg.to_ascii_uppercase();
                    let sb = base_name(seg);
                    let same = su == up || (sb == base && !is_line && (!has_suffix || sb == su));
                    if !same {
                        continue;
                    }
                    let is_decl = d.analysis.symbols.iter().any(|s| s.name_range == seg_range);
                    if is_decl && !include_declaration {
                        continue;
                    }
                    if let Some(l) = d.location(&seg_range) {
                        out.push(l);
                    }
                }
            }
            // Declarations whose name token is a keyword (rare) are not in refs.
        }
        out
    }

    pub fn hover(&self, uri: &str, pos: lt::Position) -> Option<lt::Hover> {
        let doc = self.load(uri)?;
        let (name, member, is_line, kind) = Self::target(&doc, pos)?;
        let files = self.file_set(uri);
        let at = (doc.as_ref(), doc.offset(pos));
        let (chain, call) = Self::chain_at(&doc, pos);
        let method = if member { self.method_of(&files, &chain, at) } else { None };
        let defs = if method.is_some() { vec![] } else { self.definitions_of(&files, &name, member, is_line, at) };
        let value = if let Some(m) = method {
            match m {
                Method::User(d, s) => {
                    let file = d.path.as_deref().and_then(|p| p.file_name()).map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                    format!("```qbasic\n{}\n```\n`{call}` calls {} `{}` in {}", s.detail, kind_name(s.kind), s.name, file)
                }
                Method::Library(p) => {
                    let mut v = format!("```qbasic\n{}\n```\n`{call}` calls `{}` from **{}** (`$IMPORT:'{}'`)", p.syntax, p.name, p.module, p.module);
                    if !p.doc.is_empty() {
                        v.push_str(&format!("\n\n{}", p.doc));
                    }
                    v
                }
            }
        } else if let Some((d, s)) = defs.first() {
            let file = d.path.as_deref().and_then(|p| p.file_name()).map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
            format!("```qbasic\n{}\n```\n{} `{}` in {}", s.detail, kind_name(s.kind), s.name, file)
        } else if kind == TokenKind::Keyword {
            let info = keywords::lookup(&name)?;
            let mut v = String::new();
            if !info.syntax.is_empty() {
                v.push_str(&format!("```qbasic\n{}\n```\n", info.syntax));
            }
            v.push_str(&format!("**{}** — {}", info.name, info.doc));
            v
        } else {
            let p = qb_syntax::library::lookup(&name)?;
            let mut v = format!("```qbasic\n{}\n```\n`{}` from **{}** (`$IMPORT:'{}'`)", p.syntax, p.name, p.module, p.module);
            if !p.doc.is_empty() {
                v.push_str(&format!("\n\n{}", p.doc));
            }
            v
        };
        Some(lt::Hover {
            contents: lt::HoverContents::Markup(lt::MarkupContent { kind: lt::MarkupKind::Markdown, value }),
            range: None,
        })
    }

    /// Members after `x.` when the TYPE of `x` is known: its fields and the
    /// `T_Name` procedures that method syntax reaches. Elsewhere every symbol,
    /// keyword and library procedure; nothing when a typed `.` has no known
    /// receiver (a dotted variable name or a number).
    pub fn completion(&self, uri: &str, pos: lt::Position, dot_typed: bool) -> lt::CompletionResponse {
        let doc = self.load(uri);
        let at = doc.as_deref().map(|d| (d, d.offset(pos)));
        if let Some(at) = at {
            let chain = dotted_before(&at.0.parse.text, at.1);
            let ty = match chain.split_last() {
                Some((_, recv)) if !recv.is_empty() => Self::receiver_type(&self.file_set(uri), recv, at),
                _ => None,
            };
            if let Some(ty) = ty {
                return lt::CompletionResponse::Array(Self::members(&self.file_set(uri), &ty));
            }
        }
        if dot_typed { lt::CompletionResponse::Array(vec![]) } else { self.all_completions(uri, at) }
    }

    fn members(files: &[Rc<Doc>], ty: &str) -> Vec<lt::CompletionItem> {
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        for f in Self::type_fields(files, ty).unwrap_or_default() {
            seen.insert(base_name(&f.name));
            items.push(lt::CompletionItem {
                label: f.name,
                kind: Some(lt::CompletionItemKind::FIELD),
                detail: Some(f.detail),
                ..Default::default()
            });
        }
        let prefix = format!("{}_", ty.to_ascii_uppercase());
        // A call needs no type suffix, and the library docs write none.
        let method_label = |name: &str| bare_name(&name[prefix.len()..]).to_string();
        let is_method = |name: &str| name.len() > prefix.len() && name.to_ascii_uppercase().starts_with(&prefix);
        for d in files {
            for s in d.analysis.symbols.iter().filter(|s| matches!(s.kind, SymbolKind::Sub | SymbolKind::Function)) {
                if is_method(&s.name) && seen.insert(base_name(&method_label(&s.name))) {
                    items.push(lt::CompletionItem {
                        label: method_label(&s.name),
                        kind: Some(lt::CompletionItemKind::METHOD),
                        detail: Some(s.detail.clone()),
                        ..Default::default()
                    });
                }
            }
        }
        let mut libs: Vec<_> = qb_syntax::library::all().filter(|p| is_method(p.name)).collect();
        libs.sort_by(|a, b| a.name.cmp(b.name));
        for p in libs {
            if seen.insert(base_name(&method_label(p.name))) {
                items.push(lt::CompletionItem {
                    label: method_label(p.name),
                    kind: Some(lt::CompletionItemKind::METHOD),
                    detail: Some(format!("{} — {}", p.module, p.syntax)),
                    documentation: (!p.doc.is_empty()).then(|| lt::Documentation::String(p.doc.to_string())),
                    ..Default::default()
                });
            }
        }
        items
    }

    /// Every name seen at `at`: parameters and locals only inside their procedure.
    fn all_completions(&self, uri: &str, at: Option<(&Doc, usize)>) -> lt::CompletionResponse {
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        for d in self.file_set(uri) {
            let here = at.filter(|(ad, _)| same_doc(&d, ad)).map(|(_, off)| off);
            for s in &d.analysis.symbols {
                if s.kind == SymbolKind::LineNumber || (s.scope.is_some() && !here.is_some_and(|off| s.visible_at(off))) {
                    continue;
                }
                if seen.insert(s.name.to_ascii_uppercase()) {
                    items.push(lt::CompletionItem {
                        label: s.name.clone(),
                        kind: Some(completion_kind(s.kind)),
                        detail: Some(s.detail.clone()),
                        ..Default::default()
                    });
                }
            }
        }
        let mut kws: Vec<_> = keywords::all().collect();
        kws.sort_by(|a, b| a.name.cmp(&b.name));
        for k in kws {
            if !seen.insert(k.name.clone()) {
                continue;
            }
            items.push(lt::CompletionItem {
                label: k.name.clone(),
                kind: Some(match k.kind {
                    KeywordKind::BuiltinFunction => lt::CompletionItemKind::FUNCTION,
                    KeywordKind::BuiltinSub => lt::CompletionItemKind::METHOD,
                    KeywordKind::Type => lt::CompletionItemKind::TYPE_PARAMETER,
                    KeywordKind::Operator => lt::CompletionItemKind::OPERATOR,
                    KeywordKind::Statement => lt::CompletionItemKind::KEYWORD,
                }),
                detail: (!k.syntax.is_empty()).then(|| k.syntax.clone()),
                ..Default::default()
            });
        }
        let mut libs: Vec<_> = qb_syntax::library::all().collect();
        libs.sort_by(|a, b| a.name.cmp(b.name));
        for p in libs {
            let label = bare_name(p.name);
            if !seen.insert(label.to_ascii_uppercase()) {
                continue;
            }
            items.push(lt::CompletionItem {
                label: label.to_string(),
                kind: Some(if p.is_function { lt::CompletionItemKind::FUNCTION } else { lt::CompletionItemKind::METHOD }),
                detail: Some(format!("{} — {}", p.module, p.syntax)),
                documentation: (!p.doc.is_empty()).then(|| lt::Documentation::String(p.doc.to_string())),
                ..Default::default()
            });
        }
        lt::CompletionResponse::Array(items)
    }

    pub fn formatting(&self, uri: &str, opts: &lt::FormattingOptions) -> Option<Vec<lt::TextEdit>> {
        let doc = self.load(uri)?;
        let indent = if opts.insert_spaces { " ".repeat(opts.tab_size.clamp(1, 16) as usize) } else { "\t".into() };
        let fo = qb_fmt::Options { indent, ..qb_fmt::Options::default() };
        let out = qb_fmt::format(&doc.parse.text, &fo);
        if out == doc.parse.text {
            return Some(vec![]);
        }
        let end = doc.position(doc.parse.text.len());
        Some(vec![lt::TextEdit { range: lt::Range { start: lt::Position { line: 0, character: 0 }, end }, new_text: out }])
    }
}

/// Applies one change of a `didChange`: the text between two positions
/// (lines and UTF-16 columns, clamped to the text) replaced, or the whole
/// text when the change has no range.
pub fn apply_change(text: &mut String, range: Option<lt::Range>, new: &str) {
    let Some(r) = range else {
        *text = new.to_string();
        return;
    };
    let index = LineIndex::new(text);
    let start = index.offset(text, r.start.line, r.start.character);
    let end = index.offset(text, r.end.line, r.end.character).max(start);
    text.replace_range(start..end, new);
}

/// `p` with each part that does not exist matched in another letter case
/// (the compiler's ResolvePathCase$, for file systems that tell case apart);
/// `None` when a part has no match.
fn resolve_case(p: &Path) -> Option<PathBuf> {
    use std::path::Component;
    if p.is_file() {
        return Some(p.to_path_buf());
    }
    let parts: Vec<Component> = p.components().collect();
    let mut done = PathBuf::new();
    for (i, part) in parts.iter().enumerate() {
        let Component::Normal(name) = part else {
            done.push(part.as_os_str());
            continue;
        };
        let last = i + 1 == parts.len();
        let fits = |q: &Path| if last { q.is_file() } else { q.is_dir() };
        let exact = done.join(name);
        if fits(exact.as_path()) {
            done = exact;
            continue;
        }
        let want = name.to_str()?.to_lowercase();
        let dir = if done.as_os_str().is_empty() { Path::new(".") } else { done.as_path() };
        let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
            .ok()?
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_str().is_some_and(|n| n.to_lowercase() == want))
            .map(|e| done.join(e.file_name()))
            .filter(|q| fits(q.as_path()))
            .collect();
        // (several names can differ only in case: the first in order, every time)
        found.sort();
        done = found.into_iter().next()?;
    }
    Some(done)
}

/// Splits a dotted name into segments with their byte ranges.
fn segments(name: &str, start: usize) -> Vec<(&str, Range<usize>)> {
    let mut v = Vec::new();
    let mut pos = 0;
    for seg in name.split('.') {
        v.push((seg, start + pos..start + pos + seg.len()));
        pos += seg.len() + 1;
    }
    v
}

/// A procedure name without its type suffix (`Http_Get$` -> `Http_Get`), as a call may write it.
fn bare_name(name: &str) -> &str {
    name.trim_end_matches(['%', '&', '!', '#', '$', '~', '`'])
}

/// Whether two loaded documents are the same file.
fn same_doc(a: &Doc, b: &Doc) -> bool {
    std::ptr::eq(a, b) || uri_key(&a.uri) == uri_key(&b.uri)
}

fn kind_name(k: SymbolKind) -> &'static str {
    match k {
        SymbolKind::Sub => "SUB",
        SymbolKind::Function => "FUNCTION",
        SymbolKind::Type => "TYPE",
        SymbolKind::Field => "field",
        SymbolKind::Const => "CONST",
        SymbolKind::Label => "label",
        SymbolKind::LineNumber => "line number",
        SymbolKind::Variable => "variable",
    }
}

fn completion_kind(k: SymbolKind) -> lt::CompletionItemKind {
    match k {
        SymbolKind::Sub | SymbolKind::Function => lt::CompletionItemKind::FUNCTION,
        SymbolKind::Type => lt::CompletionItemKind::STRUCT,
        SymbolKind::Field => lt::CompletionItemKind::FIELD,
        SymbolKind::Const => lt::CompletionItemKind::CONSTANT,
        SymbolKind::Label | SymbolKind::LineNumber => lt::CompletionItemKind::REFERENCE,
        SymbolKind::Variable => lt::CompletionItemKind::VARIABLE,
    }
}

#[allow(deprecated)]
fn to_document_symbol(doc: &Doc, s: &Symbol) -> lt::DocumentSymbol {
    let kind = match s.kind {
        SymbolKind::Sub => lt::SymbolKind::METHOD,
        SymbolKind::Function => lt::SymbolKind::FUNCTION,
        SymbolKind::Type => lt::SymbolKind::STRUCT,
        SymbolKind::Field => lt::SymbolKind::FIELD,
        SymbolKind::Const => lt::SymbolKind::CONSTANT,
        SymbolKind::Label | SymbolKind::LineNumber => lt::SymbolKind::KEY,
        SymbolKind::Variable => lt::SymbolKind::VARIABLE,
    };
    lt::DocumentSymbol {
        name: s.name.clone(),
        detail: Some(s.detail.clone()),
        kind,
        tags: None,
        deprecated: None,
        range: doc.range(&s.range),
        selection_range: doc.range(&s.name_range),
        children: (!s.children.is_empty()).then(|| s.children.iter().map(|c| to_document_symbol(doc, c)).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qb-lsp-unit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn unc_includes_are_not_probed() {
        assert!(is_unc_like(r"\\host\share\x.bi"));
        assert!(is_unc_like("//host/share/x.bi"));
        assert!(is_unc_like(r"\\?\C:\x.bi"));
        assert!(is_unc_like(r"/\host\x.bi"));
        assert!(!is_unc_like("lib/x.bi"));
        assert!(!is_unc_like(r"C:\lib\x.bi"));
        assert!(!is_unc_like("/usr/lib/x.bi"));

        let dir = temp_dir("unc");
        let inc = dir.join("a.bi");
        std::fs::write(&inc, "CONST A = 1\n").unwrap();
        let main = Doc::new(path_to_uri(&dir.join("main.bas")), "");
        let server = Server::new();
        assert_eq!(server.resolve_include(&main, None, r"\\127.0.0.1\share\a.bi"), None);
        assert_eq!(server.resolve_include(&main, None, "//127.0.0.1/share/a.bi"), None);
        // Relative and plain absolute paths still resolve.
        assert!(server.resolve_include(&main, None, "a.bi").is_some());
        assert!(server.resolve_include(&main, None, inc.to_str().unwrap()).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The compiler's order: the including file's folder, the main file's
    /// folder, then the workspace folder.
    #[test]
    fn include_search_order() {
        let dir = temp_dir("order");
        let (lib, root) = (dir.join("lib"), dir.join("root"));
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(dir.join("both.bi"), "").unwrap();
        std::fs::write(lib.join("both.bi"), "").unwrap();
        std::fs::write(dir.join("main_only.bi"), "").unwrap();
        std::fs::write(root.join("root_only.bi"), "").unwrap();
        let inc = Doc::new(path_to_uri(&lib.join("inc.bi")), "");
        let mut server = Server::new();
        let key = |u: Option<String>| u.map(|u| uri_key(&u));
        // the including file's folder first
        assert_eq!(key(server.resolve_include(&inc, Some(dir.as_path()), "both.bi")), Some(uri_key(&path_to_uri(&lib.join("both.bi")))));
        // then the main file's folder
        assert_eq!(key(server.resolve_include(&inc, Some(dir.as_path()), "main_only.bi")), Some(uri_key(&path_to_uri(&dir.join("main_only.bi")))));
        // the workspace folder last, and only when the server knows it
        assert_eq!(server.resolve_include(&inc, Some(dir.as_path()), "root_only.bi"), None);
        server.set_roots(vec![root.clone()]);
        assert_eq!(key(server.resolve_include(&inc, Some(dir.as_path()), "root_only.bi")), Some(uri_key(&path_to_uri(&root.join("root_only.bi")))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An include that is open and not saved yet is used, also when the
    /// source names it in another letter case.
    #[test]
    fn open_unsaved_include_is_used() {
        let dir = temp_dir("open");
        let main_uri = path_to_uri(&dir.join("main.bas"));
        let inc_uri = path_to_uri(&dir.join("new.bi"));
        let mut server = Server::new();
        let open = |server: &mut Server, uri: &str, text: &str| {
            let n = Notification::new("textDocument/didOpen".into(), serde_json::json!({"textDocument": {"uri": uri, "languageId": "qbasic", "version": 1, "text": text}}));
            server.notification(n)
        };
        open(&mut server, &inc_uri, "SUB Fresh\nEND SUB\n");
        let out = open(&mut server, &main_uri, "'$INCLUDE: 'NEW.BI'\nFresh\n");
        let diags = &out[0].params["diagnostics"];
        assert!(diags.as_array().unwrap().is_empty(), "{diags}");
        let def = server.definition(&main_uri, lt::Position { line: 1, character: 1 });
        assert_eq!(def.len(), 1, "{def:?}");
        assert_eq!(uri_key(def[0].uri.as_str()), uri_key(&inc_uri));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Off Windows a name in another letter case is found on disk.
    #[test]
    fn include_in_another_letter_case() {
        let dir = temp_dir("case");
        std::fs::create_dir_all(dir.join("Lib")).unwrap();
        std::fs::write(dir.join("Lib").join("Inc.BI"), "").unwrap();
        let found = resolve_case(&dir.join("lib").join("inc.bi")).expect("found in another case");
        assert!(found.is_file());
        if cfg!(target_os = "linux") {
            assert!(found.ends_with(Path::new("Lib").join("Inc.BI")), "{}", found.display());
        }
        assert_eq!(resolve_case(&dir.join("lib").join("missing.bi")), None);
        let main = Doc::new(path_to_uri(&dir.join("main.bas")), "");
        assert!(Server::new().resolve_include(&main, None, "LIB/inc.bi").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ranged_changes_use_utf16_positions() {
        let pos = |line, character| lt::Position { line, character };
        let range = |a, b| Some(lt::Range { start: a, end: b });
        // "😀" is two UTF-16 units, "ก" one (three bytes in UTF-8)
        let mut t = "a😀b\r\nกข\nend".to_string();
        apply_change(&mut t, range(pos(0, 3), pos(0, 4)), "B");
        assert_eq!(t, "a😀B\r\nกข\nend");
        apply_change(&mut t, range(pos(1, 1), pos(1, 2)), "ค");
        assert_eq!(t, "a😀B\r\nกค\nend");
        // insertion, a range over a line break, and a column past the line end
        apply_change(&mut t, range(pos(2, 0), pos(2, 0)), ">");
        assert_eq!(t, "a😀B\r\nกค\n>end");
        apply_change(&mut t, range(pos(0, 99), pos(1, 0)), " ");
        assert_eq!(t, "a😀B กค\n>end");
        // past the last line: the end of the text
        apply_change(&mut t, range(pos(9, 0), pos(9, 0)), "!");
        assert_eq!(t, "a😀B กค\n>end!");
        apply_change(&mut t, None, "new");
        assert_eq!(t, "new");
    }

    #[test]
    fn incremental_did_change() {
        let mut server = Server::new();
        let uri = "untitled:inc";
        server.notification(Notification::new("textDocument/didOpen".into(), serde_json::json!({"textDocument": {"uri": uri, "languageId": "qbasic", "version": 1, "text": "PRINT 1\nEND IF\n"}})));
        let change = serde_json::json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [
            {"range": {"start": {"line": 1, "character": 0}, "end": {"line": 2, "character": 0}}, "text": ""},
            {"range": {"start": {"line": 0, "character": 6}, "end": {"line": 0, "character": 7}}, "text": "42"},
        ]});
        let out = server.notification(Notification::new("textDocument/didChange".into(), change));
        assert_eq!(server.docs[&uri_key(uri)].parse.text, "PRINT 42\n");
        assert!(out[0].params["diagnostics"].as_array().unwrap().is_empty(), "{:?}", out[0].params);
    }

    #[test]
    fn bodies_that_are_not_messages_get_an_error_reply() {
        let e = parse_body(b"{nope").unwrap_err();
        assert_eq!((e["id"].clone(), e["error"]["code"].clone()), (serde_json::Value::Null, serde_json::json!(-32700)));
        let e = parse_body(b"[1, 2]").unwrap_err();
        assert_eq!((e["id"].clone(), e["error"]["code"].clone()), (serde_json::Value::Null, serde_json::json!(-32600)));
        // a request with a usable id but a bad shape is answered under its id
        let e = parse_body(br#"{"jsonrpc": "2.0", "id": 7, "method": 5}"#).unwrap_err();
        assert_eq!((e["id"].clone(), e["error"]["code"].clone()), (serde_json::json!(7), serde_json::json!(-32600)));
        assert!(parse_body(br#"{"jsonrpc": "2.0", "id": 1, "method": "shutdown"}"#).is_ok());
        assert!(parse_body(br#"{"jsonrpc": "2.0", "method": "initialized", "params": {}}"#).is_ok());
    }

    #[test]
    fn workspace_folders_from_initialize() {
        let (a, b) = (path_to_uri(&std::env::temp_dir().join("ws-a")), path_to_uri(&std::env::temp_dir().join("ws-b")));
        let roots = workspace_roots(&serde_json::json!({"rootUri": a, "workspaceFolders": [{"uri": b, "name": "b"}, {"uri": a, "name": "a"}]}));
        assert_eq!(roots, vec![uri_to_path(&b).unwrap(), uri_to_path(&a).unwrap()]);
        assert!(workspace_roots(&serde_json::json!({"rootUri": null})).is_empty());
        assert_eq!(workspace_roots(&serde_json::json!({"rootPath": "/x"})), vec![PathBuf::from("/x")]);
    }

    #[test]
    fn disk_files_cached_until_changed() {
        let dir = temp_dir("cache");
        let inc = dir.join("a.bi");
        std::fs::write(&inc, "CONST A = 1\n").unwrap();
        let uri = path_to_uri(&inc);
        let server = Server::new();
        let first = server.load(&uri).unwrap();
        let again = server.load(&uri).unwrap();
        assert!(Rc::ptr_eq(&first, &again), "unchanged file is re-parsed");
        std::fs::write(&inc, "CONST A = 1\nCONST BB = 2\n").unwrap();
        let changed = server.load(&uri).unwrap();
        assert!(!Rc::ptr_eq(&first, &changed), "changed file served from cache");
        assert!(changed.parse.text.contains("BB"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

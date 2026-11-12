//! Language server for QBasic / QB64 / QBNex over stdio.
//!
//! Features: diagnostics (syntax errors, block mismatches, missing include
//! files), document symbols (SUB/FUNCTION/TYPE with fields/CONST/labels/
//! referenced line numbers), go-to-definition and references across a file
//! and its `$INCLUDE`s, hover for built-in keywords and user symbols,
//! completion, and whole-document formatting through `qb-fmt`.

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

use crate::analysis::{Analysis, Symbol, SymbolKind, analyze, base_name, word_at};
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

/// Modification time and length of a file on disk.
type Stamp = (SystemTime, u64);

/// Server state.
pub struct Server {
    docs: HashMap<String, Rc<Doc>>,
    /// Files read from disk (includes that are not open), reused while their
    /// modification time and length are unchanged.
    disk: RefCell<HashMap<String, (Stamp, Rc<Doc>)>>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the server on stdio until the client exits.
pub fn run_stdio() -> Result<(), BoxError> {
    let (conn, io) = Connection::stdio();
    run(&conn)?;
    drop(conn);
    io.join()?;
    Ok(())
}

pub fn capabilities() -> lt::ServerCapabilities {
    lt::ServerCapabilities {
        text_document_sync: Some(lt::TextDocumentSyncCapability::Kind(lt::TextDocumentSyncKind::FULL)),
        document_symbol_provider: Some(lt::OneOf::Left(true)),
        definition_provider: Some(lt::OneOf::Left(true)),
        references_provider: Some(lt::OneOf::Left(true)),
        hover_provider: Some(lt::HoverProviderCapability::Simple(true)),
        completion_provider: Some(lt::CompletionOptions::default()),
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
    let (id, _params) = conn.initialize_start()?;
    conn.sender.send(Message::Response(Response::new_ok(id, init)))?;
    let mut server = Server::new();
    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => {
                if conn.handle_shutdown(&req)? {
                    return Ok(());
                }
                let resp = server.request(req);
                conn.sender.send(Message::Response(resp))?;
            }
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
        Server { docs: HashMap::new(), disk: RefCell::new(HashMap::new()) }
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
                    serde_json::to_value(self.completion(p.text_document_position.text_document.uri.as_str()))
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
                let Some(change) = p.content_changes.into_iter().last() else { return vec![] };
                self.docs.insert(uri_key(&uri), Rc::new(Doc::new(uri.clone(), &change.text)));
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

    /// Resolves an include path written in `doc` (relative to its folder).
    fn resolve_include(doc: &Doc, written: &str) -> Option<String> {
        // UNC and device paths (`\\host\share\x`, `//host/x`, `\\?\`, `\\.\`)
        // are never touched: probing one on Windows opens an SMB connection
        // (leaking the user's NTLM hash to the host named in the source).
        if is_unc_like(written) {
            return None;
        }
        let dir = doc.path.as_deref()?.parent()?;
        let rel: PathBuf = written.split(['/', '\\']).collect();
        let p = if Path::new(written).is_absolute() { PathBuf::from(written) } else { dir.join(rel) };
        p.is_file().then(|| path_to_uri(&p))
    }

    /// The document plus every file it includes, transitively (depth <= 100).
    fn file_set(&self, uri: &str) -> Vec<Rc<Doc>> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![(uri.to_string(), 0)];
        while let Some((u, depth)) = stack.pop() {
            if depth > 100 || !seen.insert(uri_key(&u)) {
                continue;
            }
            let Some(doc) = self.load(&u) else { continue };
            for (inc, _) in doc.analysis.includes.iter().rev() {
                if let Some(iu) = Self::resolve_include(&doc, inc) {
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
            for (inc, r) in &doc.analysis.includes {
                if doc.path.is_some() && Self::resolve_include(doc, inc).is_none() {
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

    fn definitions_of(&self, files: &[Rc<Doc>], name: &str, member: bool, line_number: bool) -> Vec<(Rc<Doc>, Symbol)> {
        let mut exact = Vec::new();
        let mut loose = Vec::new();
        let up = name.to_ascii_uppercase();
        let base = base_name(name);
        for d in files {
            let mut consider = |s: &Symbol| {
                if line_number != (s.kind == SymbolKind::LineNumber) {
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
        if exact.is_empty() { loose } else { exact }
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
        self.definitions_of(&files, &name, member, is_line)
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
        let defs = self.definitions_of(&files, &name, member, is_line);
        let value = if let Some((d, s)) = defs.first() {
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
        } else if let Some(p) = qb_syntax::library::lookup(&name) {
            let mut v = format!("```qbasic\n{}\n```\n`{}` from **{}** (`$IMPORT:'{}'`)", p.syntax, p.name, p.module, p.module);
            if !p.doc.is_empty() {
                v.push_str(&format!("\n\n{}", p.doc));
            }
            v
        } else {
            return None;
        };
        Some(lt::Hover {
            contents: lt::HoverContents::Markup(lt::MarkupContent { kind: lt::MarkupKind::Markdown, value }),
            range: None,
        })
    }

    pub fn completion(&self, uri: &str) -> lt::CompletionResponse {
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        for d in self.file_set(uri) {
            for s in &d.analysis.symbols {
                if s.kind == SymbolKind::LineNumber {
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
            if !seen.insert(p.name.to_ascii_uppercase()) {
                continue;
            }
            items.push(lt::CompletionItem {
                label: p.name.to_string(),
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
        assert_eq!(Server::resolve_include(&main, r"\\127.0.0.1\share\a.bi"), None);
        assert_eq!(Server::resolve_include(&main, "//127.0.0.1/share/a.bi"), None);
        // Relative and plain absolute paths still resolve.
        assert!(Server::resolve_include(&main, "a.bi").is_some());
        assert!(Server::resolve_include(&main, inc.to_str().unwrap()).is_some());
        let _ = std::fs::remove_dir_all(&dir);
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

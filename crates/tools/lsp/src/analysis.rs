//! Per-file facts the server needs: symbols, include directives, and the
//! tokens that may refer to symbols.

use std::ops::Range;

use qb_syntax::{BlockKind, Node, NodeKind, Parse, Role, TokenKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Sub,
    Function,
    Type,
    Field,
    Const,
    Label,
    LineNumber,
    Variable,
}

/// A definition found in a file.
#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    /// Whole construct (block or statement).
    pub range: Range<usize>,
    /// The name token.
    pub name_range: Range<usize>,
    /// First line of the construct, for hovers.
    pub detail: String,
    /// TYPE fields.
    pub children: Vec<Symbol>,
    /// Declared type of a variable or field (`List` in `DIM names AS List`).
    pub type_name: Option<String>,
    /// The SUB/FUNCTION a parameter or local variable belongs to (its whole
    /// block); `None` for main-module names.
    pub scope: Option<Range<usize>>,
}

impl Symbol {
    /// Whether a variable is visible at `offset` of its file: main-module
    /// names everywhere, parameters and locals inside their procedure.
    pub fn visible_at(&self, offset: usize) -> bool {
        self.scope.as_ref().is_none_or(|r| r.contains(&offset))
    }
}

/// A token that may refer to a symbol.
#[derive(Clone, Debug)]
pub struct NameRef {
    pub name: String,
    pub range: Range<usize>,
    /// Line-number reference (`GOTO 100`) or line-number definition.
    pub is_line_number: bool,
}

#[derive(Clone, Debug)]
pub struct Analysis {
    pub symbols: Vec<Symbol>,
    /// (path as written, range of the directive)
    pub includes: Vec<(String, Range<usize>)>,
    pub refs: Vec<NameRef>,
}

/// Upper-case name with any type suffix removed.
pub fn base_name(name: &str) -> String {
    let n = if name.contains('`') { name.trim_end_matches(|c: char| c.is_ascii_digit()) } else { name };
    n.trim_end_matches(['%', '&', '!', '#', '$', '~', '`']).to_ascii_uppercase()
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or("").trim().to_string()
}

const JUMP_WORDS: &[&str] = &["GOTO", "GOSUB", "THEN", "ELSE", "RESTORE", "RESUME", "RETURN", "RUN"];

pub fn analyze(p: &Parse) -> Analysis {
    let text = &p.text;
    let mut symbols = Vec::new();
    let mut includes = Vec::new();
    let mut refs = Vec::new();

    // Include directives, labels, line numbers and references come from the token stream.
    let mut prev_sig: Option<(TokenKind, String)> = None;
    let mut in_jump_list = false;
    for t in &p.tokens {
        let s = t.text(text);
        match t.kind {
            TokenKind::MetaComment | TokenKind::Metacommand => {
                if let Some(path) = qb_syntax::include_path(s) {
                    includes.push((path.to_string(), t.range()));
                }
            }
            TokenKind::Label => {
                symbols.push(Symbol {
                    name: s.to_string(),
                    kind: SymbolKind::Label,
                    range: t.range(),
                    name_range: t.range(),
                    detail: format!("{s}:"),
                    children: Vec::new(),
                    type_name: None,
                    scope: None,
                });
                refs.push(NameRef { name: s.to_string(), range: t.range(), is_line_number: false });
            }
            TokenKind::LineNumber => {
                symbols.push(Symbol {
                    name: s.to_string(),
                    kind: SymbolKind::LineNumber,
                    range: t.range(),
                    name_range: t.range(),
                    detail: format!("line {s}"),
                    children: Vec::new(),
                    type_name: None,
                    scope: None,
                });
                refs.push(NameRef { name: s.to_string(), range: t.range(), is_line_number: true });
            }
            TokenKind::Ident => refs.push(NameRef { name: s.to_string(), range: t.range(), is_line_number: false }),
            TokenKind::Number => {
                let after_jump = prev_sig.as_ref().is_some_and(|(k, w)| *k == TokenKind::Keyword && JUMP_WORDS.contains(&w.as_str()));
                if (after_jump || in_jump_list) && s.bytes().all(|b| b.is_ascii_digit()) {
                    refs.push(NameRef { name: s.to_string(), range: t.range(), is_line_number: true });
                }
            }
            _ => {}
        }
        if !t.kind.is_trivia() {
            let up = s.to_ascii_uppercase();
            // `ON x GOTO 10, 20, 30`
            if t.kind == TokenKind::Keyword && (up == "GOTO" || up == "GOSUB") {
                in_jump_list = true;
            } else if !matches!(t.kind, TokenKind::Comma | TokenKind::Number | TokenKind::Ident) {
                in_jump_list = false;
            }
            prev_sig = Some((t.kind, up));
        }
        if t.kind == TokenKind::Newline || t.kind == TokenKind::Colon {
            in_jump_list = false;
        }
    }

    collect_defs(p, &p.root, &mut symbols, false, None);
    symbols.sort_by_key(|s| s.range.start);
    Analysis { symbols, includes, refs }
}

fn sig_tokens(p: &Parse, stmt: &Node) -> Vec<(TokenKind, Range<usize>, String)> {
    stmt.token_indices()
        .into_iter()
        .map(|i| p.tokens[i as usize])
        .filter(|t| !t.kind.is_trivia())
        .map(|t| (t.kind, t.range(), t.text(&p.text).to_string()))
        .collect()
}

fn is_name(k: TokenKind) -> bool {
    matches!(k, TokenKind::Ident | TokenKind::Keyword)
}

fn collect_defs(p: &Parse, node: &Node, out: &mut Vec<Symbol>, in_library: bool, scope: Option<&Range<usize>>) {
    for child in node.child_nodes() {
        match child.kind {
            NodeKind::Block(kind) => {
                let header = child.header().unwrap();
                let sig = sig_tokens(p, header);
                let block_range = child.range(&p.tokens);
                let detail = first_line(&p.text[header.range(&p.tokens)]);
                let mk = |k: SymbolKind, idx: usize| {
                    sig.get(idx).filter(|t| is_name(t.0)).map(|t| Symbol {
                        name: t.2.clone(),
                        kind: k,
                        range: block_range.clone(),
                        name_range: t.1.clone(),
                        detail: detail.clone(),
                        children: Vec::new(),
                        type_name: None,
                        scope: None,
                    })
                };
                let procedure = matches!(kind, BlockKind::Sub | BlockKind::Function | BlockKind::DefFn);
                if procedure {
                    out.extend(mk(if kind == BlockKind::Sub { SymbolKind::Sub } else { SymbolKind::Function }, 1));
                    for (i, ty) in params(&sig) {
                        let t = &sig[i];
                        out.push(Symbol {
                            name: t.2.clone(),
                            kind: SymbolKind::Variable,
                            range: t.1.clone(),
                            name_range: t.1.clone(),
                            detail: ty.as_ref().map_or_else(|| t.2.clone(), |ty| format!("{} AS {ty}", t.2)),
                            children: Vec::new(),
                            type_name: ty,
                            scope: Some(block_range.clone()),
                        });
                    }
                }
                if kind == BlockKind::Type && let Some(mut s) = mk(SymbolKind::Type, 1) {
                    for (stmt, _) in statements_of(child) {
                        let f = sig_tokens(p, stmt);
                        if f.len() >= 2 && is_name(f[0].0) && f[1].2.eq_ignore_ascii_case("AS") {
                            s.children.push(Symbol {
                                name: f[0].2.clone(),
                                kind: SymbolKind::Field,
                                range: stmt.range(&p.tokens),
                                name_range: f[0].1.clone(),
                                detail: first_line(&p.text[stmt.range(&p.tokens)]),
                                children: Vec::new(),
                                type_name: f.get(2).filter(|t| is_name(t.0)).map(|t| t.2.clone()),
                                scope: None,
                            });
                        }
                    }
                    out.push(s);
                }
                if kind != BlockKind::Type {
                    // A procedure's header holds its parameters (above); other
                    // headers can be definitions (`IF ... THEN DIM`). Bodies
                    // hold more definitions.
                    let inner = if procedure { Some(&block_range) } else { scope };
                    if !procedure {
                        statement_defs(p, header, out, in_library, scope);
                    }
                    collect_defs(p, child, out, in_library || kind == BlockKind::DeclareLibrary, inner);
                }
            }
            NodeKind::Body => collect_defs(p, child, out, in_library, scope),
            NodeKind::Statement(_) => statement_defs(p, child, out, in_library, scope),
            NodeKind::Root => collect_defs(p, child, out, in_library, scope),
        }
    }
}

/// Parameters in a SUB/FUNCTION header, `(a AS List, OPTIONAL b$ = "", c() AS Box)`:
/// the index in `sig` of each name and its declared type.
fn params(sig: &[(TokenKind, Range<usize>, String)]) -> Vec<(usize, Option<String>)> {
    let Some(open) = sig.iter().position(|t| t.0 == TokenKind::LParen) else { return Vec::new() };
    let mut out: Vec<(usize, Option<String>)> = Vec::new();
    let (mut depth, mut expect_name, mut after_as) = (0, true, false);
    for (i, t) in sig.iter().enumerate().skip(open) {
        match t.0 {
            TokenKind::LParen => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            TokenKind::RParen => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            TokenKind::Comma if depth == 1 => {
                (expect_name, after_as) = (true, false);
                continue;
            }
            _ => {}
        }
        if depth != 1 {
            continue;
        }
        let up = t.2.to_ascii_uppercase();
        if after_as {
            after_as = false;
            if let Some(last) = out.last_mut().filter(|_| is_name(t.0)) {
                last.1 = Some(t.2.clone());
            }
        } else if is_name(t.0) && up == "AS" {
            after_as = true;
        } else if expect_name && !(is_name(t.0) && matches!(up.as_str(), "BYVAL" | "BYREF" | "OPTIONAL")) && t.0 == TokenKind::Ident {
            out.push((i, None));
            expect_name = false;
        }
    }
    out
}

fn statements_of(block: &Node) -> Vec<(&Node, ())> {
    let mut v = Vec::new();
    for b in block.child_nodes().filter(|n| n.kind == NodeKind::Body) {
        for s in b.child_nodes() {
            if matches!(s.kind, NodeKind::Statement(_)) {
                v.push((s, ()));
            }
        }
    }
    v
}

fn statement_defs(p: &Parse, stmt: &Node, out: &mut Vec<Symbol>, in_library: bool, scope: Option<&Range<usize>>) {
    if !matches!(stmt.kind, NodeKind::Statement(Role::Plain | Role::Open(_))) {
        return;
    }
    let sig = sig_tokens(p, stmt);
    let Some(first) = sig.first() else { return };
    let head = first.2.to_ascii_uppercase();
    let range = stmt.range(&p.tokens);
    let detail = first_line(&p.text[range.clone()]);
    let push = |out: &mut Vec<Symbol>, kind, t: &(TokenKind, Range<usize>, String)| {
        out.push(Symbol {
            name: t.2.clone(),
            kind,
            range: range.clone(),
            name_range: t.1.clone(),
            detail: detail.clone(),
            children: Vec::new(),
            type_name: None,
            // Only variables are local; a CONST in a SUB is found program-wide as before.
            scope: if kind == SymbolKind::Variable { scope.cloned() } else { None },
        })
    };
    match head.as_str() {
        // External procedures in DECLARE LIBRARY.
        "SUB" | "FUNCTION" if in_library => {
            if let Some(t) = sig.get(1).filter(|t| is_name(t.0)) {
                push(out, if head == "SUB" { SymbolKind::Sub } else { SymbolKind::Function }, t);
            }
        }
        "CONST" => {
            // CONST a = 1, b = 2
            let mut depth = 0;
            for (i, t) in sig.iter().enumerate().skip(1) {
                match t.0 {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    _ => {}
                }
                let starts_item = i == 1 || (depth == 0 && sig[i - 1].0 == TokenKind::Comma);
                if starts_item && t.0 == TokenKind::Ident && sig.get(i + 1).is_some_and(|n| n.2 == "=") {
                    push(out, SymbolKind::Const, t);
                }
            }
        }
        "DIM" | "REDIM" | "COMMON" | "STATIC" | "SHARED" => {
            let mut depth = 0;
            let mut expect_name = true;
            let mut after_as = false;
            // QB64 `DIM AS LONG a, b` puts the type first; it then applies to every name.
            let mut lead_type: Option<String> = None;
            let mut own_var: Option<usize> = None;
            for t in sig.iter().skip(1) {
                match t.0 {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    TokenKind::Comma if depth == 0 => {
                        expect_name = true;
                        after_as = false;
                        continue;
                    }
                    _ => {}
                }
                if depth != 0 {
                    continue;
                }
                let up = t.2.to_ascii_uppercase();
                if after_as {
                    after_as = false;
                    if is_name(t.0) {
                        match own_var.filter(|_| !expect_name) {
                            Some(i) => out[i].type_name = Some(t.2.clone()),
                            None => lead_type = Some(t.2.clone()),
                        }
                    }
                    continue;
                }
                if t.0 == TokenKind::Keyword && up == "AS" {
                    after_as = true;
                    continue;
                }
                if expect_name {
                    // Skip modifiers.
                    if t.0 == TokenKind::Keyword && matches!(up.as_str(), "SHARED" | "_PRESERVE" | "PRESERVE") {
                        continue;
                    }
                    if t.0 == TokenKind::Ident {
                        push(out, SymbolKind::Variable, t);
                        let i = out.len() - 1;
                        out[i].type_name = lead_type.clone();
                        own_var = Some(i);
                        expect_name = false;
                    }
                }
            }
        }
        _ => {}
    }
}

/// The identifier segment at `offset` (dotted names are split at `.`), with
/// its byte range and whether it follows a dot (`p.x`, or `a(1).x` where the
/// dot is a token of its own after the index).
pub fn word_at(p: &Parse, offset: usize) -> Option<(TokenKind, String, Range<usize>, bool)> {
    let at = |t: &qb_syntax::Token| !t.kind.is_trivia();
    let i = p
        .tokens
        .iter()
        .position(|t| (t.start as usize) <= offset && offset < t.end as usize && at(t))
        .or_else(|| p.tokens.iter().position(|t| t.end as usize == offset && at(t)))?;
    let tok = &p.tokens[i];
    let text = tok.text(&p.text);
    let r = tok.range();
    let after_index = i >= 2 && p.tokens[i - 1].kind == TokenKind::Dot && p.tokens[i - 2].kind == TokenKind::RParen;
    if tok.kind == TokenKind::Ident && text.contains('.') {
        let rel = offset.saturating_sub(r.start).min(text.len());
        let seg_start = text[..rel].rfind('.').map_or(0, |i| i + 1);
        let seg_end = text[rel..].find('.').map_or(text.len(), |i| rel + i);
        let member = seg_start > 0 || after_index;
        return Some((tok.kind, text[seg_start..seg_end].to_string(), r.start + seg_start..r.start + seg_end, member));
    }
    Some((tok.kind, text.to_string(), r, tok.kind == TokenKind::Ident && after_index))
}

/// The dotted name before `offset` on its line, split at `.`: `names.Ad`
/// gives `["names", "Ad"]`, `p.items.` gives `["p", "items", ""]`. An array
/// index is skipped, so `boxes(i + 1).Ad` gives `["boxes", "Ad"]`. Empty when
/// the text there is not a name.
pub fn dotted_before(text: &str, offset: usize) -> Vec<&str> {
    dotted_span(text, offset).map(|(_, parts)| parts).unwrap_or_default()
}

/// As [`dotted_before`], with the byte offset where the dotted name starts.
pub fn dotted_span(text: &str, offset: usize) -> Option<(usize, Vec<&str>)> {
    let b = text.as_bytes();
    let name_byte = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'.';
    let mut end = offset.min(text.len());
    let mut pieces = Vec::new();
    let start = loop {
        let mut start = end;
        while start > 0 && name_byte(b[start - 1]) {
            start -= 1;
        }
        pieces.push(&text[start..end]);
        if !(text[start..end].starts_with('.') && start > 0 && b[start - 1] == b')') {
            break start;
        }
        // Back over the balanced index `( ... )` on this line.
        let mut depth = 0;
        let mut i = start;
        loop {
            i = i.checked_sub(1)?;
            match b[i] {
                b')' => depth += 1,
                b'(' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                b'\n' | b'\r' => return None,
                _ => {}
            }
        }
        end = i;
    };
    pieces.reverse();
    if !pieces[0].starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        return None;
    }
    let mut parts: Vec<&str> = pieces[0].split('.').collect();
    for p in &pieces[1..] {
        parts.extend(p.split('.').skip(1));
    }
    Some((start, parts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_definitions() {
        let src = "CONST A = 1, B$ = \"x\"\nTYPE pt\nx AS INTEGER\ny AS INTEGER\nEND TYPE\nDIM SHARED p AS pt, q(10)\nstart:\nSUB s (n)\nGOTO 100\nEND SUB\nFUNCTION f$ ()\nEND FUNCTION\n100 PRINT\n";
        let a = analyze(&qb_syntax::parse(src));
        let names: Vec<(&str, SymbolKind)> = a.symbols.iter().map(|s| (s.name.as_str(), s.kind)).collect();
        assert_eq!(
            names,
            vec![
                ("A", SymbolKind::Const),
                ("B$", SymbolKind::Const),
                ("pt", SymbolKind::Type),
                ("p", SymbolKind::Variable),
                ("q", SymbolKind::Variable),
                ("start", SymbolKind::Label),
                ("s", SymbolKind::Sub),
                ("n", SymbolKind::Variable),
                ("f$", SymbolKind::Function),
                ("100", SymbolKind::LineNumber),
            ]
        );
        assert_eq!(a.symbols[2].children.len(), 2);
        assert!(a.refs.iter().any(|r| r.is_line_number && r.name == "100" && r.range.start < 120));
        assert_eq!(base_name("f$"), "F");
        assert_eq!(base_name("x~&"), "X");
    }

    #[test]
    fn records_declared_types() {
        let src = "TYPE Box\nitems AS List\nEND TYPE\nDIM names AS List, n AS INTEGER, k\nDIM AS Dict a, b\nREDIM q(5) AS Box\n";
        let a = analyze(&qb_syntax::parse(src));
        let ty = |n: &str| a.symbols.iter().find(|s| s.name == n).and_then(|s| s.type_name.as_deref());
        assert_eq!(ty("names"), Some("List"));
        assert_eq!(ty("n"), Some("INTEGER"));
        assert_eq!(ty("k"), None);
        assert_eq!((ty("a"), ty("b")), (Some("Dict"), Some("Dict")));
        assert_eq!(ty("q"), Some("Box"));
        assert!(a.symbols.iter().all(|s| s.name != "Dict"));
        assert_eq!(a.symbols[0].children[0].type_name.as_deref(), Some("List"));
    }

    #[test]
    fn splits_dotted_text_before_cursor() {
        assert_eq!(dotted_before("PRINT names.Ad", 14), vec!["names", "Ad"]);
        assert_eq!(dotted_before("x = p.items.", 12), vec!["p", "items", ""]);
        assert_eq!(dotted_before("PRINT ", 6), Vec::<&str>::new());
        assert_eq!(dotted_before("x = 1.5", 7), Vec::<&str>::new());
        assert_eq!(dotted_before("boxes(i + 1).Ad", 15), vec!["boxes", "Ad"]);
        assert_eq!(dotted_before("a(f(1)).b(2).", 13), vec!["a", "b", ""]);
        assert_eq!(dotted_before("x = (1).", 8), Vec::<&str>::new());
        assert_eq!(dotted_span("PRINT b(1).items.Add", 20), Some((6, vec!["b", "items", "Add"])));
    }

    #[test]
    fn parameters_and_locals_are_scoped() {
        let src = "DIM g AS Dict\nSUB s (BYVAL n AS LONG, l AS List, OPTIONAL t$ = \"(,)\", a() AS Box)\nDIM k AS Set\nEND SUB\n";
        let a = analyze(&qb_syntax::parse(src));
        let var = |n: &str| a.symbols.iter().find(|s| s.name == n && s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var("g").scope, None);
        let sub = a.symbols.iter().find(|s| s.kind == SymbolKind::Sub).unwrap();
        for (n, ty) in [("n", Some("LONG")), ("l", Some("List")), ("t$", None), ("a", Some("Box")), ("k", Some("Set"))] {
            assert_eq!(var(n).type_name.as_deref(), ty, "{n}");
            assert_eq!(var(n).scope.as_ref(), Some(&sub.range), "{n}");
        }
        assert!(var("l").visible_at(sub.range.start + 1) && !var("l").visible_at(0));
    }}

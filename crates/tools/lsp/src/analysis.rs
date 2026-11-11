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

    collect_defs(p, &p.root, &mut symbols, false);
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

fn collect_defs(p: &Parse, node: &Node, out: &mut Vec<Symbol>, in_library: bool) {
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
                    })
                };
                match kind {
                    BlockKind::Sub => out.extend(mk(SymbolKind::Sub, 1)),
                    BlockKind::Function | BlockKind::DefFn => out.extend(mk(SymbolKind::Function, 1)),
                    BlockKind::Type => {
                        if let Some(mut s) = mk(SymbolKind::Type, 1) {
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
                                    });
                                }
                            }
                            out.push(s);
                        }
                    }
                    _ => {}
                }
                if kind != BlockKind::Type {
                    // Header statements can hold parameters; bodies hold more definitions.
                    statement_defs(p, header, out, in_library);
                    collect_defs(p, child, out, in_library || kind == BlockKind::DeclareLibrary);
                }
            }
            NodeKind::Body => collect_defs(p, child, out, in_library),
            NodeKind::Statement(_) => statement_defs(p, child, out, in_library),
            NodeKind::Root => collect_defs(p, child, out, in_library),
        }
    }
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

fn statement_defs(p: &Parse, stmt: &Node, out: &mut Vec<Symbol>, in_library: bool) {
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
            for t in sig.iter().skip(1) {
                match t.0 {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    TokenKind::Comma if depth == 0 => {
                        expect_name = true;
                        continue;
                    }
                    _ => {}
                }
                if expect_name && depth == 0 {
                    let up = t.2.to_ascii_uppercase();
                    // Skip modifiers; QB64 `DIM AS LONG a, b` puts the type first.
                    if t.0 == TokenKind::Keyword && matches!(up.as_str(), "SHARED" | "_PRESERVE" | "PRESERVE" | "AS") {
                        continue;
                    }
                    if t.0 == TokenKind::Ident {
                        push(out, SymbolKind::Variable, t);
                        expect_name = false;
                    }
                }
            }
        }
        _ => {}
    }
}

/// The identifier segment at `offset` (dotted names are split at `.`), with
/// its byte range and whether it follows a dot.
pub fn word_at(p: &Parse, offset: usize) -> Option<(TokenKind, String, Range<usize>, bool)> {
    let tok = p
        .tokens
        .iter()
        .find(|t| (t.start as usize) <= offset && offset < t.end as usize)
        .filter(|t| !t.kind.is_trivia())
        .or_else(|| p.tokens.iter().find(|t| t.end as usize == offset && !t.kind.is_trivia()))?;
    let text = tok.text(&p.text);
    let r = tok.range();
    if tok.kind == TokenKind::Ident && text.contains('.') {
        let rel = offset.saturating_sub(r.start).min(text.len());
        let seg_start = text[..rel].rfind('.').map_or(0, |i| i + 1);
        let seg_end = text[rel..].find('.').map_or(text.len(), |i| rel + i);
        return Some((tok.kind, text[seg_start..seg_end].to_string(), r.start + seg_start..r.start + seg_end, seg_start > 0));
    }
    Some((tok.kind, text.to_string(), r, false))
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
                ("f$", SymbolKind::Function),
                ("100", SymbolKind::LineNumber),
            ]
        );
        assert_eq!(a.symbols[2].children.len(), 2);
        assert!(a.refs.iter().any(|r| r.is_line_number && r.name == "100" && r.range.start < 120));
        assert_eq!(base_name("f$"), "F");
        assert_eq!(base_name("x~&"), "X");
    }
}

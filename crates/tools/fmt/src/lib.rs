//! Source formatter for QBasic / QB64 / QBNex, built on `qb-syntax`.
//!
//! What it changes:
//! * keyword case ([`KeywordCase`]);
//! * indentation of block bodies (IF/FOR/DO/WHILE/SELECT/SUB/FUNCTION/TYPE/...;
//!   CASE lines one level inside SELECT); line numbers and labels go to column 0;
//! * spacing between tokens (one space around binary operators and after
//!   commas, none inside parentheses, ...); trailing blanks; line endings.
//!
//! What it never changes: the tokens themselves (apart from keyword case),
//! comments, strings, `DATA` items and metacommands. Lines holding a lexical
//! error are only re-indented. Formatting is idempotent; [`equivalent`]
//! checks that two texts have the same tokens.

use qb_syntax::keywords::{self, KeywordKind};
use qb_syntax::{BlockKind, Element, Encoding, Node, NodeKind, Parse, Role, TokenKind, parse};

/// How to write keywords.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KeywordCase {
    #[default]
    Upper,
    Lower,
    /// Leave keywords as written.
    Preserve,
}

/// Line ending of the output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineEnding {
    /// The first line ending found in the input (`\n` when there is none).
    #[default]
    Auto,
    Lf,
    CrLf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub keyword_case: KeywordCase,
    /// One indentation level (`"    "` by default; `"\t"` for tabs).
    pub indent: String,
    pub line_ending: LineEnding,
}

impl Default for Options {
    fn default() -> Self {
        Options { keyword_case: KeywordCase::Upper, indent: "    ".into(), line_ending: LineEnding::Auto }
    }
}

/// Formats source text.
pub fn format(text: &str, opts: &Options) -> String {
    let p = parse(text);
    Formatter::new(&p, opts).run()
}

/// Formats source bytes (CP437 or UTF-8), keeping the encoding.
pub fn format_bytes(bytes: &[u8], opts: &Options) -> Vec<u8> {
    let (text, enc): (String, Encoding) = qb_syntax::encoding::decode(bytes);
    qb_syntax::encoding::encode(&format(&text, opts), enc)
}

/// Meaningful tokens of a text, normalised for comparison: whitespace and
/// line continuations dropped, newlines as `\n`, keywords upper-case.
pub fn normalized_tokens(text: &str) -> Vec<(TokenKind, String)> {
    let tokens = qb_syntax::lex(text);
    let mut out = Vec::new();
    let mut skip_newline = false;
    for t in &tokens {
        match t.kind {
            TokenKind::Whitespace => {}
            TokenKind::LineContinuation => skip_newline = true,
            TokenKind::Newline if skip_newline => skip_newline = false,
            TokenKind::Newline => out.push((t.kind, "\n".to_string())),
            TokenKind::Keyword => {
                skip_newline = false;
                out.push((t.kind, t.text(text).to_ascii_uppercase()))
            }
            _ => {
                skip_newline = false;
                out.push((t.kind, t.text(text).to_string()))
            }
        }
    }
    out
}

/// `Ok` when both texts have the same meaningful tokens; otherwise describes
/// the first difference.
pub fn equivalent(a: &str, b: &str) -> Result<(), String> {
    let ta = normalized_tokens(a);
    let tb = normalized_tokens(b);
    for (i, (x, y)) in ta.iter().zip(&tb).enumerate() {
        if x != y {
            return Err(format!("token {i} differs: {x:?} vs {y:?}"));
        }
    }
    if ta.len() != tb.len() {
        return Err(format!("token count differs: {} vs {}", ta.len(), tb.len()));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Ctx {
    depth: usize,
    in_library: bool,
}

struct Formatter<'a> {
    p: &'a Parse,
    opts: &'a Options,
    ctx: Vec<Ctx>,
}

impl<'a> Formatter<'a> {
    fn new(p: &'a Parse, opts: &'a Options) -> Self {
        let mut ctx = vec![Ctx { depth: 0, in_library: false }; p.tokens.len()];
        assign(&p.root, Ctx { depth: 0, in_library: false }, &mut ctx);
        Formatter { p, opts, ctx }
    }

    fn text(&self, i: usize) -> &'a str {
        self.p.tokens[i].text(&self.p.text)
    }

    fn newline(&self) -> &'static str {
        match self.opts.line_ending {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Auto => {
                match self.p.tokens.iter().find(|t| t.kind == TokenKind::Newline).map(|t| t.text(&self.p.text)) {
                    Some("\r\n") => "\r\n",
                    _ => "\n",
                }
            }
        }
    }

    fn run(&self) -> String {
        let nl = self.newline();
        let mut out = String::with_capacity(self.p.text.len() + 64);
        let mut line: Vec<usize> = Vec::new();
        let mut continued = false;
        let mut before: Option<usize> = None;
        for i in 0..self.p.tokens.len() {
            if self.p.tokens[i].kind == TokenKind::Newline {
                let next_continued = self.ends_with_continuation(&line);
                self.line(&line, continued, before, &mut out);
                before = line
                    .iter()
                    .rev()
                    .copied()
                    .find(|&j| !matches!(self.p.tokens[j].kind, TokenKind::Whitespace | TokenKind::LineContinuation));
                out.push_str(nl);
                line.clear();
                continued = next_continued;
            } else {
                line.push(i);
            }
        }
        self.line(&line, continued, before, &mut out);
        out
    }

    fn ends_with_continuation(&self, line: &[usize]) -> bool {
        line.iter().rev().map(|&i| self.p.tokens[i].kind).find(|k| *k != TokenKind::Whitespace)
            == Some(TokenKind::LineContinuation)
    }

    fn line(&self, line: &[usize], continued: bool, before: Option<usize>, out: &mut String) {
        let before = if continued { before } else { None };
        // Significant tokens with "had blank before".
        let mut sig: Vec<(usize, bool)> = Vec::new();
        let mut ws = false;
        for &i in line {
            if self.p.tokens[i].kind == TokenKind::Whitespace {
                ws = true;
            } else {
                sig.push((i, ws));
                ws = false;
            }
        }
        let Some(&(first, _)) = sig.first() else { return };
        let depth = self.ctx[first].depth + usize::from(continued);
        let first_kind = self.p.tokens[first].kind;
        let col0 = !continued && matches!(first_kind, TokenKind::LineNumber | TokenKind::Label);
        if !col0 {
            for _ in 0..depth {
                out.push_str(&self.opts.indent);
            }
        }
        let raw = sig.iter().any(|&(i, _)| matches!(self.p.tokens[i].kind, TokenKind::Error | TokenKind::UnterminatedString));
        if raw {
            let a = self.p.tokens[first].start as usize;
            let b = self.p.tokens[sig.last().unwrap().0].end as usize;
            out.push_str(&self.p.text[a..b]);
            return;
        }
        for (n, &(i, had_ws)) in sig.iter().enumerate() {
            if n > 0 && self.space_between(&sig, n, had_ws, before) {
                out.push(' ');
            }
            self.token(&sig, n, i, out);
        }
    }

    fn token(&self, sig: &[(usize, bool)], n: usize, i: usize, out: &mut String) {
        let t = self.p.tokens[i];
        let text = self.text(i);
        if t.kind != TokenKind::Keyword {
            out.push_str(text);
            return;
        }
        // External names in DECLARE LIBRARY keep their case (they are linked by name).
        if self.ctx[i].in_library && n > 0 {
            let prev = self.text(sig[n - 1].0);
            if prev.eq_ignore_ascii_case("SUB") || prev.eq_ignore_ascii_case("FUNCTION") || prev.eq_ignore_ascii_case("ALIAS") {
                out.push_str(text);
                return;
            }
        }
        match self.opts.keyword_case {
            KeywordCase::Upper => out.push_str(&text.to_ascii_uppercase()),
            KeywordCase::Lower => out.push_str(&text.to_ascii_lowercase()),
            KeywordCase::Preserve => out.push_str(text),
        }
    }

    /// Is the operator at `sig[n]` a unary sign?
    fn is_unary(&self, sig: &[(usize, bool)], n: usize, before: Option<usize>) -> bool {
        let text = self.text(sig[n].0);
        if text != "-" && text != "+" {
            return false;
        }
        let prev = if n == 0 { before } else { Some(sig[n - 1].0) };
        let Some(p) = prev else { return true };
        let pk = self.p.tokens[p].kind;
        match pk {
            TokenKind::Operator
            | TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::LBrace
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Colon
            | TokenKind::Hash
            | TokenKind::Question
            | TokenKind::LineNumber => true,
            TokenKind::Keyword => {
                !matches!(keywords::lookup(self.text(p)).map(|k| k.kind), Some(KeywordKind::BuiltinFunction))
            }
            _ => false,
        }
    }

    fn space_between(&self, sig: &[(usize, bool)], n: usize, had_ws: bool, before: Option<usize>) -> bool {
        use TokenKind as K;
        let (p, _) = sig[n - 1];
        let (q, _) = sig[n];
        let pk = self.p.tokens[p].kind;
        let nk = self.p.tokens[q].kind;
        // A blank after a line-leading `#` would turn the line into a `#` comment.
        if pk == K::Hash && (nk.is_comment() || nk == K::LineContinuation) {
            return had_ws;
        }
        if nk.is_comment() || nk == K::LineContinuation {
            return true;
        }
        if pk == K::Label && nk == K::Colon {
            return false;
        }
        // `name :` at the start of a line must not become the label `name:`.
        if pk == K::Ident && nk == K::Colon && n == 1 && before.is_none() {
            return had_ws;
        }
        if matches!(nk, K::Comma) && pk == K::Keyword {
            return had_ws;
        }
        if matches!(nk, K::Comma | K::Semicolon | K::RParen | K::RBracket | K::RBrace | K::Colon) {
            return false;
        }
        if matches!(pk, K::LParen | K::LBracket | K::LBrace | K::Hash | K::At) {
            return false;
        }
        // `$"..."` interpolation; anything else after a lone `$` keeps its spacing
        // (`$ IF` must not become the metacommand `$IF`).
        if pk == K::Dollar {
            return had_ws && nk != K::String;
        }
        if pk == K::Dot || nk == K::Dot || pk == K::Ampersand || nk == K::Ampersand {
            return had_ws;
        }
        if matches!(pk, K::Comma | K::Semicolon | K::Colon) {
            return true;
        }
        // Letter ranges: `DEFINT A-Z`, `_DEFINE A-Z AS LONG`.
        let first = self.text(sig[0].0).to_ascii_uppercase();
        if matches!(first.as_str(), "DEFINT" | "DEFLNG" | "DEFSNG" | "DEFDBL" | "DEFSTR" | "_DEFINE")
            && (self.text(p) == "-" || self.text(q) == "-")
            && !(pk == K::Operator && nk == K::Operator)
        {
            return false;
        }
        if pk == K::Operator {
            // Two operators that had a blank between them keep it (`- =` is not `-=`).
            return !self.is_unary(sig, n - 1, before) || (nk == K::Operator && had_ws);
        }
        if nk == K::Operator {
            return true;
        }
        if nk == K::LParen || nk == K::LBracket || nk == K::LBrace {
            return match pk {
                K::Ident | K::Keyword | K::RParen | K::RBracket => had_ws,
                _ => true,
            };
        }
        if pk.is_wordlike() && nk.is_wordlike() {
            return true;
        }
        if matches!(pk, K::RParen | K::RBracket | K::RBrace) && nk.is_wordlike() {
            return true;
        }
        had_ws
    }
}

/// Records the depth (and library context) of every token.
fn assign(node: &Node, ctx: Ctx, out: &mut [Ctx]) {
    match node.kind {
        NodeKind::Block(kind) => {
            let lib = ctx.in_library || kind == BlockKind::DeclareLibrary;
            let mut seen_case = false;
            for c in &node.children {
                match c {
                    Element::Token(t) => out[*t as usize] = ctx,
                    Element::Node(n) => match n.kind {
                        NodeKind::Body => {
                            let extra = if kind == BlockKind::Select && seen_case { 2 } else { 1 };
                            assign(n, Ctx { depth: ctx.depth + extra, in_library: lib }, out);
                        }
                        NodeKind::Statement(Role::Middle(BlockKind::Select)) => {
                            seen_case = true;
                            assign(n, Ctx { depth: ctx.depth + 1, in_library: lib }, out);
                        }
                        _ => assign(n, Ctx { depth: ctx.depth, in_library: lib }, out),
                    },
                }
            }
        }
        _ => {
            for c in &node.children {
                match c {
                    Element::Token(t) => out[*t as usize] = ctx,
                    Element::Node(n) => assign(n, ctx, out),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(s: &str) -> String {
        format(s, &Options::default())
    }

    #[test]
    fn indents_blocks() {
        let src = "sub s\nfor i=1 to 3\nif i>1 then\nprint i;\"x\",-i\nelse\nprint  -1\nend if\nnext\nend sub\n";
        let want = "SUB s\n    FOR i = 1 TO 3\n        IF i > 1 THEN\n            PRINT i; \"x\", -i\n        ELSE\n            PRINT -1\n        END IF\n    NEXT\nEND SUB\n";
        assert_eq!(fmt(src), want);
        assert_eq!(fmt(want), want);
    }

    #[test]
    fn select_labels_numbers() {
        let src = "SELECT CASE x\n' c\nCASE 1\ny=2\nCASE ELSE\nEND SELECT\n  start:\n   10 PRINT a ( 1 ) , b(2)\n";
        let want = "SELECT CASE x\n    ' c\n    CASE 1\n        y = 2\n    CASE ELSE\nEND SELECT\nstart:\n10 PRINT a (1), b(2)\n";
        assert_eq!(fmt(src), want);
    }

    #[test]
    fn keeps_data_strings_comments() {
        let src = "data  a  b ,\"x  y\" : print \"a  b\" 'x   y\r\nx = 1 _\r\n+2\r\n";
        let want = "DATA a  b ,\"x  y\": PRINT \"a  b\" 'x   y\r\nx = 1 _\r\n    + 2\r\n";
        assert_eq!(fmt(src), want);
    }

    #[test]
    fn library_names_keep_case() {
        let src = "declare library\nsub sleep (byval ms&)\nend declare\n";
        let opts = Options { keyword_case: KeywordCase::Upper, ..Options::default() };
        assert_eq!(format(src, &opts), "DECLARE LIBRARY\n    SUB sleep (BYVAL ms&)\nEND DECLARE\n");
    }

    #[test]
    fn lower_case_and_tabs() {
        let opts = Options { keyword_case: KeywordCase::Lower, indent: "\t".into(), ..Options::default() };
        assert_eq!(format("DO\nLOOP UNTIL INKEY$<>\"\"", &opts), "do\nloop until inkey$ <> \"\"");
        assert_eq!(format("WHILE 1\nCLS\nWEND\n", &opts), "while 1\n\tcls\nwend\n");
    }
}

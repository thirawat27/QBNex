//! Lossless lexer and concrete syntax tree for QBasic / QB64 / QBNex source.
//!
//! * [`lex`] splits text into tokens that cover every byte (comments, line
//!   numbers, labels, `$` metacommands and `'$INCLUDE` included).
//! * [`parse`] groups the tokens into statements and blocks
//!   (IF/FOR/DO/WHILE/SELECT/SUB/FUNCTION/TYPE/...), recovering from errors
//!   with diagnostics instead of failing.
//! * [`encoding`] reads CP437 or UTF-8 source bytes and writes them back.
//!
//! Nothing here panics on any input.

mod builtins_data;
pub mod cst;
pub mod encoding;
pub mod keywords;
pub mod lexer;
pub mod library;
mod library_data;
mod line_index;

pub use cst::{BlockKind, Diagnostic, Element, Node, NodeKind, Parse, Role, Severity, parse};
pub use encoding::Encoding;
pub use lexer::{Token, TokenKind, lex};
pub use line_index::LineIndex;

/// Decodes bytes (CP437 or UTF-8) and parses them.
pub fn parse_bytes(bytes: &[u8]) -> (Parse, Encoding) {
    let (text, enc) = encoding::decode(bytes);
    (parse(&text), enc)
}

/// The file named by an include directive token (`'$INCLUDE: 'x.bi'`,
/// `REM $INCLUDE: 'x'`, `$INCLUDE:'x'`), if the token is one.
pub fn include_path(token_text: &str) -> Option<&str> {
    let t = token_text.trim_start();
    let body = if let Some(r) = t.strip_prefix('\'') {
        r
    } else if t.len() >= 3 && t[..3].eq_ignore_ascii_case("REM") {
        &t[3..]
    } else {
        t
    };
    let body = body.trim_start();
    let rest = body.strip_prefix('$')?;
    if rest.len() < 7 || !rest[..7].eq_ignore_ascii_case("INCLUDE") {
        return None;
    }
    let rest = rest[7..].trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('\'')?;
    let end = rest.find('\'')?;
    let path = rest[..end].trim();
    (!path.is_empty()).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn include_forms() {
        assert_eq!(include_path("'$INCLUDE: 'lib/a.bi'"), Some("lib/a.bi"));
        assert_eq!(include_path("REM $include:'b.bm'"), Some("b.bm"));
        assert_eq!(include_path("$INCLUDE:'c.bi'"), Some("c.bi"));
        assert_eq!(include_path("' $DYNAMIC"), None);
        assert_eq!(include_path("'$INCLUDE"), None);
    }
}

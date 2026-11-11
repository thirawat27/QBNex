//! Lossless lexer. Every byte of the input belongs to exactly one token, in
//! order, so concatenating the token texts gives the input back.

use crate::keywords;

/// Token kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// Spaces, tabs, form feeds, a DOS end-of-file byte (0x1A).
    Whitespace,
    /// `\n`, `\r\n` or a lone `\r`.
    Newline,
    /// A `_` that continues the statement on the next line.
    LineContinuation,
    /// `' text`, `REM text`, or (QBNex) `# text` at the start of a statement.
    Comment,
    /// A comment holding a metacommand: `'$INCLUDE: 'x.bi'`, `REM $DYNAMIC`.
    MetaComment,
    /// A `$` metacommand line: `$CONSOLE:ONLY`, `$IF WIN THEN`, ...
    Metacommand,
    /// A line number at the start of a line.
    LineNumber,
    /// The name of a `label:` at the start of a line (the colon is a separate token).
    Label,
    Keyword,
    Ident,
    Number,
    /// A terminated string literal.
    String,
    /// A string literal that runs to the end of the line without a closing quote.
    UnterminatedString,
    /// The raw items of a `DATA` statement (trailing blanks excluded).
    DataText,
    /// Arithmetic/comparison/assignment operators: `+ - * / \ ^ = < > <= >= <> += ...`.
    Operator,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    Dot,
    Hash,
    Question,
    At,
    Dollar,
    Ampersand,
    /// Any character the lexer does not recognise.
    Error,
}

impl TokenKind {
    /// Whitespace, newlines and continuations: carry no meaning.
    pub fn is_trivia(self) -> bool {
        matches!(self, TokenKind::Whitespace | TokenKind::Newline | TokenKind::LineContinuation)
    }

    /// Comments of any kind (metacomments included).
    pub fn is_comment(self) -> bool {
        matches!(self, TokenKind::Comment | TokenKind::MetaComment)
    }

    /// Tokens that look like words or values (need a space between two of them).
    pub fn is_wordlike(self) -> bool {
        matches!(
            self,
            TokenKind::Keyword
                | TokenKind::Ident
                | TokenKind::Number
                | TokenKind::String
                | TokenKind::UnterminatedString
                | TokenKind::LineNumber
                | TokenKind::Label
                | TokenKind::DataText
                | TokenKind::Error
        )
    }
}

/// A token: kind plus byte range in the source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: u32,
    pub end: u32,
}

impl Token {
    pub fn text<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start as usize..self.end as usize]
    }
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

fn is_hspace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\x0c' | '\x1a' | '\0' | '\x0b')
}

/// Offset in `body` (the text after the opening quote of `$"...`) of the
/// closing quote, as the compiler's `SyntaxExt_InterpEnd&`: inside `{...}`
/// string literals and nested braces belong to the expression; `{{` and `}}`
/// outside one are escapes.
fn interp_end(body: &str) -> Option<usize> {
    let b = body.as_bytes();
    let mut depth = 0u32;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if depth == 0 {
            match c {
                b'"' => return Some(i),
                b'{' | b'}' if b.get(i + 1) == Some(&c) => i += 1,
                b'{' => depth = 1,
                _ => {}
            }
        } else {
            match c {
                b'"' => i += 1 + body[i + 1..].find('"')?,
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
        }
        i += 1;
    }
    None
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    /// Only whitespace since the last newline (line numbers, labels).
    line_start: bool,
    /// At the start of a statement (metacommands, `#` comments).
    stmt_start: bool,
}

/// Splits `src` into tokens. Never fails: unknown input becomes
/// [`TokenKind::Error`] tokens.
pub fn lex(src: &str) -> Vec<Token> {
    let mut lx = Lexer { src, pos: 0, tokens: Vec::new(), line_start: true, stmt_start: true };
    while lx.pos < src.len() {
        lx.step();
    }
    lx.tokens
}

impl<'a> Lexer<'a> {
    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }
    fn peek_at(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }
    fn push(&mut self, kind: TokenKind, end: usize) {
        debug_assert!(end > self.pos);
        self.tokens.push(Token { kind, start: self.pos as u32, end: end as u32 });
        self.pos = end;
    }
    /// End (byte offset) of the current line, excluding the newline.
    fn line_end(&self) -> usize {
        self.rest().find(['\r', '\n']).map_or(self.src.len(), |i| self.pos + i)
    }
    /// `end` moved back over trailing horizontal whitespace, but not before `min`.
    fn trim_back(&self, min: usize, end: usize) -> usize {
        let trimmed = self.src[min..end].trim_end_matches(is_hspace);
        min + trimmed.len()
    }

    fn step(&mut self) {
        let c = self.peek().unwrap();
        let start = self.pos;
        // Newlines.
        if c == '\r' || c == '\n' {
            let len = if self.rest().starts_with("\r\n") { 2 } else { 1 };
            let continued = self.tokens.iter().rev().find(|t| t.kind != TokenKind::Whitespace).is_some_and(|t| t.kind == TokenKind::LineContinuation);
            self.push(TokenKind::Newline, start + len);
            // A continued line does not start a new line for line numbers/labels.
            self.line_start = !continued;
            self.stmt_start = !continued;
            return;
        }
        if is_hspace(c) {
            let len = self.rest().find(|ch| !is_hspace(ch)).unwrap_or(self.rest().len());
            self.push(TokenKind::Whitespace, start + len);
            return;
        }
        let line_start = self.line_start;
        let stmt_start = self.stmt_start;
        self.line_start = false;
        self.stmt_start = false;

        match c {
            '\'' => {
                let end = self.trim_back(start, self.line_end());
                let body = self.src[start + 1..end].trim_start_matches(is_hspace);
                let kind = if body.starts_with('$') { TokenKind::MetaComment } else { TokenKind::Comment };
                self.push(kind, end.max(start + 1));
            }
            '$' if line_start && self.peek_at(1).is_some_and(|n| n.is_ascii_alphabetic()) => {
                let end = self.trim_back(start, self.line_end());
                self.push(TokenKind::Metacommand, end);
            }
            '#' if line_start && self.peek_at(1).is_none_or(|n| is_hspace(n) || n == '\r' || n == '\n' || n == '!') => {
                let end = self.trim_back(start, self.line_end());
                self.push(TokenKind::Comment, end.max(start + 1));
            }
            '"' => {
                let line_end = self.line_end();
                let body = &self.src[start + 1..line_end];
                let close = if start > 0 && self.src.as_bytes()[start - 1] == b'$' {
                    interp_end(body)
                } else {
                    body.find('"')
                };
                match close {
                    Some(i) => self.push(TokenKind::String, start + 1 + i + 1),
                    None => {
                        let end = self.trim_back(start + 1, line_end);
                        self.push(TokenKind::UnterminatedString, end);
                    }
                }
            }
            '0'..='9' => {
                let end = self.scan_number(start);
                let kind = if line_start && self.src[start..end].bytes().all(|b| b.is_ascii_digit()) {
                    TokenKind::LineNumber
                } else {
                    TokenKind::Number
                };
                self.push(kind, end);
                if kind == TokenKind::LineNumber {
                    self.stmt_start = true;
                }
            }
            '.' if self.peek_at(1).is_some_and(|n| n.is_ascii_digit()) => {
                let end = self.scan_number(start);
                self.push(TokenKind::Number, end);
            }
            '&' if matches!(self.peek_at(1), Some('h' | 'H' | 'o' | 'O' | 'b' | 'B')) => {
                let radix = self.peek_at(1).unwrap().to_ascii_uppercase();
                let mut end = start + 2;
                let bytes = self.src.as_bytes();
                while end < bytes.len() {
                    let b = bytes[end];
                    let ok = match radix {
                        'H' => b.is_ascii_hexdigit(),
                        'O' => (b'0'..=b'7').contains(&b),
                        _ => b == b'0' || b == b'1',
                    };
                    if !ok {
                        break;
                    }
                    end += 1;
                }
                end = self.scan_suffix(end, true);
                self.push(TokenKind::Number, end);
            }
            '_' if self.is_continuation(start) => {
                self.push(TokenKind::LineContinuation, start + 1);
                self.stmt_start = stmt_start;
                self.line_start = line_start;
            }
            c if is_ident_start(c) => self.word(start, line_start, stmt_start),
            _ => self.punct(start, c, stmt_start),
        }
    }

    /// `_` followed only by blanks until the end of the line (or input), and
    /// not glued to a preceding word.
    fn is_continuation(&self, at: usize) -> bool {
        let after = &self.src[at + 1..];
        let rest_line = after.split(['\r', '\n']).next().unwrap_or("");
        if !rest_line.chars().all(is_hspace) {
            return false;
        }
        let before = self.src[..at].chars().next_back();
        before.is_none_or(|b| !is_ident_char(b))
    }

    fn scan_number(&self, start: usize) -> usize {
        let b = self.src.as_bytes();
        let mut i = start;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i < b.len() && b[i] == b'.' {
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
        // Exponent: E/D/F followed by optional sign and digits.
        if i < b.len() && matches!(b[i], b'e' | b'E' | b'd' | b'D' | b'f' | b'F') {
            let mut j = i + 1;
            if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                j += 1;
            }
            if j < b.len() && b[j].is_ascii_digit() {
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                i = j;
            }
        }
        self.scan_suffix(i, true)
    }

    /// Type suffix after a number or name: `% & ! # $ ~ \``, plus digits after a backtick.
    fn scan_suffix(&self, mut i: usize, number: bool) -> usize {
        let b = self.src.as_bytes();
        let start = i;
        while i < b.len() && i - start < 3 {
            match b[i] {
                b'%' | b'!' | b'#' | b'~' => i += 1,
                b'$' if !number => i += 1,
                b'&' => i += 1,
                b'`' => {
                    i += 1;
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                _ => break,
            }
        }
        i
    }

    fn word(&mut self, start: usize, line_start: bool, stmt_start: bool) {
        let len = self.rest().find(|ch: char| !is_ident_char(ch)).unwrap_or(self.rest().len());
        let word_end = start + len;
        let word = &self.src[start..word_end];
        let upper = word.to_ascii_uppercase();

        // REM comment.
        if upper == "REM" {
            let end = self.trim_back(start, self.line_end());
            let body = self.src[word_end..end].trim_start_matches(is_hspace);
            let kind = if body.starts_with('$') { TokenKind::MetaComment } else { TokenKind::Comment };
            self.push(kind, end);
            return;
        }
        let mut end = self.scan_suffix(word_end, false);
        // `PRINT#1`, `INPUT#f`: a keyword never takes a `#` suffix.
        if end > word_end && self.src.as_bytes()[word_end] == b'#' && keywords::is_keyword(word) && !keywords::is_keyword(&self.src[start..end]) {
            end = word_end;
        }
        let full = &self.src[start..end];
        // Label at the start of a line.
        if line_start && end == word_end && self.src[end..].starts_with(':') && !keywords::is_keyword(word) && !word.contains('.') {
            self.push(TokenKind::Label, end);
            self.stmt_start = true;
            return;
        }
        let kw = !word.contains('.') && (keywords::is_keyword(full) || (end == word_end && keywords::is_keyword(word)));
        let kind = if kw { TokenKind::Keyword } else { TokenKind::Ident };
        self.push(kind, end);
        if kw && upper == "DATA" && end == word_end {
            self.data_text();
        }
        let _ = stmt_start;
    }

    /// After `DATA`: the raw items up to `:` (outside quotes) or the end of the line.
    fn data_text(&mut self) {
        let ws = self.rest().find(|c| !is_hspace(c)).unwrap_or(self.rest().len());
        if ws > 0 {
            self.push(TokenKind::Whitespace, self.pos + ws);
        }
        let start = self.pos;
        let line_end = self.line_end();
        let mut in_str = false;
        let mut end = line_end;
        for (i, ch) in self.src[start..line_end].char_indices() {
            match ch {
                '"' => in_str = !in_str,
                ':' if !in_str => {
                    end = start + i;
                    break;
                }
                _ => {}
            }
        }
        let end = self.trim_back(start, end);
        if end > start {
            self.push(TokenKind::DataText, end);
        }
    }

    fn punct(&mut self, start: usize, c: char, _stmt_start: bool) {
        use TokenKind::*;
        let op2 = ["<=", ">=", "<>", "=<", "=>", "><", "+=", "-=", "*=", "/=", "\\=", "^="];
        if op2.iter().any(|op| self.rest().starts_with(op)) {
            self.push(Operator, start + 2);
            return;
        }
        let kind = match c {
            '+' | '-' | '*' | '/' | '\\' | '^' | '=' | '<' | '>' => Operator,
            '(' => LParen,
            ')' => RParen,
            '[' => LBracket,
            ']' => RBracket,
            '{' => LBrace,
            '}' => RBrace,
            ',' => Comma,
            ';' => Semicolon,
            ':' => Colon,
            '.' => Dot,
            '#' => Hash,
            '?' => Question,
            '@' => At,
            '$' => Dollar,
            '&' => Ampersand,
            _ => Error,
        };
        if kind == Colon {
            self.stmt_start = true;
        }
        if kind == Question {
            // `?` is PRINT: what follows is not at statement start.
            self.stmt_start = false;
        }
        self.push(kind, start + c.len_utf8());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<(TokenKind, &str)> {
        lex(src).into_iter().filter(|t| t.kind != TokenKind::Whitespace).map(|t| (t.kind, t.text(src))).collect()
    }

    #[test]
    fn basic_line() {
        use TokenKind::*;
        assert_eq!(
            kinds("10 PRINT a$; LEFT$(b$, 2) ' hi\n"),
            vec![
                (LineNumber, "10"),
                (Keyword, "PRINT"),
                (Ident, "a$"),
                (Semicolon, ";"),
                (Keyword, "LEFT$"),
                (LParen, "("),
                (Ident, "b$"),
                (Comma, ","),
                (Number, "2"),
                (RParen, ")"),
                (Comment, "' hi"),
                (Newline, "\n"),
            ]
        );
    }

    #[test]
    fn metas_labels_data() {
        use TokenKind::*;
        assert_eq!(kinds("$CONSOLE:ONLY")[0], (Metacommand, "$CONSOLE:ONLY"));
        assert_eq!(kinds("'$INCLUDE: 'a.bi'  ")[0], (MetaComment, "'$INCLUDE: 'a.bi'"));
        assert_eq!(kinds("rem $dynamic")[0], (MetaComment, "rem $dynamic"));
        assert_eq!(kinds("start: CLS")[..2], [(Label, "start"), (Colon, ":")]);
        assert_eq!(kinds("DATA 1, \"a:b\", x y : PRINT")[1], (DataText, "1, \"a:b\", x y"));
        assert_eq!(kinds("x = &HFF& + 1.5E+3#")[2], (Number, "&HFF&"));
        assert_eq!(kinds("x = &HFF& + 1.5E+3#")[4], (Number, "1.5E+3#"));
        assert_eq!(kinds("a = b _\n + c")[3], (LineContinuation, "_"));
        assert_eq!(kinds("PRINT \"abc")[1], (UnterminatedString, "\"abc"));
        assert_eq!(kinds("PRINT #1, x")[1], (Hash, "#"));
        assert_eq!(kinds("# comment")[0], (Comment, "# comment"));
    }

    #[test]
    fn interpolated_strings_may_quote_inside_braces() {
        use TokenKind::*;
        // the compiler's SyntaxExt_InterpEnd&: quotes inside {...} belong to the expression
        let k = kinds("PRINT $\"[{n$ + \"!\"}] {LEN(\"abc\")}\": x = 1");
        assert_eq!(k[1], (Dollar, "$"));
        assert_eq!(k[2], (String, "\"[{n$ + \"!\"}] {LEN(\"abc\")}\""));
        assert_eq!(k[3], (Colon, ":"));
        // nested braces, and {{ }} escapes outside an expression
        assert_eq!(kinds("? $\"{{x}} {f({1})}\"")[2], (String, "\"{{x}} {f({1})}\""));
        // a plain string is unchanged
        assert_eq!(kinds("PRINT \"{\" + \"x\"")[1], (String, "\"{\""));
        assert_eq!(kinds("PRINT $\"{a + \"b")[2], (UnterminatedString, "\"{a + \"b"));
    }

    #[test]
    fn lossless() {
        let src = "a\r\nb\rc\n\u{1a}  é ' x\n";
        let s: String = lex(src).iter().map(|t| t.text(src)).collect();
        assert_eq!(s, src);
    }
}

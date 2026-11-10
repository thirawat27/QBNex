//! Concrete syntax tree: statements grouped into blocks. Lossless: walking the
//! tree in order visits every token exactly once.

use crate::lexer::{Token, TokenKind, lex};

/// Kinds of block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockKind {
    If,
    For,
    Do,
    While,
    Select,
    Sub,
    Function,
    Type,
    DefFn,
    DeclareLibrary,
    Class,
    Enum,
    Try,
}

impl BlockKind {
    /// Opening keyword(s), for messages.
    pub fn opener(self) -> &'static str {
        match self {
            BlockKind::If => "IF",
            BlockKind::For => "FOR",
            BlockKind::Do => "DO",
            BlockKind::While => "WHILE",
            BlockKind::Select => "SELECT CASE",
            BlockKind::Sub => "SUB",
            BlockKind::Function => "FUNCTION",
            BlockKind::Type => "TYPE",
            BlockKind::DefFn => "DEF FN",
            BlockKind::DeclareLibrary => "DECLARE LIBRARY",
            BlockKind::Class => "CLASS",
            BlockKind::Enum => "ENUM",
            BlockKind::Try => "TRY",
        }
    }
    /// Closing keyword(s), for messages.
    pub fn closer(self) -> &'static str {
        match self {
            BlockKind::If => "END IF",
            BlockKind::For => "NEXT",
            BlockKind::Do => "LOOP",
            BlockKind::While => "WEND",
            BlockKind::Select => "END SELECT",
            BlockKind::Sub => "END SUB",
            BlockKind::Function => "END FUNCTION",
            BlockKind::Type => "END TYPE",
            BlockKind::DefFn => "END DEF",
            BlockKind::DeclareLibrary => "END DECLARE",
            BlockKind::Class => "END CLASS",
            BlockKind::Enum => "END ENUM",
            BlockKind::Try => "END TRY",
        }
    }
}

/// What a statement does to the block structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Plain,
    Open(BlockKind),
    /// `ELSE`, `ELSEIF`, `CASE`, `CATCH`, `FINALLY`.
    Middle(BlockKind),
    /// Closes this many blocks of the kind (`NEXT j, i` closes two).
    Close(BlockKind, u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Root,
    /// Header statement, then alternating bodies and middle statements, then
    /// the closing statement (missing when the block is unclosed or closed
    /// by a shared `NEXT`).
    Block(BlockKind),
    /// Contents of a block between two of its statements.
    Body,
    Statement(Role),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Element {
    Node(Node),
    /// Index into [`Parse::tokens`].
    Token(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub children: Vec<Element>,
}

impl Node {
    fn new(kind: NodeKind) -> Self {
        Node { kind, children: Vec::new() }
    }

    /// Token indices of this node, in order.
    pub fn token_indices(&self) -> Vec<u32> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect(&self, out: &mut Vec<u32>) {
        for c in &self.children {
            match c {
                Element::Token(t) => out.push(*t),
                Element::Node(n) => n.collect(out),
            }
        }
    }

    fn first_token(&self) -> Option<u32> {
        self.children.iter().find_map(|c| match c {
            Element::Token(t) => Some(*t),
            Element::Node(n) => n.first_token(),
        })
    }

    fn last_token(&self) -> Option<u32> {
        self.children.iter().rev().find_map(|c| match c {
            Element::Token(t) => Some(*t),
            Element::Node(n) => n.last_token(),
        })
    }

    /// Byte range covered by the node (empty at 0 for an empty node).
    pub fn range(&self, tokens: &[Token]) -> std::ops::Range<usize> {
        match (self.first_token(), self.last_token()) {
            (Some(a), Some(b)) => tokens[a as usize].start as usize..tokens[b as usize].end as usize,
            _ => 0..0,
        }
    }

    /// Child nodes.
    pub fn child_nodes(&self) -> impl Iterator<Item = &Node> {
        self.children.iter().filter_map(|c| match c {
            Element::Node(n) => Some(n),
            Element::Token(_) => None,
        })
    }

    /// The header statement of a block.
    pub fn header(&self) -> Option<&Node> {
        match self.kind {
            NodeKind::Block(_) => self.child_nodes().next(),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: std::ops::Range<usize>,
    pub severity: Severity,
    pub message: String,
}

/// Result of parsing one source text.
#[derive(Clone, Debug)]
pub struct Parse {
    pub text: String,
    pub tokens: Vec<Token>,
    pub root: Node,
    pub diagnostics: Vec<Diagnostic>,
}

impl Parse {
    pub fn token_text(&self, idx: u32) -> &str {
        self.tokens[idx as usize].text(&self.text)
    }

    /// Texts of the meaningful tokens of a statement node.
    pub fn statement_words(&self, stmt: &Node) -> Vec<(TokenKind, &str)> {
        stmt.token_indices()
            .into_iter()
            .map(|i| &self.tokens[i as usize])
            .filter(|t| !t.kind.is_trivia())
            .map(|t| (t.kind, t.text(&self.text)))
            .collect()
    }

    /// Every statement node with its block depth, in source order.
    pub fn statements(&self) -> Vec<(&Node, usize)> {
        fn walk<'a>(n: &'a Node, depth: usize, out: &mut Vec<(&'a Node, usize)>) {
            for c in n.child_nodes() {
                match c.kind {
                    NodeKind::Statement(_) => out.push((c, depth)),
                    NodeKind::Block(_) => walk(c, depth, out),
                    NodeKind::Body => walk(c, depth + 1, out),
                    NodeKind::Root => walk(c, depth, out),
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, 0, &mut out);
        out
    }

    /// Reassembles the text from the tree (equals [`Parse::text`]).
    pub fn to_text(&self) -> String {
        self.root.token_indices().iter().map(|&i| self.token_text(i)).collect()
    }
}

/// Parses source text. Never panics; problems become diagnostics.
pub fn parse(text: &str) -> Parse {
    let tokens = lex(text);
    let mut diagnostics = Vec::new();
    for t in &tokens {
        match t.kind {
            TokenKind::UnterminatedString => diagnostics.push(Diagnostic {
                range: t.range(),
                severity: Severity::Error,
                message: "unterminated string literal".into(),
            }),
            TokenKind::Error => diagnostics.push(Diagnostic {
                range: t.range(),
                severity: Severity::Error,
                message: format!("unexpected character {:?}", t.text(text)),
            }),
            _ => {}
        }
    }
    let items = split_statements(text, &tokens);
    let root = Builder { text, tokens: &tokens, diagnostics: &mut diagnostics, stack: Vec::new(), root: Node::new(NodeKind::Root) }.build(items);
    diagnostics.sort_by_key(|d| d.range.start);
    Parse { text: text.to_owned(), tokens, root, diagnostics }
}

enum Item {
    Token(u32),
    Stmt(Vec<u32>),
}

fn upper(text: &str, t: &Token) -> String {
    t.text(text).to_ascii_uppercase()
}

fn split_statements(text: &str, tokens: &[Token]) -> Vec<Item> {
    let mut items = Vec::new();
    let mut cur: Vec<u32> = Vec::new();
    let mut held: Vec<u32> = Vec::new();
    let mut then_seen = false;
    let mut single_if = false;

    fn flush(items: &mut Vec<Item>, cur: &mut Vec<u32>, held: &mut Vec<u32>) {
        if !cur.is_empty() {
            items.push(Item::Stmt(std::mem::take(cur)));
        }
        items.extend(held.drain(..).map(Item::Token));
    }

    for (i, t) in tokens.iter().enumerate() {
        let i = i as u32;
        match t.kind {
            TokenKind::Whitespace | TokenKind::LineContinuation => {
                if cur.is_empty() {
                    items.push(Item::Token(i));
                } else {
                    held.push(i);
                }
            }
            TokenKind::Newline => {
                let continued = !cur.is_empty()
                    && held.iter().rev().map(|&h| tokens[h as usize].kind).find(|k| *k != TokenKind::Whitespace)
                        == Some(TokenKind::LineContinuation);
                if continued {
                    held.push(i);
                } else {
                    flush(&mut items, &mut cur, &mut held);
                    items.push(Item::Token(i));
                    then_seen = false;
                    single_if = false;
                }
            }
            // `IF x THEN : stmt` is a single-line IF too.
            TokenKind::Colon if (single_if || then_seen) && !cur.is_empty() => {
                single_if = true;
                cur.append(&mut held);
                cur.push(i);
            }
            TokenKind::Comment
            | TokenKind::MetaComment
            | TokenKind::Colon
            | TokenKind::LineNumber
            | TokenKind::Label
            | TokenKind::Metacommand => {
                flush(&mut items, &mut cur, &mut held);
                items.push(Item::Token(i));
                if t.kind != TokenKind::Colon || !single_if {
                    then_seen = false;
                    single_if = false;
                }
            }
            _ => {
                if cur.is_empty() {
                    then_seen = false;
                    single_if = false;
                }
                cur.append(&mut held);
                cur.push(i);
                let first_is_if = cur.first().is_some_and(|&f| {
                    let ft = &tokens[f as usize];
                    ft.kind == TokenKind::Keyword && matches!(upper(text, ft).as_str(), "IF" | "ELSEIF")
                });
                if then_seen && first_is_if {
                    single_if = true;
                } else if first_is_if && t.kind == TokenKind::Keyword && upper(text, t) == "THEN" {
                    then_seen = true;
                }
            }
        }
    }
    flush(&mut items, &mut cur, &mut held);
    items
}

fn classify(text: &str, tokens: &[Token], stmt: &[u32]) -> Role {
    let sig: Vec<&Token> = stmt.iter().map(|&i| &tokens[i as usize]).filter(|t| !t.kind.is_trivia()).collect();
    let Some(first) = sig.first() else { return Role::Plain };
    if first.kind != TokenKind::Keyword {
        return Role::Plain;
    }
    let kw = |n: usize| -> String {
        sig.get(n).filter(|t| t.kind == TokenKind::Keyword).map(|t| upper(text, t)).unwrap_or_default()
    };
    let head = kw(0);
    let last_is_then = sig.last().is_some_and(|t| t.kind == TokenKind::Keyword && upper(text, t) == "THEN");
    match head.as_str() {
        "IF" if last_is_then => Role::Open(BlockKind::If),
        "ELSEIF" | "ELSE" => Role::Middle(BlockKind::If),
        "ENDIF" => Role::Close(BlockKind::If, 1),
        "END" => {
            let k = match kw(1).as_str() {
                "IF" => BlockKind::If,
                "SELECT" => BlockKind::Select,
                "SUB" => BlockKind::Sub,
                "FUNCTION" => BlockKind::Function,
                "TYPE" => BlockKind::Type,
                "DEF" => BlockKind::DefFn,
                "DECLARE" => BlockKind::DeclareLibrary,
                "CLASS" => BlockKind::Class,
                "ENUM" => BlockKind::Enum,
                "TRY" => BlockKind::Try,
                _ => return Role::Plain,
            };
            Role::Close(k, 1)
        }
        "FOR" => Role::Open(BlockKind::For),
        "NEXT" => {
            let mut depth = 0i32;
            let mut commas = 0u16;
            for t in &sig[1..] {
                match t.kind {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    TokenKind::Comma if depth == 0 => commas = commas.saturating_add(1),
                    _ => {}
                }
            }
            Role::Close(BlockKind::For, commas.saturating_add(1))
        }
        "DO" => Role::Open(BlockKind::Do),
        "LOOP" => Role::Close(BlockKind::Do, 1),
        "WHILE" => Role::Open(BlockKind::While),
        "WEND" => Role::Close(BlockKind::While, 1),
        "SELECT" => Role::Open(BlockKind::Select),
        "CASE" => Role::Middle(BlockKind::Select),
        "SUB" => Role::Open(BlockKind::Sub),
        "FUNCTION" => Role::Open(BlockKind::Function),
        "TYPE" if sig.get(1).is_some_and(|t| t.kind == TokenKind::Ident) => Role::Open(BlockKind::Type),
        "DEF" => {
            let fn_name = sig.get(1).is_some_and(|t| {
                t.kind == TokenKind::Ident && t.text(text).get(..2).is_some_and(|p| p.eq_ignore_ascii_case("FN"))
            });
            let mut depth = 0i32;
            let has_eq = sig.iter().any(|t| {
                match t.kind {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    _ => {}
                }
                depth == 0 && t.kind == TokenKind::Operator && t.text(text) == "="
            });
            if fn_name && !has_eq { Role::Open(BlockKind::DefFn) } else { Role::Plain }
        }
        "DECLARE" if sig.iter().any(|t| t.kind == TokenKind::Keyword && upper(text, t) == "LIBRARY") => {
            Role::Open(BlockKind::DeclareLibrary)
        }
        "CLASS" => Role::Open(BlockKind::Class),
        "ENUM" => Role::Open(BlockKind::Enum),
        "TRY" => Role::Open(BlockKind::Try),
        "CATCH" | "FINALLY" => Role::Middle(BlockKind::Try),
        _ => Role::Plain,
    }
}

struct Frame {
    kind: BlockKind,
    block: Node,
    body: Node,
    header_range: std::ops::Range<usize>,
}

struct Builder<'a> {
    text: &'a str,
    tokens: &'a [Token],
    diagnostics: &'a mut Vec<Diagnostic>,
    stack: Vec<Frame>,
    root: Node,
}

impl Builder<'_> {
    fn container(&mut self) -> &mut Node {
        match self.stack.last_mut() {
            Some(f) => &mut f.body,
            None => &mut self.root,
        }
    }

    fn range_of(&self, stmt: &[u32]) -> std::ops::Range<usize> {
        let a = &self.tokens[*stmt.first().unwrap() as usize];
        let b = &self.tokens[*stmt.last().unwrap() as usize];
        a.start as usize..b.end as usize
    }

    fn error(&mut self, range: std::ops::Range<usize>, message: String) {
        self.diagnostics.push(Diagnostic { range, severity: Severity::Error, message });
    }

    /// Pops the top frame and appends the finished block to the new container.
    fn pop(&mut self, closer: Option<Node>) {
        let mut f = self.stack.pop().unwrap();
        f.block.children.push(Element::Node(f.body));
        if let Some(c) = closer {
            f.block.children.push(Element::Node(c));
        }
        self.container().children.push(Element::Node(f.block));
    }

    fn pop_unclosed(&mut self) {
        let f = self.stack.last().unwrap();
        let (range, kind) = (f.header_range.clone(), f.kind);
        self.error(range, format!("{} without {}", kind.opener(), kind.closer()));
        self.pop(None);
    }

    /// Makes the innermost frame of `kind` the top one, closing frames above it.
    fn unwind_to(&mut self, kind: BlockKind) -> bool {
        let Some(pos) = self.stack.iter().rposition(|f| f.kind == kind) else { return false };
        while self.stack.len() > pos + 1 {
            self.pop_unclosed();
        }
        true
    }

    fn build(mut self, items: Vec<Item>) -> Node {
        for item in items {
            match item {
                Item::Token(t) => self.container().children.push(Element::Token(t)),
                Item::Stmt(stmt) => self.statement(stmt),
            }
        }
        while !self.stack.is_empty() {
            self.pop_unclosed();
        }
        self.root
    }

    fn statement(&mut self, stmt: Vec<u32>) {
        let mut role = classify(self.text, self.tokens, &stmt);
        // Inside DECLARE LIBRARY, SUB/FUNCTION lines are declarations, not blocks.
        if matches!(role, Role::Open(BlockKind::Sub | BlockKind::Function))
            && self.stack.last().is_some_and(|f| f.kind == BlockKind::DeclareLibrary)
        {
            role = Role::Plain;
        }
        let range = self.range_of(&stmt);
        let node = |role| Node { kind: NodeKind::Statement(role), children: stmt.iter().map(|&t| Element::Token(t)).collect() };
        match role {
            Role::Plain => self.container().children.push(Element::Node(node(role))),
            Role::Open(kind) => {
                if matches!(kind, BlockKind::Sub | BlockKind::Function)
                    && self.stack.iter().any(|f| matches!(f.kind, BlockKind::Sub | BlockKind::Function))
                {
                    self.error(range.clone(), format!("{} inside another SUB or FUNCTION", kind.opener()));
                }
                let mut block = Node::new(NodeKind::Block(kind));
                block.children.push(Element::Node(node(role)));
                self.stack.push(Frame { kind, block, body: Node::new(NodeKind::Body), header_range: range });
            }
            Role::Middle(kind) => {
                if self.unwind_to(kind) {
                    let f = self.stack.last_mut().unwrap();
                    let body = std::mem::replace(&mut f.body, Node::new(NodeKind::Body));
                    f.block.children.push(Element::Node(body));
                    f.block.children.push(Element::Node(node(role)));
                } else {
                    let word = self.tokens[stmt[0] as usize].text(self.text).to_ascii_uppercase();
                    self.error(range, format!("{word} without {}", kind.opener()));
                    self.container().children.push(Element::Node(node(Role::Plain)));
                }
            }
            Role::Close(kind, count) => {
                let available = self.stack.iter().filter(|f| f.kind == kind).count();
                if available == 0 {
                    self.error(range, format!("{} without {}", kind.closer(), kind.opener()));
                    self.container().children.push(Element::Node(node(Role::Plain)));
                    return;
                }
                if (count as usize) > available {
                    self.error(range.clone(), format!("{} closes more blocks than are open", kind.closer()));
                }
                let n = (count as usize).min(available);
                for k in 0..n {
                    self.unwind_to(kind);
                    if k + 1 == n {
                        self.pop(Some(node(role)));
                    } else {
                        self.pop(None);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles(src: &str) -> Vec<(Role, usize)> {
        let p = parse(src);
        p.statements().into_iter().map(|(n, d)| match n.kind {
            NodeKind::Statement(r) => (r, d),
            _ => unreachable!(),
        }).collect()
    }

    #[test]
    fn nesting_depths() {
        let src = "SUB s\nFOR i = 1 TO 2\nIF i THEN\nPRINT i\nELSE\nPRINT 0\nEND IF\nNEXT\nEND SUB\n";
        let p = parse(src);
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        let d: Vec<usize> = roles(src).into_iter().map(|r| r.1).collect();
        assert_eq!(d, vec![0, 1, 2, 3, 2, 3, 2, 1, 0]);
        assert_eq!(p.to_text(), src);
    }

    #[test]
    fn single_line_if_and_colons() {
        let src = "IF a THEN PRINT 1: PRINT 2 ELSE PRINT 3\nFOR i = 1 TO 3: PRINT i: NEXT\nIF b THEN : RETURN\n";
        let p = parse(src);
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        assert_eq!(roles(src).len(), 5);
    }

    #[test]
    fn declare_library_bodies() {
        let p = parse("DECLARE LIBRARY\nFUNCTION GetTicks& ()\nSUB Beep2 (BYVAL f&)\nEND DECLARE\n");
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        let d: Vec<usize> = p.statements().into_iter().map(|s| s.1).collect();
        assert_eq!(d, vec![0, 1, 1, 0]);
    }

    #[test]
    fn next_closes_many() {
        let p = parse("FOR i = 1 TO 2\nFOR j = 1 TO 2\nNEXT j, i\n");
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    }

    #[test]
    fn recovery() {
        let p = parse("FOR i = 1 TO 2\nIF x THEN\nNEXT\nEND IF\nWEND\n");
        let msgs: Vec<&str> = p.diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(msgs, vec!["IF without END IF", "END IF without IF", "WEND without WHILE"]);
        let p = parse("SUB a\nPRINT \"x\n");
        assert_eq!(p.diagnostics.len(), 2);
    }
}

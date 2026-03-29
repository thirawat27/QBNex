use crate::ast_nodes::*;
use core_types::{QError, QResult, QType};
use std::collections::{HashMap, HashSet, VecDeque};
use tokenizer::scanner::Scanner;
use tokenizer::tokens::{Keyword, SpannedToken, Token, TokenSpan};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    current: usize,
    known_functions: HashSet<String>,
    collected_user_types: HashMap<String, UserType>,
    pending_statements: VecDeque<Statement>,
}

impl Parser {
    pub fn new(input: String) -> QResult<Self> {
        let mut scanner = Scanner::new(input);
        let tokens = scanner.tokenize_spanned()?;
        Ok(Self {
            tokens,
            current: 0,
            known_functions: HashSet::new(),
            collected_user_types: HashMap::new(),
            pending_statements: VecDeque::new(),
        })
    }

    fn peek_spanned(&self) -> Option<&SpannedToken> {
        self.tokens.get(self.current)
    }

    fn peek(&self) -> Option<&Token> {
        self.peek_spanned().map(|token| &token.token)
    }

    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.current + 1).map(|token| &token.token)
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn previous(&self) -> Option<&Token> {
        if self.current > 0 {
            self.tokens.get(self.current - 1).map(|token| &token.token)
        } else {
            None
        }
    }

    fn current_span(&self) -> Option<TokenSpan> {
        self.peek_spanned()
            .or_else(|| {
                self.current
                    .checked_sub(1)
                    .and_then(|idx| self.tokens.get(idx))
            })
            .map(|token| token.span)
    }

    fn is_at_end(&self) -> bool {
        self.peek() == Some(&Token::Eof) || self.current >= self.tokens.len()
    }

    fn check(&self, token_type: &Token) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek() == Some(token_type)
    }

    fn check_keyword(&self, kw: Keyword) -> bool {
        if self.is_at_end() {
            return false;
        }
        matches!(self.peek(), Some(Token::Keyword(k)) if *k == kw)
    }

    fn match_token(&mut self, types: &[Token]) -> bool {
        for t in types {
            if self.check(t) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn match_keyword(&mut self, kw: Keyword) -> bool {
        if self.check_keyword(kw) {
            self.advance();
            return true;
        }
        false
    }

    fn match_identifier_ci(&mut self, expected: &str) -> bool {
        match self.peek() {
            Some(Token::Identifier(name)) if name.eq_ignore_ascii_case(expected) => {
                self.advance();
                true
            }
            _ => false,
        }
    }

    fn debug_token(token: &Token) -> String {
        match token {
            Token::Keyword(keyword) => format!("Keyword({keyword:?})"),
            Token::Identifier(name) => format!("Identifier({name})"),
            Token::IntegerLiteral(value) => format!("IntegerLiteral({value})"),
            Token::LongLiteral(value) => format!("LongLiteral({value})"),
            Token::SingleLiteral(value) => format!("SingleLiteral({value})"),
            Token::DoubleLiteral(value) => format!("DoubleLiteral({value})"),
            Token::StringLiteral(value) => {
                let mut preview = value.clone();
                if preview.len() > 24 {
                    preview.truncate(24);
                    preview.push_str("...");
                }
                format!("StringLiteral({preview:?})")
            }
            other => format!("{other:?}"),
        }
    }

    fn current_token_context(&self) -> String {
        if self.tokens.is_empty() {
            return "no tokens".to_string();
        }

        let start = self.current.saturating_sub(3);
        let end = (self.current + 4).min(self.tokens.len());
        let mut parts = Vec::new();

        for idx in start..end {
            let marker = if idx == self.current { ">>" } else { "  " };
            parts.push(format!(
                "{marker}{idx}: {}",
                Self::debug_token(&self.tokens[idx].token)
            ));
        }

        parts.join(", ")
    }

    fn syntax_error_here(&self, message: impl Into<String>) -> QError {
        let message = format!(
            "{} at token {} [{}]",
            message.into(),
            self.current,
            self.current_token_context()
        );
        if let Some(span) = self.current_span() {
            QError::syntax_at(message, span.offset, span.len)
        } else {
            QError::syntax(message)
        }
    }

    fn consume(&mut self, expected: Token, message: &str) -> QResult<&Token> {
        if self.check(&expected) {
            return Ok(self.advance().unwrap());
        }
        Err(self.syntax_error_here(format!(
            "{} (expected {}, found {})",
            message,
            Self::debug_token(&expected),
            self.peek()
                .map(Self::debug_token)
                .unwrap_or_else(|| "EOF".to_string())
        )))
    }

    fn consume_keyword(&mut self, expected: Keyword, message: &str) -> QResult<()> {
        if self.check_keyword(expected) {
            self.advance();
            return Ok(());
        }
        Err(self.syntax_error_here(format!(
            "{} (expected keyword {:?}, found {})",
            message,
            expected,
            self.peek()
                .map(Self::debug_token)
                .unwrap_or_else(|| "EOF".to_string())
        )))
    }

    fn skip_newlines(&mut self) {
        while self.match_token(&[Token::Newline, Token::Colon, Token::LineContinuation]) {}
    }

    fn keyword_is_used_as_identifier(&self) -> bool {
        matches!(
            self.peek_next(),
            Some(Token::Equal | Token::OpenParen | Token::Dot)
        )
    }

    fn is_empty_declared_type_annotation(
        annotation: &(Option<char>, Option<String>, Option<usize>),
    ) -> bool {
        annotation.0.is_none() && annotation.1.is_none() && annotation.2.is_none()
    }

    fn parse_variable_declarations(
        &mut self,
        context: &str,
    ) -> QResult<Vec<(Variable, Option<Vec<ArrayDimension>>)>> {
        let shared_annotation = self.parse_declared_type_annotation()?;
        let mut variables = Vec::new();

        loop {
            let name =
                self.parse_qualified_name(&format!("Expected variable name after {context}"))?;
            let dimensions = self.parse_array_dimensions()?;

            let mut annotation = self.parse_declared_type_annotation()?;
            if Self::is_empty_declared_type_annotation(&annotation) {
                annotation = shared_annotation.clone();
            }
            let (type_suffix, declared_type, fixed_length) = annotation;
            let type_suffix = type_suffix.or_else(|| Variable::suffix_from_name(&name));

            variables.push((
                Variable {
                    name,
                    by_val: false,
                    type_suffix,
                    declared_type,
                    fixed_length,
                    indices: Vec::new(),
                },
                dimensions,
            ));

            if !self.match_token(&[Token::Comma]) {
                break;
            }
        }

        Ok(variables)
    }

    fn parse_name_segment(&mut self, message: &str) -> QResult<String> {
        match self.peek() {
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            Some(Token::Keyword(keyword)) => {
                let keyword = format!("{keyword:?}");
                self.advance();
                Ok(keyword)
            }
            _ => Err(self.syntax_error_here(message.to_string())),
        }
    }

    fn parse_qualified_name(&mut self, message: &str) -> QResult<String> {
        let mut name = self.parse_name_segment(message)?;
        while self.match_token(&[Token::Dot]) {
            let segment = self.parse_name_segment("Expected identifier after '.'")?;
            name.push('.');
            name.push_str(&segment);
        }
        Ok(name)
    }

    fn parse_metacommand(&mut self) {
        self.advance();
        while !self.check(&Token::Newline) && !self.is_at_end() {
            self.advance();
        }
    }

    fn consume_declared_type_name(&mut self, message: &str) -> QResult<String> {
        let Some(token) = self.advance() else {
            return Err(QError::Syntax(message.to_string()));
        };

        let mut type_name = match token {
            Token::Identifier(type_name) => type_name.clone(),
            Token::Keyword(Keyword::String) => "STRING".to_string(),
            Token::Keyword(kw) => format!("{:?}", kw).to_uppercase(),
            _ => return Err(QError::Syntax(message.to_string())),
        };

        if matches!(type_name.to_uppercase().as_str(), "_UNSIGNED" | "UNSIGNED") {
            if let Some(next_type) = self.peek().and_then(|token| match token {
                Token::Identifier(name) => Some(name.clone()),
                Token::Keyword(Keyword::String) => Some("STRING".to_string()),
                Token::Keyword(kw) => Some(format!("{:?}", kw).to_uppercase()),
                _ => None,
            }) {
                type_name = format!("{} {}", type_name, next_type);
                self.advance();
            }
        }

        Ok(type_name.to_uppercase())
    }

    fn parse_declared_type_spec(
        &mut self,
        missing_type_message: &str,
        fixed_length_message: &str,
    ) -> QResult<(String, Option<usize>)> {
        let type_upper = self.consume_declared_type_name(missing_type_message)?;
        let fixed_length = if type_upper == "STRING" {
            self.parse_fixed_length_string(fixed_length_message)?
        } else {
            None
        };
        Ok((type_upper, fixed_length))
    }

    fn parse_fixed_length_string(&mut self, error_message: &str) -> QResult<Option<usize>> {
        if !self.match_token(&[Token::Multiply]) {
            return Ok(None);
        }

        let length_expr = self.parse_expression()?;
        match length_expr {
            Expression::Literal(core_types::QType::Integer(len)) if len >= 0 => {
                Ok(Some(len as usize))
            }
            Expression::Literal(core_types::QType::Long(len)) if len >= 0 => Ok(Some(len as usize)),
            Expression::Literal(core_types::QType::Single(len)) if len >= 0.0 => {
                Ok(Some(len as usize))
            }
            Expression::Literal(core_types::QType::Double(len)) if len >= 0.0 => {
                Ok(Some(len as usize))
            }
            _ => Err(QError::Syntax(error_message.to_string())),
        }
    }

    fn type_name_to_suffix(type_name: &str) -> Option<char> {
        match type_name {
            "INTEGER" => Some('%'),
            "LONG" => Some('&'),
            "SINGLE" => Some('!'),
            "DOUBLE" => Some('#'),
            "STRING" => Some('$'),
            _ => None,
        }
    }

    fn type_name_to_qtype(type_name: &str) -> QType {
        match type_name {
            "INTEGER" => QType::Integer(0),
            "LONG" => QType::Long(0),
            "SINGLE" => QType::Single(0.0),
            "DOUBLE" => QType::Double(0.0),
            "STRING" => QType::String(String::new()),
            "_BYTE" | "BYTE" | "_BIT" | "BIT" => QType::Integer(0),
            "_FLOAT" | "_INTEGER64" | "INTEGER64" | "_OFFSET" | "OFFSET" => QType::Double(0.0),
            "_UNSIGNED _BYTE" | "UNSIGNED _BYTE" | "_UNSIGNED BYTE" | "UNSIGNED BYTE"
            | "_UNSIGNED _BIT" | "UNSIGNED _BIT" | "_UNSIGNED BIT" | "UNSIGNED BIT"
            | "_UNSIGNED INTEGER" | "UNSIGNED INTEGER" => QType::Integer(0),
            "_UNSIGNED LONG"
            | "UNSIGNED LONG"
            | "_UNSIGNED _INTEGER64"
            | "UNSIGNED _INTEGER64"
            | "_UNSIGNED INTEGER64"
            | "UNSIGNED INTEGER64"
            | "_UNSIGNED _OFFSET"
            | "UNSIGNED _OFFSET"
            | "_UNSIGNED OFFSET"
            | "UNSIGNED OFFSET" => QType::Double(0.0),
            _ => QType::UserDefined(type_name.as_bytes().to_vec()),
        }
    }

    fn function_name_to_qtype(name: &str) -> Option<QType> {
        if name.ends_with('%') {
            Some(QType::Integer(0))
        } else if name.ends_with('&') {
            Some(QType::Long(0))
        } else if name.ends_with('!') {
            Some(QType::Single(0.0))
        } else if name.ends_with('#') {
            Some(QType::Double(0.0))
        } else if name.ends_with('$') {
            Some(QType::String(String::new()))
        } else {
            None
        }
    }

    fn default_function_return_type(name: &str) -> QType {
        Self::function_name_to_qtype(name).unwrap_or(QType::Single(0.0))
    }

    fn parse_declared_type_annotation(
        &mut self,
    ) -> QResult<(Option<char>, Option<String>, Option<usize>)> {
        if !self.match_keyword(Keyword::As) {
            return Ok((None, None, None));
        }

        let (type_upper, fixed_length) = self.parse_declared_type_spec(
            "Expected type name after AS",
            "Fixed-length STRING requires a constant length",
        )?;
        let suffix = Self::type_name_to_suffix(&type_upper);

        Ok((
            suffix,
            if suffix.is_none() {
                Some(type_upper)
            } else {
                None
            },
            fixed_length,
        ))
    }

    fn parse_parameter_list(&mut self) -> QResult<Vec<Variable>> {
        let mut params = Vec::new();
        if self.match_token(&[Token::OpenParen]) {
            if !self.check(&Token::CloseParen) {
                loop {
                    let by_val = if self.match_keyword(Keyword::ByVal) {
                        true
                    } else {
                        self.match_keyword(Keyword::ByRef);
                        false
                    };
                    let Some(Token::Identifier(param_name)) = self.advance() else {
                        return Err(QError::Syntax(
                            "Expected parameter name in procedure signature".to_string(),
                        ));
                    };
                    let param_name = param_name.clone();
                    let indices = if self.match_token(&[Token::OpenParen]) {
                        let mut indices = Vec::new();
                        if !self.check(&Token::CloseParen) {
                            loop {
                                indices.push(self.parse_expression()?);
                                if !self.match_token(&[Token::Comma]) {
                                    break;
                                }
                            }
                        }
                        self.consume(Token::CloseParen, "Expected ')' after parameter dimensions")?;
                        indices
                    } else {
                        Vec::new()
                    };
                    let (type_suffix, declared_type, fixed_length) =
                        self.parse_declared_type_annotation()?;
                    let type_suffix =
                        type_suffix.or_else(|| Variable::suffix_from_name(&param_name));
                    params.push(Variable {
                        name: param_name,
                        by_val,
                        type_suffix,
                        declared_type,
                        fixed_length,
                        indices,
                    });

                    if !self.match_token(&[Token::Comma]) {
                        break;
                    }
                }
            }
            self.consume(Token::CloseParen, "Expected ')' after parameters")?;
        }

        Ok(params)
    }

    fn parse_function_return_annotation(&mut self, name: &str) -> QResult<(QType, Option<usize>)> {
        if !self.match_keyword(Keyword::As) {
            return Ok((Self::default_function_return_type(name), None));
        }

        let (type_upper, fixed_length) = self.parse_declared_type_spec(
            "Expected return type name after AS",
            "Fixed-length STRING return requires a constant length",
        )?;

        Ok((Self::type_name_to_qtype(&type_upper), fixed_length))
    }

    pub fn parse(&mut self) -> QResult<Program> {
        let mut program = Program::new();
        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() {
                break;
            }

            // Handle SUB definition
            if self.check_keyword(Keyword::Sub) {
                let sub_def = self.parse_sub_definition()?;
                program.subs.insert(sub_def.name.clone(), sub_def);
                continue;
            }

            // Handle FUNCTION definition
            if self.check_keyword(Keyword::Function) {
                let func_def = self.parse_function_definition()?;
                self.known_functions.insert(func_def.name.to_uppercase());
                program.functions.insert(func_def.name.clone(), func_def);
                continue;
            }

            // Handle TYPE definition
            if self.check_keyword(Keyword::Type) {
                let type_def = self.parse_type_definition()?;
                self.collected_user_types
                    .insert(type_def.name.clone(), type_def);
                continue;
            }

            if self.check_keyword(Keyword::Declare) && self.is_declare_library_block_start() {
                for stmt in self.parse_declare_library_block()? {
                    if let Statement::Declare {
                        name,
                        is_function: true,
                        ..
                    } = &stmt
                    {
                        self.known_functions.insert(name.to_uppercase());
                    }
                    program.statements.push(stmt);
                }
                continue;
            }

            if let Some(stmt) = self.parse_statement()? {
                program.statements.push(stmt);
            }
        }
        program.user_types.extend(self.collected_user_types.clone());
        Ok(program)
    }

    fn is_declare_library_block_start(&self) -> bool {
        if !self.check_keyword(Keyword::Declare) {
            return false;
        }

        let next = self.peek_next();
        match next {
            Some(Token::Identifier(name)) if name.eq_ignore_ascii_case("LIBRARY") => true,
            Some(Token::Identifier(name))
                if name.eq_ignore_ascii_case("DYNAMIC")
                    || name.eq_ignore_ascii_case("CUSTOMTYPE") =>
            {
                matches!(
                    self.tokens.get(self.current + 2).map(|token| &token.token),
                    Some(Token::Identifier(name)) if name.eq_ignore_ascii_case("LIBRARY")
                )
            }
            _ => false,
        }
    }

    fn parse_declare_library_block(&mut self) -> QResult<Vec<Statement>> {
        self.consume_keyword(Keyword::Declare, "Expected DECLARE")?;

        let _ = self.match_identifier_ci("DYNAMIC") || self.match_identifier_ci("CUSTOMTYPE");
        if !self.match_identifier_ci("LIBRARY") {
            return Err(self.syntax_error_here("Expected LIBRARY after DECLARE"));
        }

        if matches!(self.peek(), Some(Token::StringLiteral(_))) {
            self.advance();
        }

        self.skip_newlines();
        let mut declarations = Vec::new();

        while !self.is_at_end() {
            self.skip_newlines();

            if self.check_keyword(Keyword::End)
                && matches!(self.peek_next(), Some(Token::Keyword(Keyword::Declare)))
            {
                self.advance();
                self.advance();
                break;
            }

            if self.match_keyword(Keyword::Function) {
                declarations.push(self.parse_declare_signature(true)?);
                continue;
            }

            if self.match_keyword(Keyword::Sub) {
                declarations.push(self.parse_declare_signature(false)?);
                continue;
            }

            return Err(self.syntax_error_here(
                "Expected SUB/FUNCTION declaration or END DECLARE in DECLARE LIBRARY block",
            ));
        }

        Ok(declarations)
    }

    fn parse_statement(&mut self) -> QResult<Option<Statement>> {
        if let Some(stmt) = self.pending_statements.pop_front() {
            return Ok(Some(stmt));
        }

        if self.check_keyword(Keyword::Type) {
            let type_def = self.parse_type_definition()?;
            self.collected_user_types
                .insert(type_def.name.clone(), type_def);
            return Ok(None);
        }

        if self.is_declare_library_block_start() {
            let mut declarations = self.parse_declare_library_block()?;
            for stmt in &declarations {
                if let Statement::Declare {
                    name,
                    is_function: true,
                    ..
                } = stmt
                {
                    self.known_functions.insert(name.to_uppercase());
                }
            }
            if declarations.is_empty() {
                return Ok(None);
            }
            let first = declarations.remove(0);
            self.pending_statements.extend(declarations);
            return Ok(Some(first));
        }

        if let Some(Token::IntegerLiteral(line)) = self.peek() {
            let line = *line as u16;
            self.advance();
            return Ok(Some(Statement::LineNumber { number: line }));
        }

        // Handle DEFINT, DEFSNG, DEFDBL, DEFSTR, DEFLNG
        if self.match_keyword(Keyword::DefInt) {
            return self.parse_deftype("INTEGER").map(Some);
        }
        if self.match_keyword(Keyword::DefSng) {
            return self.parse_deftype("SINGLE").map(Some);
        }
        if self.match_keyword(Keyword::DefDbl) {
            return self.parse_deftype("DOUBLE").map(Some);
        }
        if self.match_keyword(Keyword::DefStr) {
            return self.parse_deftype("STRING").map(Some);
        }
        if self.match_keyword(Keyword::DefLng) {
            return self.parse_deftype("LONG").map(Some);
        }

        // Handle OPTION BASE (can be OPTIONBASE or OPTION BASE)
        if self.check_keyword(Keyword::OptionBase) && !self.keyword_is_used_as_identifier() {
            self.advance();
            // Already consumed OPTIONBASE, now expect the number
            if let Some(Token::IntegerLiteral(base)) = self.advance() {
                return Ok(Some(Statement::OptionBase { base: *base }));
            }
            return Err(QError::Syntax(
                "Expected number after OPTION BASE".to_string(),
            ));
        }

        // Handle OPTION followed by BASE
        if self.check_keyword(Keyword::Option) && !self.keyword_is_used_as_identifier() {
            self.advance();
            if self.match_keyword(Keyword::Base) {
                if let Some(Token::IntegerLiteral(base)) = self.advance() {
                    return Ok(Some(Statement::OptionBase { base: *base }));
                }
                return Err(QError::Syntax(
                    "Expected number after OPTION BASE".to_string(),
                ));
            }
            if self.match_identifier_ci("_EXPLICIT") || self.match_identifier_ci("EXPLICIT") {
                return Ok(Some(Statement::Print {
                    expressions: Vec::new(),
                    separators: Vec::new(),
                    newline: false,
                }));
            }
            if self.match_identifier_ci("_EXPLICITARRAY")
                || self.match_identifier_ci("EXPLICITARRAY")
            {
                return Ok(Some(Statement::Print {
                    expressions: Vec::new(),
                    separators: Vec::new(),
                    newline: false,
                }));
            }
            return Err(
                self.syntax_error_here("Expected BASE, _EXPLICIT, or _EXPLICITARRAY after OPTION")
            );
        }

        // Handle DECLARE
        if self.match_keyword(Keyword::Declare) {
            return self.parse_declare().map(Some);
        }

        // Handle CONST
        if self.match_keyword(Keyword::Const) {
            return self.parse_const().map(Some);
        }

        // Handle SWAP
        if self.match_keyword(Keyword::Swap) {
            return self.parse_swap().map(Some);
        }

        // Handle SLEEP
        if self.match_keyword(Keyword::Sleep) {
            let duration = if !self.check(&Token::Newline)
                && !self.check(&Token::Colon)
                && !self.is_at_end()
            {
                Some(self.parse_expression()?)
            } else {
                None
            };
            return Ok(Some(Statement::Sleep { duration }));
        }

        // Handle SYSTEM
        if self.match_keyword(Keyword::System) {
            return Ok(Some(Statement::System));
        }

        // Handle KILL
        if self.match_keyword(Keyword::Kill) {
            let filename = self.parse_expression()?;
            return Ok(Some(Statement::Kill { filename }));
        }

        // Handle NAME
        if self.match_keyword(Keyword::Name) {
            let old_name = self.parse_expression()?;
            self.consume_keyword(Keyword::As, "Expected AS in NAME statement")?;
            let new_name = self.parse_expression()?;
            return Ok(Some(Statement::NameFile { old_name, new_name }));
        }

        // Handle FILES
        if self.match_keyword(Keyword::Files) {
            let pattern = if !self.check(&Token::Newline)
                && !self.check(&Token::Colon)
                && !self.is_at_end()
            {
                Some(self.parse_expression()?)
            } else {
                None
            };
            return Ok(Some(Statement::Files { pattern }));
        }

        // Handle CHDIR
        if self.match_keyword(Keyword::ChDir) {
            let path = self.parse_expression()?;
            return Ok(Some(Statement::ChDir { path }));
        }

        // Handle MKDIR
        if self.match_keyword(Keyword::MkDir) {
            let path = self.parse_expression()?;
            return Ok(Some(Statement::MkDir { path }));
        }

        // Handle RMDIR
        if self.match_keyword(Keyword::RmDir) {
            let path = self.parse_expression()?;
            return Ok(Some(Statement::RmDir { path }));
        }

        // Handle FIELD
        if self.match_keyword(Keyword::Field) {
            return self.parse_field().map(Some);
        }

        // Handle LSET
        if self.match_keyword(Keyword::LSet) {
            return self.parse_lset().map(Some);
        }

        // Handle RSET
        if self.match_keyword(Keyword::RSet) {
            return self.parse_rset().map(Some);
        }

        // Handle RANDOMIZE
        if self.match_keyword(Keyword::Randomize) {
            let seed = if !self.check(&Token::Newline)
                && !self.check(&Token::Colon)
                && !self.is_at_end()
            {
                Some(self.parse_expression()?)
            } else {
                None
            };
            return Ok(Some(Statement::Randomize { seed }));
        }

        // Handle standalone SHARED inside SUB/FUNCTION as a compatibility no-op.
        if self.match_keyword(Keyword::Shared) {
            while !self.check(&Token::Newline) && !self.check(&Token::Colon) && !self.is_at_end() {
                self.advance();
            }
            return Ok(Some(Statement::Print {
                expressions: Vec::new(),
                separators: Vec::new(),
                newline: false,
            }));
        }

        // Handle CLS
        if self.match_keyword(Keyword::Cls) {
            let mode = if !self.check(&Token::Newline)
                && !self.check(&Token::Colon)
                && !self.is_at_end()
            {
                Some(self.parse_expression()?)
            } else {
                None
            };
            return Ok(Some(Statement::Cls { mode }));
        }

        // Handle TRON/TROFF trace-control statements
        if self.match_keyword(Keyword::TrOn) {
            return Ok(Some(Statement::TrOn));
        }
        if self.match_keyword(Keyword::TrOff) {
            return Ok(Some(Statement::TrOff));
        }

        // Handle LOCATE
        if self.match_keyword(Keyword::Locate) {
            return self.parse_locate().map(Some);
        }

        // Handle BEEP
        if self.match_keyword(Keyword::Beep) {
            return Ok(Some(Statement::Beep));
        }

        // Handle SOUND
        if self.match_keyword(Keyword::Sound) {
            let frequency = self.parse_expression()?;
            self.consume(Token::Comma, "Expected comma in SOUND")?;
            let duration = self.parse_expression()?;
            return Ok(Some(Statement::Sound {
                frequency,
                duration,
            }));
        }

        // Handle PLAY
        if self.match_keyword(Keyword::Play) {
            if self.match_keyword(Keyword::On) {
                return Ok(Some(Statement::PlayOn));
            } else if self.match_keyword(Keyword::Off) {
                return Ok(Some(Statement::PlayOff));
            } else if self.match_keyword(Keyword::Stop) {
                return Ok(Some(Statement::PlayStop));
            }
            let melody = self.parse_expression()?;
            return Ok(Some(Statement::Play { melody }));
        }

        // Handle SCREEN
        if self.match_keyword(Keyword::Screen) {
            let mode = if !self.check(&Token::Newline)
                && !self.check(&Token::Comma)
                && !self.check(&Token::Colon)
                && !self.is_at_end()
            {
                Some(self.parse_expression()?)
            } else {
                None
            };
            while self.match_token(&[Token::Comma]) {
                if self.check(&Token::Newline) || self.check(&Token::Colon) || self.is_at_end() {
                    break;
                }
                if self.check(&Token::Comma) {
                    continue;
                }
                let _ = self.parse_expression()?;
            }
            return Ok(Some(Statement::Screen { mode }));
        }

        // Handle COLOR
        if self.match_keyword(Keyword::Color) {
            return self.parse_color().map(Some);
        }

        // Handle WIDTH
        if self.match_keyword(Keyword::Width) {
            return self.parse_width().map(Some);
        }

        // Handle VIEW
        if self.match_keyword(Keyword::View) {
            return self.parse_view().map(Some);
        }

        // Handle WINDOW
        if self.match_keyword(Keyword::Window) {
            return self.parse_window().map(Some);
        }

        // Handle PSET
        if self.match_keyword(Keyword::Pset) {
            return self.parse_pset().map(Some);
        }

        // Handle PRESET
        if self.match_keyword(Keyword::Preset) {
            return self.parse_preset().map(Some);
        }

        // Handle LINE (graphics or LINE INPUT)
        if self.match_keyword(Keyword::Line) {
            // Check if it's LINE INPUT
            if self.check_keyword(Keyword::Input) {
                self.advance(); // consume INPUT
                return self.parse_line_input().map(Some);
            }
            // Otherwise it's LINE graphics
            return self.parse_line().map(Some);
        }

        // Handle CIRCLE
        if self.match_keyword(Keyword::Circle) {
            return self.parse_circle().map(Some);
        }

        // Handle PAINT
        if self.match_keyword(Keyword::Paint) {
            return self.parse_paint().map(Some);
        }

        // Handle POKE
        if self.match_keyword(Keyword::Poke) {
            return self.parse_poke().map(Some);
        }

        // Handle WAIT
        if self.match_keyword(Keyword::Wait) {
            return self.parse_wait().map(Some);
        }

        // Handle BLOAD
        if self.match_keyword(Keyword::BLoad) {
            return self.parse_bload().map(Some);
        }

        // Handle BSAVE
        if self.match_keyword(Keyword::BSave) {
            return self.parse_bsave().map(Some);
        }

        // Handle OUT
        if self.match_keyword(Keyword::Out) {
            return self.parse_out().map(Some);
        }

        // Handle DRAW
        if self.match_keyword(Keyword::Draw) {
            let commands = self.parse_expression()?;
            return Ok(Some(Statement::Draw { commands }));
        }

        // Handle PALETTE
        if self.match_keyword(Keyword::Palette) {
            return self.parse_palette().map(Some);
        }

        // Handle KEY
        if self.match_keyword(Keyword::Key) {
            return self.parse_key().map(Some);
        }

        // Handle ERROR (trigger error)
        if self.match_keyword(Keyword::Error) {
            let error_code = self.parse_expression()?;
            return Ok(Some(Statement::Error { code: error_code }));
        }

        // Handle RESUME
        if self.match_keyword(Keyword::Resume) {
            if self.match_keyword(Keyword::Next) {
                return Ok(Some(Statement::ResumeNext));
            } else if let Some(Token::Identifier(label)) = self.peek() {
                let label = label.clone();
                self.advance();
                return Ok(Some(Statement::ResumeLabel { label }));
            } else {
                return Ok(Some(Statement::Resume));
            }
        }

        // Handle ON (event handling)
        if self.check_keyword(Keyword::On) {
            // Check if it's ON ERROR or ON...GOTO/GOSUB
            if let Some(Token::Keyword(Keyword::Error)) = self.peek_next() {
                self.advance(); // consume ON
                self.advance(); // consume ERROR
                return self.parse_on_error().map(Some);
            } else if let Some(Token::Keyword(Keyword::Timer)) = self.peek_next() {
                return self.parse_on_timer().map(Some);
            } else if let Some(Token::Keyword(Keyword::Play)) = self.peek_next() {
                return self.parse_on_play().map(Some);
            }
            // Could be ON...GOTO or ON...GOSUB
            return self.parse_on_goto_gosub().map(Some);
        }

        // Handle TIMER
        if self.match_keyword(Keyword::Timer) {
            if self.match_keyword(Keyword::On) {
                return Ok(Some(Statement::TimerOn));
            } else if self.match_keyword(Keyword::Off) {
                return Ok(Some(Statement::TimerOff));
            } else if self.match_keyword(Keyword::Stop) {
                return Ok(Some(Statement::TimerStop));
            }
            // Otherwise it's a function call, backtrack
            self.current -= 1;
        }

        // Handle OPEN
        if self.match_keyword(Keyword::Open) {
            return self.parse_open().map(Some);
        }

        // Handle CLOSE
        if self.match_keyword(Keyword::Close) {
            return self.parse_close().map(Some);
        }

        // Handle INPUT
        if self.match_keyword(Keyword::Input) {
            return self.parse_input().map(Some);
        }

        // Handle WRITE
        if self.match_keyword(Keyword::Write) {
            return self.parse_write().map(Some);
        }

        // Handle GET
        if self.match_keyword(Keyword::Get) {
            if self.check(&Token::OpenParen) {
                return self.parse_get_image().map(Some);
            }
            return self.parse_get().map(Some);
        }

        // Handle PUT
        if self.match_keyword(Keyword::Put) {
            if self.check(&Token::OpenParen) {
                return self.parse_put_image().map(Some);
            }
            return self.parse_put().map(Some);
        }

        // Handle SEEK
        if self.match_keyword(Keyword::Seek) {
            return self.parse_seek().map(Some);
        }

        // Handle DATA
        if self.match_keyword(Keyword::Data) {
            return self.parse_data().map(Some);
        }

        // Handle READ
        if self.match_keyword(Keyword::Read) {
            return self.parse_read().map(Some);
        }

        // Handle RESTORE
        if self.match_keyword(Keyword::Restore) {
            let label = match self.peek() {
                Some(Token::Identifier(name)) => {
                    let name = name.clone();
                    self.advance();
                    Some(name)
                }
                Some(Token::IntegerLiteral(value)) => {
                    let value = value.to_string();
                    self.advance();
                    Some(value)
                }
                Some(Token::LongLiteral(value)) => {
                    let value = value.to_string();
                    self.advance();
                    Some(value)
                }
                _ => None,
            };
            return Ok(Some(Statement::Restore { label }));
        }

        // Handle ERASE
        if self.match_keyword(Keyword::Erase) {
            return self.parse_erase().map(Some);
        }

        // Handle REDIM
        if self.match_keyword(Keyword::Redim) {
            return self.parse_redim().map(Some);
        }

        // Handle CALL
        if self.match_keyword(Keyword::Call) {
            return self.parse_call().map(Some);
        }

        // Handle EXIT
        if self.match_keyword(Keyword::Exit) {
            if self.match_keyword(Keyword::For) {
                return Ok(Some(Statement::Exit {
                    exit_type: ExitType::For,
                }));
            } else if self.match_keyword(Keyword::Do) {
                return Ok(Some(Statement::Exit {
                    exit_type: ExitType::Do,
                }));
            } else if self.match_keyword(Keyword::While) {
                return Ok(Some(Statement::Exit {
                    exit_type: ExitType::While,
                }));
            } else if self.match_keyword(Keyword::Function) {
                return Ok(Some(Statement::Exit {
                    exit_type: ExitType::Function,
                }));
            } else if self.match_keyword(Keyword::Sub) {
                return Ok(Some(Statement::Exit {
                    exit_type: ExitType::Sub,
                }));
            }
            return Err(QError::Syntax(
                "Expected FOR, DO, FUNCTION, or SUB after EXIT".to_string(),
            ));
        }

        // Handle CLEAR (with optional parameters)
        if self.match_keyword(Keyword::Clear) {
            // CLEAR can have optional parameters: CLEAR [, [, stack_size]]
            // Skip all parameters including empty ones
            while self.match_token(&[Token::Comma]) {
                // Check if next token is NOT a comma (meaning there's an expression)
                if !self.check(&Token::Comma)
                    && !self.check(&Token::Newline)
                    && !self.check(&Token::Colon)
                    && !self.is_at_end()
                {
                    self.parse_expression()?; // Skip parameter
                }
            }
            return Ok(Some(Statement::Clear));
        }

        // Handle CHAIN
        if self.match_keyword(Keyword::Chain) {
            let filename = self.parse_expression()?;
            let delete = if self.match_token(&[Token::Comma]) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            return Ok(Some(Statement::Chain { filename, delete }));
        }

        // Handle SHELL
        if self.match_keyword(Keyword::Shell) {
            while self.match_identifier_ci("_HIDE") || self.match_identifier_ci("_DONTWAIT") {}
            let command = if !self.check(&Token::Newline)
                && !self.check(&Token::Colon)
                && !self.is_at_end()
            {
                Some(self.parse_expression()?)
            } else {
                None
            };
            return Ok(Some(Statement::Shell { command }));
        }

        // Handle COMMON SHARED (must come before DIM)
        if self.match_keyword(Keyword::Common) {
            let is_shared = self.match_keyword(Keyword::Shared);
            return self.parse_common(is_shared).map(Some);
        }

        if self.match_keyword(Keyword::Print) {
            return self.parse_print(false).map(Some);
        }
        if self.match_keyword(Keyword::LPrint) {
            return self.parse_print(true).map(Some);
        }
        if self.match_keyword(Keyword::Dim) {
            return self.parse_dim().map(Some);
        }
        if self.match_keyword(Keyword::For) {
            return self.parse_for().map(Some);
        }
        if self.match_keyword(Keyword::Next) {
            // Should be handled by for, but if dangling
            return Err(QError::Syntax("NEXT without FOR".to_string()));
        }
        if self.match_keyword(Keyword::While) {
            return self.parse_while().map(Some);
        }
        if self.match_keyword(Keyword::Wend) {
            return Err(QError::Syntax("WEND without WHILE".to_string()));
        }
        if self.match_keyword(Keyword::Do) {
            return self.parse_do().map(Some);
        }
        if self.match_keyword(Keyword::Select) {
            return self.parse_select().map(Some);
        }
        if self.match_keyword(Keyword::If) {
            return self.parse_if().map(Some);
        }
        if self.match_keyword(Keyword::Goto) {
            return self.parse_goto().map(Some);
        }
        if self.match_keyword(Keyword::Gosub) {
            return self.parse_gosub().map(Some);
        }
        if self.match_keyword(Keyword::Return) {
            return Ok(Some(Statement::Return));
        }
        if self.match_keyword(Keyword::End) {
            return Ok(Some(Statement::End));
        }

        // Handle LET (optional keyword for assignment)
        if self.match_keyword(Keyword::Let) {
            return self.parse_assignment().map(Some);
        }

        // Handle DEF FN (inline function definition) and DEF SEG
        if self.match_keyword(Keyword::Def) {
            // Check for DEF SEG
            if self.match_keyword(Keyword::Seg) {
                // DEF SEG [= expression]
                let segment = if self.match_token(&[Token::Equal]) {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
                return Ok(Some(Statement::DefSeg { segment }));
            }

            // Otherwise, it should be DEF FN<name>
            if let Some(Token::Identifier(fn_name)) = self.peek() {
                let fn_name_str = fn_name.clone();

                // Check if the identifier starts with "FN"
                if fn_name_str.to_uppercase().starts_with("FN") {
                    self.advance(); // consume the FN<name> identifier

                    // Parse parameters (optional parentheses)
                    let mut params = Vec::new();
                    if self.check(&Token::OpenParen) {
                        self.advance(); // consume '('
                        if !self.check(&Token::CloseParen) {
                            loop {
                                if let Some(Token::Identifier(param)) = self.peek() {
                                    params.push(param.clone());
                                    self.advance();
                                } else {
                                    return Err(QError::Syntax(
                                        "Expected parameter name in DEF FN".to_string(),
                                    ));
                                }
                                if !self.match_token(&[Token::Comma]) {
                                    break;
                                }
                            }
                        }
                        self.consume(Token::CloseParen, "Expected ')' after DEF FN parameters")?;
                    }

                    self.consume(Token::Equal, "Expected '=' in DEF FN")?;
                    let body = self.parse_expression()?;

                    return Ok(Some(Statement::DefFn {
                        name: fn_name_str,
                        params,
                        body,
                    }));
                }
            }

            return Err(QError::Syntax(
                "Expected FN<name> or SEG after DEF".to_string(),
            ));
        }

        if self.check_keyword(Keyword::Asc) {
            return self.parse_assignment().map(Some);
        }

        // Check for Label vs Assignment / implicit SUB call
        if let Some(Token::Identifier(name)) = self.peek() {
            let name_clone = name.clone();
            if name_clone.starts_with('$') {
                self.parse_metacommand();
                return Ok(None);
            }
            if let Some(Token::Colon) = self.peek_next() {
                self.advance(); // consume ident
                self.advance(); // consume colon
                return Ok(Some(Statement::Label { name: name_clone }));
            }
            return self.parse_identifier_statement().map(Some);
        }

        if matches!(self.peek(), Some(Token::Keyword(_))) {
            return self.parse_keyword_identifier_statement().map(Some);
        }

        Err(self.syntax_error_here("Unexpected token in statement parsing"))
    }

    fn parse_print(&mut self, printer: bool) -> QResult<Statement> {
        // Check for file output: PRINT #filenum, ...
        let file_number = if self.match_token(&[Token::Hash]) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        // If file output, expect comma
        if file_number.is_some() {
            self.consume(Token::Comma, "Expected ',' after file number in PRINT")?;
        }

        let mut using_format = None;
        if self.match_keyword(Keyword::Using) {
            using_format = Some(self.parse_expression()?);
            self.consume(Token::Semicolon, "Expected ';' after USING format string")?;
        }

        let mut expressions = Vec::new();
        let mut separators = Vec::new();
        let mut newline = true;

        if self.check(&Token::Newline) || self.check(&Token::Colon) || self.is_at_end() {
            if let Some(format) = using_format {
                if let Some(fnum) = file_number {
                    return Ok(Statement::PrintFileUsing {
                        file_number: fnum,
                        format,
                        expressions,
                        separators,
                        newline,
                    });
                }
                return Ok(if printer {
                    Statement::LPrintUsing {
                        format,
                        expressions,
                        separators,
                        newline,
                    }
                } else {
                    Statement::PrintUsing {
                        format,
                        expressions,
                        separators,
                        newline,
                    }
                });
            }
            if let Some(fnum) = file_number {
                return Ok(Statement::PrintFile {
                    file_number: fnum,
                    expressions,
                    separators,
                    newline,
                });
            }
            return Ok(if printer {
                Statement::LPrint {
                    expressions,
                    separators,
                    newline,
                }
            } else {
                Statement::Print {
                    expressions,
                    separators,
                    newline,
                }
            });
        }

        loop {
            expressions.push(self.parse_expression()?);
            if self.match_token(&[Token::Semicolon]) {
                separators.push(Some(PrintSeparator::Semicolon));
                newline = false;
                if self.check(&Token::Newline) || self.check(&Token::Colon) || self.is_at_end() {
                    break;
                }
            } else if self.match_token(&[Token::Comma]) {
                separators.push(Some(PrintSeparator::Comma));
                newline = false; // Simplified, normally advances to next print zone
                if self.check(&Token::Newline) || self.check(&Token::Colon) || self.is_at_end() {
                    break;
                }
            } else {
                separators.push(None);
                newline = true;
                break;
            }
        }

        if let Some(format) = using_format {
            if let Some(fnum) = file_number {
                Ok(Statement::PrintFileUsing {
                    file_number: fnum,
                    format,
                    expressions,
                    separators,
                    newline,
                })
            } else if printer {
                Ok(Statement::LPrintUsing {
                    format,
                    expressions,
                    separators,
                    newline,
                })
            } else {
                Ok(Statement::PrintUsing {
                    format,
                    expressions,
                    separators,
                    newline,
                })
            }
        } else if let Some(fnum) = file_number {
            Ok(Statement::PrintFile {
                file_number: fnum,
                expressions,
                separators,
                newline,
            })
        } else {
            Ok(if printer {
                Statement::LPrint {
                    expressions,
                    separators,
                    newline,
                }
            } else {
                Statement::Print {
                    expressions,
                    separators,
                    newline,
                }
            })
        }
    }

    fn parse_array_dimensions(&mut self) -> QResult<Option<Vec<ArrayDimension>>> {
        if !self.match_token(&[Token::OpenParen]) {
            return Ok(None);
        }

        let mut dimensions = Vec::new();
        loop {
            let first_expr = self.parse_expression()?;
            let dimension = if self.match_keyword(Keyword::To) {
                let upper_bound = self.parse_expression()?;
                ArrayDimension::explicit(first_expr, upper_bound)
            } else {
                ArrayDimension::implicit(first_expr)
            };
            dimensions.push(dimension);

            if !self.match_token(&[Token::Comma]) {
                break;
            }
        }

        self.consume(Token::CloseParen, "Expected ')' after array dimensions")?;
        Ok(Some(dimensions))
    }

    fn parse_dim(&mut self) -> QResult<Statement> {
        // Check for SHARED keyword
        let is_shared = self.match_keyword(Keyword::Shared);
        let variables = self.parse_variable_declarations("DIM")?;
        Ok(Statement::Dim {
            variables,
            is_static: false,
            is_shared,
            is_common: false,
        })
    }

    fn parse_common(&mut self, is_shared: bool) -> QResult<Statement> {
        // COMMON SHARED is similar to DIM SHARED but with is_common flag
        let variables = self.parse_variable_declarations("COMMON")?;
        Ok(Statement::Dim {
            variables,
            is_static: false,
            is_shared,
            is_common: true,
        })
    }

    fn parse_assignment(&mut self) -> QResult<Statement> {
        // Assignment targets can start with identifiers or assignable built-in forms like ASC(...)
        let target = self.parse_primary()?;
        self.consume(Token::Equal, "Expected '=' in assignment")?;
        let value = self.parse_expression()?;
        Ok(Statement::Assignment { target, value })
    }

    fn parse_identifier_statement(&mut self) -> QResult<Statement> {
        let target = self.parse_primary()?;
        self.statement_from_primary_target(target)
    }

    fn parse_keyword_identifier_statement(&mut self) -> QResult<Statement> {
        let target = self.parse_primary()?;
        self.statement_from_primary_target(target)
    }

    fn statement_from_primary_target(&mut self, target: Expression) -> QResult<Statement> {
        if self.match_token(&[Token::Equal]) {
            let value = self.parse_expression()?;
            return Ok(Statement::Assignment { target, value });
        }

        match target {
            Expression::Variable(var) => {
                let mut args = Vec::new();
                while !self.check(&Token::Newline)
                    && !self.check(&Token::Colon)
                    && !self.is_at_end()
                {
                    args.push(self.parse_expression()?);
                    if !self.match_token(&[Token::Comma]) {
                        break;
                    }
                }
                Ok(Statement::Call {
                    name: var.name,
                    args,
                })
            }
            Expression::FunctionCall(mut func) => {
                while self.match_token(&[Token::Comma]) {
                    func.args.push(self.parse_expression()?);
                }
                Ok(Statement::Call {
                    name: func.name,
                    args: func.args,
                })
            }
            Expression::ArrayAccess {
                name, mut indices, ..
            } => {
                while self.match_token(&[Token::Comma]) {
                    indices.push(self.parse_expression()?);
                }
                Ok(Statement::Call {
                    name,
                    args: indices,
                })
            }
            _ => Err(QError::Syntax(
                "Expected '=' in assignment or a subroutine call".to_string(),
            )),
        }
    }

    fn parse_inline_statement_sequence(&mut self) -> QResult<Vec<Statement>> {
        let mut statements = Vec::new();
        loop {
            while self.match_token(&[Token::Colon]) {}

            if self.check(&Token::Newline) || self.is_at_end() || self.check_keyword(Keyword::Else)
            {
                break;
            }

            if let Some(stmt) = self.parse_statement()? {
                statements.push(stmt);
            }

            if !self.check(&Token::Colon) {
                break;
            }
        }

        Ok(statements)
    }

    fn parse_for(&mut self) -> QResult<Statement> {
        if matches!(self.peek(), Some(Token::Identifier(each)) if each.eq_ignore_ascii_case("EACH"))
        {
            self.advance();
            let name = if let Some(Token::Identifier(n)) = self.advance() {
                n.clone()
            } else {
                return Err(QError::Syntax(
                    "Expected loop variable after FOR EACH".to_string(),
                ));
            };
            let variable = Variable::new(name);
            match self.advance() {
                Some(Token::Identifier(in_kw)) if in_kw.eq_ignore_ascii_case("IN") => {}
                other => {
                    return Err(QError::Syntax(format!(
                        "Expected IN in FOR EACH loop, found {:?}",
                        other
                    )))
                }
            }
            let array = self.parse_expression()?;

            self.skip_newlines();
            let mut body = Vec::new();
            while !self.check_keyword(Keyword::Next) && !self.is_at_end() {
                if let Some(stmt) = self.parse_statement()? {
                    body.push(stmt);
                }
                self.skip_newlines();
            }
            self.consume_keyword(Keyword::Next, "Expected NEXT")?;
            if let Some(Token::Identifier(_)) = self.peek() {
                self.advance();
            }

            return Ok(Statement::ForEach {
                variable,
                array,
                body,
            });
        }

        let name = if let Some(Token::Identifier(n)) = self.advance() {
            n.clone()
        } else {
            return Err(QError::Syntax("Expected variable after FOR".to_string()));
        };
        let variable = Variable::new(name);
        self.consume(Token::Equal, "Expected '=' in FOR loop")?;
        let start = self.parse_expression()?;
        self.consume_keyword(Keyword::To, "Expected TO in FOR loop")?;
        let end = self.parse_expression()?;

        let mut step = None;
        if self.match_keyword(Keyword::Step) {
            step = Some(self.parse_expression()?);
        }

        self.skip_newlines();
        let mut body = Vec::new();
        while !self.check_keyword(Keyword::Next) && !self.is_at_end() {
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
            }
            self.skip_newlines();
        }
        self.consume_keyword(Keyword::Next, "Expected NEXT")?;
        // Optional var name after NEXT
        if let Some(Token::Identifier(_)) = self.peek() {
            self.advance();
        }

        Ok(Statement::ForLoop {
            variable,
            start,
            end,
            step,
            body,
        })
    }

    fn parse_while(&mut self) -> QResult<Statement> {
        let condition = self.parse_expression()?;
        self.skip_newlines();
        let mut body = Vec::new();
        while !self.check_keyword(Keyword::Wend) && !self.is_at_end() {
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
            }
            self.skip_newlines();
        }
        self.consume_keyword(Keyword::Wend, "Expected WEND")?;
        Ok(Statement::WhileLoop { condition, body })
    }

    fn parse_do(&mut self) -> QResult<Statement> {
        let mut condition = None;
        let mut pre_condition = false;

        if self.match_keyword(Keyword::While) {
            let expr = self.parse_expression()?;
            condition = Some(Expression::UnaryOp {
                op: UnaryOp::Not,
                operand: Box::new(expr),
            });
            pre_condition = true;
        } else if self.match_keyword(Keyword::Until) {
            condition = Some(self.parse_expression()?);
            pre_condition = true;
        }

        self.skip_newlines();
        let mut body = Vec::new();
        while !self.check_keyword(Keyword::Loop) && !self.is_at_end() {
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
            }
            self.skip_newlines();
        }
        self.consume_keyword(Keyword::Loop, "Expected LOOP")?;

        if condition.is_none() {
            if self.match_keyword(Keyword::While) {
                let expr = self.parse_expression()?;
                condition = Some(Expression::UnaryOp {
                    op: UnaryOp::Not,
                    operand: Box::new(expr),
                });
            } else if self.match_keyword(Keyword::Until) {
                condition = Some(self.parse_expression()?);
            }
        }

        Ok(Statement::DoLoop {
            condition,
            body,
            pre_condition,
        })
    }

    fn parse_select(&mut self) -> QResult<Statement> {
        if !self.match_keyword(Keyword::Case) && !self.match_identifier_ci("EVERYCASE") {
            return Err(self.syntax_error_here("Expected CASE after SELECT"));
        }
        let expression = self.parse_expression()?;
        self.skip_newlines();

        let mut cases = Vec::new();

        while !self.check_keyword(Keyword::EndSelect) && !self.is_at_end() {
            self.skip_newlines();
            if self.match_keyword(Keyword::Case) {
                if self.match_keyword(Keyword::Else) {
                    let mut body = Vec::new();
                    self.skip_newlines();
                    while !self.check_keyword(Keyword::EndSelect) && !self.is_at_end() {
                        if let Some(stmt) = self.parse_statement()? {
                            body.push(stmt);
                        }
                        self.skip_newlines();
                    }
                    cases.push((Expression::CaseElse, body));
                    break;
                } else {
                    // Parse first case value
                    let case_expr = if self.match_keyword(Keyword::Is) {
                        let op = if self.match_token(&[Token::Equal]) {
                            BinaryOp::Equal
                        } else if self.match_token(&[Token::NotEqual]) {
                            BinaryOp::NotEqual
                        } else if self.match_token(&[Token::LessThan]) {
                            BinaryOp::LessThan
                        } else if self.match_token(&[Token::GreaterThan]) {
                            BinaryOp::GreaterThan
                        } else if self.match_token(&[Token::LessOrEqual]) {
                            BinaryOp::LessOrEqual
                        } else if self.match_token(&[Token::GreaterOrEqual]) {
                            BinaryOp::GreaterOrEqual
                        } else {
                            return Err(QError::Syntax(
                                "Expected comparison operator after CASE IS".to_string(),
                            ));
                        };
                        let val = self.parse_expression()?;
                        Expression::CaseIs {
                            op,
                            value: Box::new(val),
                        }
                    } else {
                        let expr1 = self.parse_expression()?;
                        if self.match_keyword(Keyword::To) {
                            let expr2 = self.parse_expression()?;
                            Expression::CaseRange {
                                start: Box::new(expr1),
                                end: Box::new(expr2),
                            }
                        } else {
                            expr1
                        }
                    };

                    // Skip any additional comma-separated values (simplified: we only use the first one)
                    while self.match_token(&[Token::Comma]) {
                        // Parse and discard additional values
                        if self.match_keyword(Keyword::Is) {
                            // Skip IS comparison
                            if self.match_token(&[
                                Token::Equal,
                                Token::NotEqual,
                                Token::LessThan,
                                Token::GreaterThan,
                                Token::LessOrEqual,
                                Token::GreaterOrEqual,
                            ]) {
                                self.parse_expression()?;
                            }
                        } else {
                            let _expr = self.parse_expression()?;
                            if self.match_keyword(Keyword::To) {
                                self.parse_expression()?;
                            }
                        }
                    }

                    let mut body = Vec::new();
                    self.skip_newlines();
                    while !self.check_keyword(Keyword::Case)
                        && !self.check_keyword(Keyword::EndSelect)
                        && !self.is_at_end()
                    {
                        if let Some(stmt) = self.parse_statement()? {
                            body.push(stmt);
                        }
                        self.skip_newlines();
                    }
                    cases.push((case_expr, body));
                }
            } else {
                // Ignore stray tokens or errors to avoid infinite loops
                let token = self.advance();
                return Err(QError::Syntax(format!(
                    "Unexpected token in SELECT CASE: {:?}",
                    token
                )));
            }
        }

        // Wait, EndSelect might be `END SELECT` Token sequence.
        if self.match_keyword(Keyword::End) {
            self.consume_keyword(Keyword::Select, "Expected SELECT after END")?;
        } else {
            self.consume_keyword(Keyword::EndSelect, "Expected END SELECT")?;
        }

        Ok(Statement::Select { expression, cases })
    }

    fn parse_if(&mut self) -> QResult<Statement> {
        let condition = self.parse_expression()?;
        if !self.match_keyword(Keyword::Then) {
            if self.check_keyword(Keyword::Goto) || self.check_keyword(Keyword::Gosub) {
                let then_stmt = self.parse_statement()?.ok_or_else(|| {
                    self.syntax_error_here("Expected GOTO or GOSUB target after IF condition")
                })?;
                let then_branch = vec![then_stmt];
                let mut else_branch = None;
                if self.match_keyword(Keyword::Else) {
                    let else_stmt = self.parse_statement()?.ok_or_else(|| {
                        self.syntax_error_here("Expected statement after ELSE in IF shorthand")
                    })?;
                    else_branch = Some(vec![else_stmt]);
                }
                return Ok(Statement::IfBlock {
                    condition,
                    then_branch,
                    else_branch,
                });
            }
            return Err(self.syntax_error_here("Expected THEN after IF"));
        }

        if self.check(&Token::Newline) || self.check(&Token::Colon) {
            // Block IF
            self.skip_newlines();
            let mut then_branch = Vec::new();
            while !self.check_keyword(Keyword::ElseIf)
                && !self.check_keyword(Keyword::Else)
                && !self.check_keyword(Keyword::EndIf)
                && !self.is_at_end()
            {
                if let Some(stmt) = self.parse_statement()? {
                    then_branch.push(stmt);
                }
                self.skip_newlines();
            }

            let mut else_ifs = Vec::new();
            while self.match_keyword(Keyword::ElseIf) {
                let cond = self.parse_expression()?;
                self.consume_keyword(Keyword::Then, "Expected THEN after ELSEIF")?;
                self.skip_newlines();
                let mut e_body = Vec::new();
                while !self.check_keyword(Keyword::ElseIf)
                    && !self.check_keyword(Keyword::Else)
                    && !self.check_keyword(Keyword::EndIf)
                    && !self.is_at_end()
                {
                    if let Some(stmt) = self.parse_statement()? {
                        e_body.push(stmt);
                    }
                    self.skip_newlines();
                }
                else_ifs.push((cond, e_body));
            }

            let mut else_branch = None;
            if self.match_keyword(Keyword::Else) {
                self.skip_newlines();
                let mut e_body = Vec::new();
                while !self.check_keyword(Keyword::EndIf) && !self.is_at_end() {
                    if let Some(stmt) = self.parse_statement()? {
                        e_body.push(stmt);
                    }
                    self.skip_newlines();
                }
                else_branch = Some(e_body);
            }

            if self.match_keyword(Keyword::End) {
                self.consume_keyword(Keyword::If, "Expected IF after END")?;
            } else {
                self.consume_keyword(Keyword::EndIf, "Expected END IF")?;
            }

            if else_ifs.is_empty() {
                Ok(Statement::IfBlock {
                    condition,
                    then_branch,
                    else_branch,
                })
            } else {
                Ok(Statement::IfElseBlock {
                    condition,
                    then_branch,
                    else_ifs,
                    else_branch,
                })
            }
        } else {
            // Single line IF
            let then_branch = self.parse_inline_statement_sequence()?;

            let mut else_branch = None;
            if self.match_keyword(Keyword::Else) {
                else_branch = Some(self.parse_inline_statement_sequence()?);
            }

            Ok(Statement::IfBlock {
                condition,
                then_branch,
                else_branch,
            })
        }
    }

    fn parse_goto(&mut self) -> QResult<Statement> {
        let target = self.parse_goto_target()?;
        Ok(Statement::Goto { target })
    }

    fn parse_gosub(&mut self) -> QResult<Statement> {
        let target = self.parse_goto_target()?;
        Ok(Statement::Gosub { target })
    }

    fn parse_goto_target(&mut self) -> QResult<GotoTarget> {
        if let Some(Token::Identifier(label)) = self.advance() {
            Ok(GotoTarget::Label(label.clone()))
        } else if let Some(Token::IntegerLiteral(line)) = self.previous() {
            Ok(GotoTarget::LineNumber(*line as u16))
        } else {
            Err(QError::Syntax("Expected label or line number".to_string()))
        }
    }

    fn parse_expression(&mut self) -> QResult<Expression> {
        self.parse_logical_imp()
    }

    fn parse_logical_imp(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_logical_eqv()?;
        while self.check_keyword(Keyword::Imp) {
            self.advance();
            let right = self.parse_logical_eqv()?;
            expr = Expression::BinaryOp {
                op: BinaryOp::Imp,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_logical_eqv(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_logical_xor()?;
        while self.check_keyword(Keyword::Eqv) {
            self.advance();
            let right = self.parse_logical_xor()?;
            expr = Expression::BinaryOp {
                op: BinaryOp::Eqv,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_logical_xor(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_logical_or()?;
        while self.check_keyword(Keyword::Xor) {
            self.advance();
            let right = self.parse_logical_or()?;
            expr = Expression::BinaryOp {
                op: BinaryOp::Xor,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_logical_and()?;
        while self.match_token(&[Token::Or]) {
            let right = self.parse_logical_and()?;
            expr = Expression::BinaryOp {
                op: BinaryOp::Or,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_equality()?;
        while self.match_token(&[Token::And]) {
            let right = self.parse_equality()?;
            expr = Expression::BinaryOp {
                op: BinaryOp::And,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_comparison()?;
        while self.match_token(&[Token::Equal, Token::NotEqual]) {
            let op = match self.previous().unwrap() {
                Token::Equal => BinaryOp::Equal,
                Token::NotEqual => BinaryOp::NotEqual,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            expr = Expression::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_term()?;
        while self.match_token(&[
            Token::LessThan,
            Token::LessOrEqual,
            Token::GreaterThan,
            Token::GreaterOrEqual,
        ]) {
            let op = match self.previous().unwrap() {
                Token::LessThan => BinaryOp::LessThan,
                Token::LessOrEqual => BinaryOp::LessOrEqual,
                Token::GreaterThan => BinaryOp::GreaterThan,
                Token::GreaterOrEqual => BinaryOp::GreaterOrEqual,
                _ => unreachable!(),
            };
            let right = self.parse_term()?;
            expr = Expression::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_factor()?;
        while self.match_token(&[Token::Plus, Token::Minus]) {
            let op = match self.previous().unwrap() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Subtract,
                _ => unreachable!(),
            };
            let right = self.parse_factor()?;
            expr = Expression::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_power()?;
        while self.match_token(&[
            Token::Multiply,
            Token::Divide,
            Token::IntegerDivide,
            Token::Modulo,
        ]) || self.check_keyword(Keyword::Mod)
        {
            let op = if self.check_keyword(Keyword::Mod) {
                self.advance();
                BinaryOp::Modulo
            } else {
                match self.previous().unwrap() {
                    Token::Multiply => BinaryOp::Multiply,
                    Token::Divide => BinaryOp::Divide,
                    Token::IntegerDivide => BinaryOp::IntegerDivide,
                    Token::Modulo => BinaryOp::Modulo,
                    _ => unreachable!(),
                }
            };
            let right = self.parse_power()?;
            expr = Expression::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_power(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_unary()?;
        // Power operator is right-associative in BASIC
        if self.match_token(&[Token::Power]) {
            let right = self.parse_power()?; // Right-associative recursion
            expr = Expression::BinaryOp {
                op: BinaryOp::Power,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> QResult<Expression> {
        if self.match_token(&[Token::Minus]) {
            let right = self.parse_unary()?;
            return Ok(Expression::UnaryOp {
                op: UnaryOp::Negate,
                operand: Box::new(right),
            });
        }
        if self.match_token(&[Token::Not]) {
            let right = self.parse_unary()?;
            return Ok(Expression::UnaryOp {
                op: UnaryOp::Not,
                operand: Box::new(right),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> QResult<Expression> {
        self.parse_field_access()
    }

    fn parse_cv_type_argument(&mut self) -> QResult<Expression> {
        let (type_name, _) = self.parse_declared_type_spec(
            "Expected type name in _CV",
            "Fixed-length STRING is not supported in _CV type arguments",
        )?;
        Ok(Expression::Variable(Variable::new(type_name)))
    }

    fn parse_field_access(&mut self) -> QResult<Expression> {
        let mut expr = self.parse_base_primary()?;

        // Handle chained field access: obj.field1.field2
        while self.match_token(&[Token::Dot]) {
            let field = match self.peek() {
                Some(Token::Identifier(field_name)) => {
                    let field = field_name.clone();
                    self.advance();
                    field
                }
                Some(Token::Keyword(kw)) => {
                    let field = format!("{:?}", kw);
                    self.advance();
                    field
                }
                _ => return Err(QError::Syntax("Expected field name after '.'".to_string())),
            };
            expr = Expression::FieldAccess {
                object: Box::new(expr),
                field,
            };
        }

        Ok(expr)
    }

    fn parse_base_primary(&mut self) -> QResult<Expression> {
        // Match numbers
        let token = self.advance().unwrap().clone();
        match token {
            Token::IntegerLiteral(v) => Ok(Expression::Literal(QType::Integer(v))),
            Token::LongLiteral(v) => Ok(Expression::Literal(QType::Long(v))),
            Token::SingleLiteral(v) => Ok(Expression::Literal(QType::Single(v))),
            Token::DoubleLiteral(v) => Ok(Expression::Literal(QType::Double(v))),
            Token::StringLiteral(v) => Ok(Expression::Literal(QType::String(v))),
            Token::OpenParen => {
                let expr = self.parse_expression()?;
                self.consume(Token::CloseParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            Token::Identifier(name) => {
                let mut type_suffix = None;
                // Suffix checking is normally handled by tokens natively or we can just parse it
                if name.ends_with('$') {
                    type_suffix = Some('$');
                }
                if name.ends_with('%') {
                    type_suffix = Some('%');
                }
                if name.ends_with('&') {
                    type_suffix = Some('&');
                }
                if name.ends_with('!') {
                    type_suffix = Some('!');
                }
                if name.ends_with('#') {
                    type_suffix = Some('#');
                }

                if self.match_token(&[Token::OpenParen]) {
                    let mut args = Vec::new();
                    if name.eq_ignore_ascii_case("_CV") && !self.check(&Token::CloseParen) {
                        args.push(self.parse_cv_type_argument()?);
                        if self.match_token(&[Token::Comma]) && !self.check(&Token::CloseParen) {
                            loop {
                                args.push(self.parse_argument_expression()?);
                                if !self.match_token(&[Token::Comma]) {
                                    break;
                                }
                            }
                        }
                    } else if !self.check(&Token::CloseParen) {
                        loop {
                            args.push(self.parse_argument_expression()?);
                            if !self.match_token(&[Token::Comma]) {
                                break;
                            }
                        }
                    }
                    self.consume(Token::CloseParen, "Expected ')' after arguments")?;

                    // Treat known user-defined functions and DEF FN names as calls.
                    if name.to_uppercase().starts_with("FN")
                        || self.known_functions.contains(&name.to_uppercase())
                    {
                        Ok(Expression::FunctionCall(FunctionCall {
                            name,
                            args,
                            type_suffix,
                        }))
                    } else {
                        Ok(Expression::ArrayAccess {
                            name,
                            indices: args,
                            type_suffix,
                        })
                    }
                } else {
                    Ok(Expression::Variable(Variable {
                        name,
                        by_val: false,
                        type_suffix,
                        declared_type: None,
                        fixed_length: None,
                        indices: Vec::new(),
                    }))
                }
            }
            Token::Keyword(kw) => {
                // Built-in functions like STR$, LEFT$ are parsed as array access by identifier,
                // BUT due to tokenization they might become keywords!
                // For instance, LEFT$ is Keyword::Left

                // Check for zero-argument functions
                let fn_name = format!("{:?}", kw).to_uppercase();
                let is_zero_arg_func = matches!(
                    fn_name.as_str(),
                    "TIMER"
                        | "RND"
                        | "DATE"
                        | "TIME"
                        | "INKEY"
                        | "CSRLIN"
                        | "FREEFILE"
                        | "COMMAND"
                        | "ERR"
                        | "ERL"
                        | "ERDEV"
                        | "ERDEVSTR"
                );

                if self.match_token(&[Token::OpenParen]) {
                    let mut args = Vec::new();
                    if !self.check(&Token::CloseParen) {
                        loop {
                            args.push(self.parse_argument_expression()?);
                            if !self.match_token(&[Token::Comma]) {
                                break;
                            }
                        }
                    }
                    self.consume(Token::CloseParen, "Expected ')' after arguments")?;

                    // We generate an ArrayAccess which our compiler will turn into FunctionCall
                    Ok(Expression::ArrayAccess {
                        name: fn_name,
                        indices: args,
                        type_suffix: None,
                    })
                } else if is_zero_arg_func {
                    // Zero-argument function - treat as function call with no args
                    Ok(Expression::ArrayAccess {
                        name: fn_name,
                        indices: Vec::new(),
                        type_suffix: None,
                    })
                } else {
                    Ok(Expression::Variable(Variable::new(fn_name)))
                }
            }
            _ => Err(self.syntax_error_here(format!(
                "Unexpected token in expression: {}",
                Self::debug_token(&token)
            ))),
        }
    }

    fn parse_argument_expression(&mut self) -> QResult<Expression> {
        let _ = self.match_token(&[Token::Hash]);
        self.parse_expression()
    }

    // Helper functions for parsing new statements
    fn parse_deftype(&mut self, type_name: &str) -> QResult<Statement> {
        // Parse letter range like A-Z or I-N, or multiple letters like A, S, T
        // Can be either "I-N" as single identifier or "I", "-", "N" as separate tokens
        // Or multiple single letters separated by commas: A, S, T

        let mut ranges = Vec::new();

        loop {
            if let Some(token) = self.advance() {
                match token {
                    Token::Identifier(range) => {
                        // Check if it contains a dash (e.g., "I-N")
                        let parts: Vec<&str> = range.split('-').collect();
                        if parts.len() == 2 {
                            let start = parts[0].chars().next().unwrap_or('A');
                            let end = parts[1].chars().next().unwrap_or('Z');
                            ranges.push((start, end));
                        } else if parts.len() == 1 {
                            // Single letter or start of range
                            let start_ch = parts[0].chars().next().unwrap_or('A');

                            // Check if next token is Minus
                            if self.match_token(&[Token::Minus]) {
                                // Expect end letter
                                if let Some(Token::Identifier(end_id)) = self.advance() {
                                    let end_ch = end_id.chars().next().unwrap_or('Z');
                                    ranges.push((start_ch, end_ch));
                                } else {
                                    return Err(QError::Syntax(
                                        "Expected end letter after '-' in DEF statement"
                                            .to_string(),
                                    ));
                                }
                            } else {
                                // Single letter only
                                ranges.push((start_ch, start_ch));
                            }
                        }
                    }
                    _ => {
                        return Err(QError::Syntax(
                            "Expected letter or letter range in DEF statement".to_string(),
                        ));
                    }
                }
            } else {
                return Err(QError::Syntax(
                    "Expected letter range in DEF statement".to_string(),
                ));
            }

            // Check if there's a comma for more letters
            if !self.match_token(&[Token::Comma]) {
                break;
            }
        }

        if !ranges.is_empty() {
            Ok(Statement::DefType {
                letter_ranges: ranges,
                type_name: type_name.to_string(),
            })
        } else {
            Err(QError::Syntax(
                "Expected at least one letter range in DEF statement".to_string(),
            ))
        }
    }

    fn parse_declare(&mut self) -> QResult<Statement> {
        let is_function = if self.match_keyword(Keyword::Function) {
            true
        } else if self.match_keyword(Keyword::Sub) {
            false
        } else {
            return Err(QError::Syntax(
                "Expected SUB or FUNCTION after DECLARE".to_string(),
            ));
        };

        self.parse_declare_signature(is_function)
    }

    fn parse_declare_signature(&mut self, is_function: bool) -> QResult<Statement> {
        if let Some(Token::Identifier(name)) = self.advance() {
            let name = name.clone();
            if is_function {
                self.known_functions.insert(name.to_uppercase());
            }
            let params = self.parse_parameter_list()?;
            let (return_type, return_fixed_length) = if is_function {
                let (return_type, return_fixed_length) =
                    self.parse_function_return_annotation(&name)?;
                (Some(return_type), return_fixed_length)
            } else {
                (None, None)
            };
            return Ok(Statement::Declare {
                name,
                is_function,
                params,
                return_type,
                return_fixed_length,
            });
        }
        Err(QError::Syntax("Expected name after DECLARE".to_string()))
    }

    fn parse_const(&mut self) -> QResult<Statement> {
        let name = self.parse_qualified_name("Expected name after CONST")?;
        self.consume(Token::Equal, "Expected '=' in CONST")?;
        let value = self.parse_expression()?;
        Ok(Statement::Const { name, value })
    }

    fn parse_swap(&mut self) -> QResult<Statement> {
        let var1 = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma in SWAP")?;
        let var2 = self.parse_expression()?;
        Ok(Statement::Swap { var1, var2 })
    }

    fn parse_field(&mut self) -> QResult<Statement> {
        // FIELD #filenum, width AS var$, ...
        self.consume(Token::Hash, "Expected '#' in FIELD")?;
        let file_number = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma in FIELD")?;

        let mut fields = Vec::new();
        loop {
            let width = self.parse_expression()?;
            self.consume_keyword(Keyword::As, "Expected AS in FIELD")?;
            let var = self.parse_expression()?;
            fields.push((width, var));

            if !self.match_token(&[Token::Comma]) {
                break;
            }
        }
        Ok(Statement::Field {
            file_number,
            fields,
        })
    }

    fn parse_lset(&mut self) -> QResult<Statement> {
        let target = self.parse_primary()?;
        self.consume(Token::Equal, "Expected '=' in LSET")?;
        let value = self.parse_expression()?;
        Ok(Statement::LSet { target, value })
    }

    fn parse_rset(&mut self) -> QResult<Statement> {
        let target = self.parse_primary()?;
        self.consume(Token::Equal, "Expected '=' in RSET")?;
        let value = self.parse_expression()?;
        Ok(Statement::RSet { target, value })
    }

    fn parse_locate(&mut self) -> QResult<Statement> {
        let mut args = Vec::with_capacity(5);
        args.push(
            if !self.check(&Token::Comma) && !self.check(&Token::Newline) && !self.is_at_end() {
                Some(self.parse_expression()?)
            } else {
                None
            },
        );

        while args.len() < 5 && self.match_token(&[Token::Comma]) {
            args.push(
                if !self.check(&Token::Comma) && !self.check(&Token::Newline) && !self.is_at_end() {
                    Some(self.parse_expression()?)
                } else {
                    None
                },
            );
        }

        Ok(Statement::Locate {
            row: args.first().cloned().flatten(),
            col: args.get(1).cloned().flatten(),
            cursor: args.get(2).cloned().flatten(),
            start: args.get(3).cloned().flatten(),
            stop: args.get(4).cloned().flatten(),
        })
    }

    fn parse_color(&mut self) -> QResult<Statement> {
        let foreground =
            if !self.check(&Token::Comma) && !self.check(&Token::Newline) && !self.is_at_end() {
                Some(self.parse_expression()?)
            } else {
                None
            };

        let background = if self.match_token(&[Token::Comma]) {
            if !self.check(&Token::Newline) && !self.is_at_end() {
                Some(self.parse_expression()?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Statement::Color {
            foreground,
            background,
        })
    }

    fn parse_width(&mut self) -> QResult<Statement> {
        let columns = self.parse_expression()?;
        let rows = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::Width { columns, rows })
    }

    fn parse_view(&mut self) -> QResult<Statement> {
        // Simplified VIEW parsing
        if self.match_keyword(Keyword::Print) {
            if self.check(&Token::Newline) || self.is_at_end() {
                return Ok(Statement::ViewPrint {
                    top: None,
                    bottom: None,
                });
            }

            // VIEW PRINT top TO bottom
            let top = if !self.check_keyword(Keyword::To) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            let bottom = if self.match_keyword(Keyword::To) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            return Ok(Statement::ViewPrint { top, bottom });
        }

        // VIEW (x1,y1)-(x2,y2), color, border
        if self.match_token(&[Token::OpenParen]) {
            let x1 = self.parse_expression()?;
            self.consume(Token::Comma, "Expected comma")?;
            let y1 = self.parse_expression()?;
            self.consume(Token::CloseParen, "Expected ')'")?;
            self.consume(Token::Minus, "Expected '-'")?;
            self.consume(Token::OpenParen, "Expected '('")?;
            let x2 = self.parse_expression()?;
            self.consume(Token::Comma, "Expected comma")?;
            let y2 = self.parse_expression()?;
            self.consume(Token::CloseParen, "Expected ')'")?;

            let fill_color = if self.match_token(&[Token::Comma]) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            let border_color = if self.match_token(&[Token::Comma]) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            return Ok(Statement::View {
                coords: ((x1, y1), (x2, y2)),
                fill_color,
                border_color,
            });
        }

        // VIEW with no parameters resets viewport
        Ok(Statement::ViewReset)
    }

    fn parse_window(&mut self) -> QResult<Statement> {
        // WINDOW (x1,y1)-(x2,y2)
        if self.match_token(&[Token::OpenParen]) {
            let x1 = self.parse_expression()?;
            self.consume(Token::Comma, "Expected comma")?;
            let y1 = self.parse_expression()?;
            self.consume(Token::CloseParen, "Expected ')'")?;
            self.consume(Token::Minus, "Expected '-'")?;
            self.consume(Token::OpenParen, "Expected '('")?;
            let x2 = self.parse_expression()?;
            self.consume(Token::Comma, "Expected comma")?;
            let y2 = self.parse_expression()?;
            self.consume(Token::CloseParen, "Expected ')'")?;

            return Ok(Statement::Window {
                coords: ((x1, y1), (x2, y2)),
            });
        }

        // WINDOW with no parameters resets
        Ok(Statement::WindowReset)
    }

    fn parse_pset(&mut self) -> QResult<Statement> {
        self.consume(Token::OpenParen, "Expected '(' in PSET")?;
        let x = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma")?;
        let y = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')'")?;

        let color = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Pset {
            coords: (x, y),
            color,
        })
    }

    fn parse_poke(&mut self) -> QResult<Statement> {
        let address = self.parse_expression()?;
        self.consume(Token::Comma, "Expected ',' after POKE address")?;
        let value = self.parse_expression()?;
        Ok(Statement::Poke { address, value })
    }

    fn parse_wait(&mut self) -> QResult<Statement> {
        let address = self.parse_expression()?;
        self.consume(Token::Comma, "Expected ',' after WAIT address")?;
        let and_mask = self.parse_expression()?;
        let xor_mask = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::Wait {
            address,
            and_mask,
            xor_mask,
        })
    }

    fn parse_bload(&mut self) -> QResult<Statement> {
        let filename = self.parse_expression()?;
        let offset = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::BLoad { filename, offset })
    }

    fn parse_bsave(&mut self) -> QResult<Statement> {
        let filename = self.parse_expression()?;
        self.consume(Token::Comma, "Expected ',' after BSAVE filename")?;
        let offset = self.parse_expression()?;
        self.consume(Token::Comma, "Expected ',' after BSAVE offset")?;
        let length = self.parse_expression()?;
        Ok(Statement::BSave {
            filename,
            offset,
            length,
        })
    }

    fn parse_out(&mut self) -> QResult<Statement> {
        let port = self.parse_expression()?;
        self.consume(Token::Comma, "Expected ',' after OUT port")?;
        let value = self.parse_expression()?;
        Ok(Statement::Out { port, value })
    }

    fn parse_preset(&mut self) -> QResult<Statement> {
        self.consume(Token::OpenParen, "Expected '(' in PRESET")?;
        let x = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma")?;
        let y = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')'")?;

        let color = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Preset {
            coords: (x, y),
            color,
        })
    }

    fn parse_line(&mut self) -> QResult<Statement> {
        // LINE (x1,y1)-(x2,y2), color, style
        self.consume(Token::OpenParen, "Expected '(' in LINE")?;
        let x1 = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma")?;
        let y1 = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')'")?;
        self.consume(Token::Minus, "Expected '-'")?;
        self.consume(Token::OpenParen, "Expected '('")?;
        let x2 = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma")?;
        let y2 = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')'")?;

        let color = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let style = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Line {
            coords: ((x1, y1), (x2, y2)),
            color,
            style,
            step: (false, false),
        })
    }

    fn parse_circle(&mut self) -> QResult<Statement> {
        // CIRCLE (x,y), radius, color
        self.consume(Token::OpenParen, "Expected '(' in CIRCLE")?;
        let x = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma")?;
        let y = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')'")?;
        self.consume(Token::Comma, "Expected comma after coordinates")?;
        let radius = self.parse_expression()?;

        let color = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Circle {
            center: (x, y),
            radius,
            color,
            start: None,
            end: None,
            aspect: None,
        })
    }

    fn parse_paint(&mut self) -> QResult<Statement> {
        // PAINT (x,y), color, border
        self.consume(Token::OpenParen, "Expected '(' in PAINT")?;
        let x = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma")?;
        let y = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')'")?;

        let paint_color = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let border_color = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Paint {
            coords: (x, y),
            paint_color,
            border_color,
        })
    }

    fn parse_palette(&mut self) -> QResult<Statement> {
        let attribute = self.parse_expression()?;
        let color = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::Palette { attribute, color })
    }

    fn parse_key(&mut self) -> QResult<Statement> {
        // KEY n, string or KEY ON/OFF/LIST
        if self.match_keyword(Keyword::On) {
            return Ok(Statement::KeyOn);
        } else if self.match_keyword(Keyword::Off) {
            return Ok(Statement::KeyOff);
        } else if let Some(Token::Identifier(cmd)) = self.peek() {
            if cmd.to_uppercase() == "LIST" {
                self.advance();
                return Ok(Statement::KeyList);
            }
        }

        let key_num = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma in KEY")?;
        let key_string = self.parse_expression()?;
        Ok(Statement::Key {
            key_num,
            key_string,
        })
    }

    fn parse_on_error(&mut self) -> QResult<Statement> {
        if self.match_keyword(Keyword::Goto) {
            if let Some(Token::IntegerLiteral(0)) = self.peek() {
                self.advance();
                return Ok(Statement::OnError { target: None });
            }
            let target = self.parse_goto_target()?;
            return Ok(Statement::OnError {
                target: Some(target),
            });
        } else if self.match_keyword(Keyword::Resume) {
            self.consume_keyword(Keyword::Next, "Expected NEXT after RESUME")?;
            return Ok(Statement::OnErrorResumeNext);
        }
        Err(QError::Syntax(
            "Expected GOTO or RESUME NEXT after ON ERROR".to_string(),
        ))
    }

    fn parse_on_timer(&mut self) -> QResult<Statement> {
        self.advance(); // consume ON
        self.advance(); // consume TIMER
        self.consume(Token::OpenParen, "Expected '(' after TIMER")?;
        let interval = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')'")?;
        self.consume_keyword(Keyword::Gosub, "Expected GOSUB")?;
        let label = if let Some(Token::Identifier(name)) = self.advance() {
            name.clone()
        } else {
            return Err(QError::Syntax("Expected label after GOSUB".to_string()));
        };
        Ok(Statement::OnTimer { interval, label })
    }

    fn parse_on_play(&mut self) -> QResult<Statement> {
        self.advance(); // consume ON
        self.advance(); // consume PLAY
        self.consume(Token::OpenParen, "Expected '(' after PLAY")?;
        let queue_limit = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')'")?;
        self.consume_keyword(Keyword::Gosub, "Expected GOSUB")?;
        let label = if let Some(Token::Identifier(name)) = self.advance() {
            name.clone()
        } else {
            return Err(QError::Syntax("Expected label after GOSUB".to_string()));
        };
        Ok(Statement::OnPlay { queue_limit, label })
    }

    fn parse_on_goto_gosub(&mut self) -> QResult<Statement> {
        self.advance(); // consume ON
        let expression = self.parse_expression()?;

        let is_gosub = if self.match_keyword(Keyword::Gosub) {
            true
        } else if self.match_keyword(Keyword::Goto) {
            false
        } else {
            return Err(QError::Syntax(
                "Expected GOTO or GOSUB after ON expression".to_string(),
            ));
        };

        let mut targets = Vec::new();
        loop {
            if let Some(Token::Identifier(label)) = self.advance() {
                targets.push(GotoTarget::Label(label.clone()));
            } else if let Some(Token::IntegerLiteral(line)) = self.previous() {
                targets.push(GotoTarget::LineNumber(*line as u16));
            }

            if !self.match_token(&[Token::Comma]) {
                break;
            }
        }

        Ok(Statement::OnGotoGosub {
            expression,
            targets,
            is_gosub,
        })
    }

    fn parse_open(&mut self) -> QResult<Statement> {
        let filename = self.parse_expression()?;
        self.consume_keyword(Keyword::For, "Expected FOR in OPEN")?;

        let mode = if self.match_keyword(Keyword::Input) {
            OpenMode::Input
        } else if self.match_keyword(Keyword::Output) {
            OpenMode::Output
        } else if self.match_keyword(Keyword::Append) {
            OpenMode::Append
        } else if self.match_keyword(Keyword::Binary) {
            OpenMode::Binary
        } else if self.match_keyword(Keyword::Random) {
            OpenMode::Random
        } else {
            return Err(QError::Syntax("Expected file mode in OPEN".to_string()));
        };

        let mut access = None;
        let mut lock = None;
        loop {
            if access.is_none() && self.match_keyword(Keyword::Access) {
                access = Some(if self.match_keyword(Keyword::Read) {
                    if self.match_keyword(Keyword::Write) {
                        OpenAccess::ReadWrite
                    } else {
                        OpenAccess::Read
                    }
                } else if self.match_keyword(Keyword::Write) {
                    OpenAccess::Write
                } else {
                    return Err(self.syntax_error_here("Expected READ or WRITE after ACCESS"));
                });
                continue;
            }

            if lock.is_none() && self.match_keyword(Keyword::Lock) {
                lock = Some(if self.match_keyword(Keyword::Shared) {
                    OpenLock::Shared
                } else if self.match_keyword(Keyword::Read) {
                    if self.match_keyword(Keyword::Write) {
                        OpenLock::LockReadWrite
                    } else {
                        OpenLock::LockRead
                    }
                } else if self.match_keyword(Keyword::Write) {
                    OpenLock::LockWrite
                } else {
                    return Err(
                        self.syntax_error_here("Expected SHARED, READ, or WRITE after LOCK")
                    );
                });
                continue;
            }

            break;
        }

        self.consume_keyword(Keyword::As, "Expected AS in OPEN")?;
        // # is optional in OPEN statement
        self.match_token(&[Token::Hash]);
        let file_number = self.parse_expression()?;
        let record_len = if self.match_keyword(Keyword::Len) {
            self.consume(Token::Equal, "Expected '=' after LEN in OPEN")?;
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Open {
            filename,
            mode,
            file_number,
            record_len,
            access,
            lock,
        })
    }

    fn parse_close(&mut self) -> QResult<Statement> {
        let mut file_numbers = Vec::new();
        if !self.check(&Token::Newline) && !self.is_at_end() {
            loop {
                // Consume optional # and parse expression
                let _ = self.match_token(&[Token::Hash]);
                file_numbers.push(self.parse_expression()?);
                if !self.match_token(&[Token::Comma]) {
                    break;
                }
            }
        }
        Ok(Statement::Close { file_numbers })
    }

    fn parse_input(&mut self) -> QResult<Statement> {
        // Check if it's file input: INPUT #filenum, var
        if self.match_token(&[Token::Hash]) {
            let file_number = self.parse_expression()?;
            self.consume(Token::Comma, "Expected comma")?;
            let mut variables = Vec::new();
            loop {
                variables.push(self.parse_expression()?);
                if !self.match_token(&[Token::Comma]) {
                    break;
                }
            }
            return Ok(Statement::InputFile {
                file_number,
                variables,
            });
        }

        // Console input
        let prompt = if let Some(Token::StringLiteral(_)) = self.peek() {
            let p = self.parse_expression()?;
            // Consume optional semicolon or comma separator
            let _ = self.match_token(&[Token::Semicolon, Token::Comma]);
            Some(p)
        } else {
            None
        };

        let semicolon = self.previous() == Some(&Token::Semicolon);

        let mut variables = Vec::new();
        loop {
            variables.push(self.parse_expression()?);
            if !self.match_token(&[Token::Comma]) {
                break;
            }
        }

        Ok(Statement::Input {
            prompt,
            variables,
            semicolon,
        })
    }

    fn parse_line_input(&mut self) -> QResult<Statement> {
        // LINE INPUT #filenum, var or LINE INPUT prompt; var
        if self.match_token(&[Token::Hash]) {
            let file_number = self.parse_expression()?;
            self.consume(Token::Comma, "Expected comma")?;
            let variable = self.parse_expression()?;
            return Ok(Statement::LineInputFile {
                file_number,
                variable,
            });
        }

        let _ = self.match_token(&[Token::Semicolon]);
        let prompt = if let Some(Token::StringLiteral(_)) = self.peek() {
            let p = self.parse_expression()?;
            self.match_token(&[Token::Semicolon, Token::Comma]);
            Some(p)
        } else {
            None
        };

        let variable = self.parse_expression()?;
        Ok(Statement::LineInput { prompt, variable })
    }

    fn parse_write(&mut self) -> QResult<Statement> {
        // WRITE #filenum, expr or WRITE expr
        if self.match_token(&[Token::Hash]) {
            let file_number = self.parse_expression()?;
            self.consume(Token::Comma, "Expected comma")?;
            let mut expressions = Vec::new();
            loop {
                expressions.push(self.parse_expression()?);
                if !self.match_token(&[Token::Comma]) {
                    break;
                }
            }
            return Ok(Statement::WriteFile {
                file_number,
                expressions,
            });
        }

        let mut expressions = Vec::new();
        if !self.check(&Token::Newline) && !self.is_at_end() {
            loop {
                expressions.push(self.parse_expression()?);
                if !self.match_token(&[Token::Comma]) {
                    break;
                }
            }
        }
        Ok(Statement::Write { expressions })
    }

    fn parse_get(&mut self) -> QResult<Statement> {
        self.consume(Token::Hash, "Expected '#' in GET")?;
        let file_number = self.parse_expression()?;

        let record = if self.match_token(&[Token::Comma]) {
            if self.check(&Token::Comma) {
                None
            } else {
                Some(self.parse_expression()?)
            }
        } else {
            None
        };

        let variable = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Get {
            file_number,
            record,
            variable,
        })
    }

    fn parse_get_image(&mut self) -> QResult<Statement> {
        self.consume(Token::OpenParen, "Expected '(' in GET")?;
        let x1 = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma in GET")?;
        let y1 = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')' in GET")?;
        self.consume(Token::Minus, "Expected '-' in GET")?;
        self.consume(Token::OpenParen, "Expected '(' in GET")?;
        let x2 = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma in GET")?;
        let y2 = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')' in GET")?;
        self.consume(Token::Comma, "Expected comma before image buffer in GET")?;
        let variable = self.parse_expression()?;

        Ok(Statement::GetImage {
            coords: ((x1, y1), (x2, y2)),
            variable,
        })
    }

    fn parse_put(&mut self) -> QResult<Statement> {
        self.consume(Token::Hash, "Expected '#' in PUT")?;
        let file_number = self.parse_expression()?;

        let record = if self.match_token(&[Token::Comma]) {
            if self.check(&Token::Comma) {
                None
            } else {
                Some(self.parse_expression()?)
            }
        } else {
            None
        };

        let variable = if self.match_token(&[Token::Comma]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Put {
            file_number,
            record,
            variable,
        })
    }

    fn parse_put_image(&mut self) -> QResult<Statement> {
        self.consume(Token::OpenParen, "Expected '(' in PUT")?;
        let x = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma in PUT")?;
        let y = self.parse_expression()?;
        self.consume(Token::CloseParen, "Expected ')' in PUT")?;
        self.consume(Token::Comma, "Expected comma before image buffer in PUT")?;
        let variable = self.parse_expression()?;

        let action = if self.match_token(&[Token::Comma]) {
            Some(self.parse_put_image_action()?)
        } else {
            None
        };

        Ok(Statement::PutImage {
            coords: (x, y),
            variable,
            action,
        })
    }

    fn parse_put_image_action(&mut self) -> QResult<Expression> {
        if self.match_keyword(Keyword::Pset) {
            return Ok(Expression::Literal(QType::String("PSET".to_string())));
        }
        if self.match_keyword(Keyword::Preset) {
            return Ok(Expression::Literal(QType::String("PRESET".to_string())));
        }
        if self.match_token(&[Token::And]) {
            return Ok(Expression::Literal(QType::String("AND".to_string())));
        }
        if self.match_token(&[Token::Or]) {
            return Ok(Expression::Literal(QType::String("OR".to_string())));
        }
        if self.match_keyword(Keyword::Xor) {
            return Ok(Expression::Literal(QType::String("XOR".to_string())));
        }

        self.parse_expression()
    }

    fn parse_seek(&mut self) -> QResult<Statement> {
        self.consume(Token::Hash, "Expected '#' in SEEK")?;
        let file_number = self.parse_expression()?;
        self.consume(Token::Comma, "Expected comma in SEEK")?;
        let position = self.parse_expression()?;
        Ok(Statement::Seek {
            file_number,
            position,
        })
    }

    fn parse_data(&mut self) -> QResult<Statement> {
        let mut values = Vec::new();
        loop {
            // Read until newline or colon
            let mut value = String::new();
            while !self.check(&Token::Newline)
                && !self.check(&Token::Colon)
                && !self.check(&Token::Comma)
                && !self.is_at_end()
            {
                if let Some(token) = self.advance() {
                    value.push_str(&Self::data_token_text(token));
                }
            }
            let value = value.trim().to_string();
            if !value.is_empty() {
                values.push(value);
            }

            if !self.match_token(&[Token::Comma]) {
                break;
            }
        }
        Ok(Statement::Data { values })
    }

    fn data_token_text(token: &Token) -> String {
        match token {
            Token::StringLiteral(value) => value.clone(),
            Token::Identifier(value) => value.clone(),
            Token::IntegerLiteral(value) => value.to_string(),
            Token::LongLiteral(value) => value.to_string(),
            Token::SingleLiteral(value) => value.to_string(),
            Token::DoubleLiteral(value) => value.to_string(),
            Token::Plus => "+".to_string(),
            Token::Minus => "-".to_string(),
            Token::Multiply => "*".to_string(),
            Token::Divide => "/".to_string(),
            Token::IntegerDivide => "\\".to_string(),
            Token::Modulo => "MOD".to_string(),
            Token::Power => "^".to_string(),
            Token::Equal => "=".to_string(),
            Token::NotEqual => "<>".to_string(),
            Token::LessThan => "<".to_string(),
            Token::GreaterThan => ">".to_string(),
            Token::LessOrEqual => "<=".to_string(),
            Token::GreaterOrEqual => ">=".to_string(),
            Token::And => "AND".to_string(),
            Token::Or => "OR".to_string(),
            Token::Not => "NOT".to_string(),
            Token::OpenParen => "(".to_string(),
            Token::CloseParen => ")".to_string(),
            Token::Semicolon => ";".to_string(),
            Token::Dollar => "$".to_string(),
            Token::Ampersand => "&".to_string(),
            Token::Percent => "%".to_string(),
            Token::Exclamation => "!".to_string(),
            Token::Hash => "#".to_string(),
            Token::Dot => ".".to_string(),
            Token::Keyword(keyword) => format!("{:?}", keyword),
            Token::Comma | Token::Colon | Token::LineContinuation | Token::Newline | Token::Eof => {
                String::new()
            }
        }
    }

    fn parse_read(&mut self) -> QResult<Statement> {
        let mut variables = Vec::new();
        loop {
            let name = self.parse_qualified_name("Expected variable name in READ")?;
            variables.push(Variable::new(name));
            if !self.match_token(&[Token::Comma]) {
                break;
            }
        }
        Ok(Statement::Read { variables })
    }

    fn parse_erase(&mut self) -> QResult<Statement> {
        let mut variables = Vec::new();
        loop {
            let name = self.parse_qualified_name("Expected variable name in ERASE")?;
            variables.push(Variable::new(name));
            if !self.match_token(&[Token::Comma]) {
                break;
            }
        }
        Ok(Statement::Erase { variables })
    }

    fn parse_redim(&mut self) -> QResult<Statement> {
        let mut preserve = false;
        loop {
            if !preserve
                && (self.match_keyword(Keyword::Preserve) || self.match_identifier_ci("_PRESERVE"))
            {
                preserve = true;
                continue;
            }

            if self.match_keyword(Keyword::Shared) {
                continue;
            }

            break;
        }

        let variables = self.parse_variable_declarations("REDIM")?;
        Ok(Statement::Redim {
            variables,
            preserve,
        })
    }

    fn parse_call(&mut self) -> QResult<Statement> {
        if let Some(Token::Identifier(name)) = self.advance() {
            let name = name.clone();
            let mut args = Vec::new();
            if self.match_token(&[Token::OpenParen]) {
                if !self.check(&Token::CloseParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.match_token(&[Token::Comma]) {
                            break;
                        }
                    }
                }
                self.consume(Token::CloseParen, "Expected ')'")?;
            }
            return Ok(Statement::Call { name, args });
        }
        Err(QError::Syntax(
            "Expected subroutine name after CALL".to_string(),
        ))
    }

    // Parse SUB definition: SUB name [(params)] [STATIC] ... END SUB
    fn parse_sub_definition(&mut self) -> QResult<SubDef> {
        self.consume_keyword(Keyword::Sub, "Expected SUB")?;

        let name = if let Some(Token::Identifier(n)) = self.advance() {
            n.clone()
        } else {
            return Err(QError::Syntax(
                "Expected subroutine name after SUB".to_string(),
            ));
        };

        let params = self.parse_parameter_list()?;

        // Check for STATIC
        let is_static = self.match_keyword(Keyword::Static);

        // Parse body until END SUB
        self.skip_newlines();
        let mut body = Vec::new();
        while !self.check_keyword(Keyword::EndSub) && !self.is_at_end() {
            if self.check_keyword(Keyword::Type) {
                let type_def = self.parse_type_definition()?;
                self.collected_user_types
                    .insert(type_def.name.clone(), type_def);
                self.skip_newlines();
                continue;
            }
            if self.is_declare_library_block_start() {
                for stmt in self.parse_declare_library_block()? {
                    if let Statement::Declare {
                        name,
                        is_function: true,
                        ..
                    } = &stmt
                    {
                        self.known_functions.insert(name.to_uppercase());
                    }
                    body.push(stmt);
                }
                self.skip_newlines();
                continue;
            }
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
            }
            self.skip_newlines();
        }

        self.consume_keyword(Keyword::EndSub, "Expected END SUB")?;

        Ok(SubDef {
            name,
            params,
            body,
            is_static,
        })
    }

    // Parse FUNCTION definition: FUNCTION name [(params)] [AS type] [STATIC] ... END FUNCTION
    fn parse_function_definition(&mut self) -> QResult<FunctionDef> {
        self.consume_keyword(Keyword::Function, "Expected FUNCTION")?;

        let name = if let Some(Token::Identifier(n)) = self.advance() {
            n.clone()
        } else {
            return Err(QError::Syntax(
                "Expected function name after FUNCTION".to_string(),
            ));
        };

        let params = self.parse_parameter_list()?;

        let (return_type, return_fixed_length) = self.parse_function_return_annotation(&name)?;

        // Check for STATIC
        let is_static = self.match_keyword(Keyword::Static);

        // Parse body until END FUNCTION
        self.skip_newlines();
        let mut body = Vec::new();
        while !self.check_keyword(Keyword::EndFunction) && !self.is_at_end() {
            if self.check_keyword(Keyword::Type) {
                let type_def = self.parse_type_definition()?;
                self.collected_user_types
                    .insert(type_def.name.clone(), type_def);
                self.skip_newlines();
                continue;
            }
            if self.is_declare_library_block_start() {
                for stmt in self.parse_declare_library_block()? {
                    if let Statement::Declare {
                        name,
                        is_function: true,
                        ..
                    } = &stmt
                    {
                        self.known_functions.insert(name.to_uppercase());
                    }
                    body.push(stmt);
                }
                self.skip_newlines();
                continue;
            }
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
            }
            self.skip_newlines();
        }

        self.consume_keyword(Keyword::EndFunction, "Expected END FUNCTION")?;

        Ok(FunctionDef {
            name,
            return_type,
            return_fixed_length,
            params,
            body,
            is_static,
        })
    }

    // Parse TYPE definition: TYPE typename ... field AS type ... END TYPE
    fn parse_type_definition(&mut self) -> QResult<UserType> {
        self.consume_keyword(Keyword::Type, "Expected TYPE")?;

        let name = if let Some(Token::Identifier(n)) = self.advance() {
            n.clone()
        } else {
            return Err(QError::Syntax("Expected type name after TYPE".to_string()));
        };

        self.skip_newlines();
        let mut fields = Vec::new();

        // Parse fields until END TYPE
        while !self.check_keyword(Keyword::EndType) && !self.is_at_end() {
            self.skip_newlines();

            if self.check_keyword(Keyword::EndType) {
                break;
            }

            if self.match_keyword(Keyword::As) {
                let (type_upper, fixed_length) = self.parse_declared_type_spec(
                    "Expected type name after AS",
                    "Fixed-length STRING requires a constant length",
                )?;
                let field_type = Self::type_name_to_qtype(&type_upper);

                loop {
                    let field_name = if let Some(token) = self.advance() {
                        match token {
                            Token::Identifier(name) => name.clone(),
                            Token::Keyword(kw) => format!("{:?}", kw),
                            _ => {
                                return Err(QError::Syntax(
                                    "Expected field name after TYPE field AS clause".to_string(),
                                ))
                            }
                        }
                    } else {
                        return Err(QError::Syntax(
                            "Expected field name after TYPE field AS clause".to_string(),
                        ));
                    };

                    fields.push(TypeField {
                        name: field_name,
                        field_type: field_type.clone(),
                        fixed_length,
                    });

                    if !self.match_token(&[Token::Comma]) {
                        break;
                    }
                }

                self.skip_newlines();
                continue;
            }

            // Parse field name (can be identifier or keyword)
            let field_name = if let Some(token) = self.advance() {
                match token {
                    Token::Identifier(name) => name.clone(),
                    Token::Keyword(kw) => format!("{:?}", kw), // Allow keywords as field names
                    _ => {
                        return Err(QError::Syntax(
                            "Expected field name in TYPE definition".to_string(),
                        ))
                    }
                }
            } else {
                return Err(QError::Syntax(
                    "Expected field name in TYPE definition".to_string(),
                ));
            };

            // Parse AS type
            self.consume_keyword(Keyword::As, "Expected AS in TYPE field")?;

            let (type_upper, fixed_length) = self.parse_declared_type_spec(
                "Expected type name after AS",
                "Fixed-length STRING requires a constant length",
            )?;
            let field_type = Self::type_name_to_qtype(&type_upper);

            fields.push(TypeField {
                name: field_name,
                field_type,
                fixed_length,
            });

            self.skip_newlines();
        }

        self.consume_keyword(Keyword::EndType, "Expected END TYPE")?;

        Ok(UserType { name, fields })
    }
}

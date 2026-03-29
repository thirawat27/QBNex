use crate::tokens::{Keyword, SpannedToken, Token, TokenSpan};
use core_types::{QError, QResult};

pub struct Scanner {
    input: String,
    byte_offsets: Vec<usize>,
    position: usize,
    line: usize,
    column: usize,
}

impl Scanner {
    pub fn new(input: String) -> Self {
        let mut byte_offsets = input.char_indices().map(|(idx, _)| idx).collect::<Vec<_>>();
        byte_offsets.push(input.len());
        Self {
            input,
            byte_offsets,
            position: 0,
            line: 1,
            column: 0,
        }
    }

    pub fn tokenize(&mut self) -> QResult<Vec<Token>> {
        self.tokenize_spanned().map(|tokens| {
            tokens
                .into_iter()
                .map(|spanned| spanned.token)
                .collect::<Vec<_>>()
        })
    }

    pub fn tokenize_spanned(&mut self) -> QResult<Vec<SpannedToken>> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = self.input.chars().collect();
        let len = chars.len();

        while self.position < len {
            let ch = chars[self.position];

            if ch == '\n' {
                let start = self.position;
                tokens.push(SpannedToken::new(
                    Token::Newline,
                    self.span_for_char_range(start, start + 1),
                ));
                self.position += 1;
                self.line += 1;
                self.column = 0;
                continue;
            }

            if ch.is_whitespace() && ch != '\n' {
                self.position += 1;
                self.column += 1;
                continue;
            }

            if let Some(consumed_newline_chars) = self.line_continuation_length(&chars) {
                self.position += 1 + consumed_newline_chars;
                self.line += 1;
                self.column = 0;
                continue;
            }

            if ch == '\'' || self.is_rem_comment_start(&chars) {
                self.skip_comment(&chars);
                continue;
            }

            if ch == '"' {
                let start = self.position;
                let token = self.read_string_literal(&chars)?;
                tokens.push(SpannedToken::new(
                    token,
                    self.span_for_char_range(start, self.position),
                ));
                continue;
            }

            if ch.is_ascii_digit()
                || (ch == '.' && self.peek_next_is_digit(&chars))
                || (ch == '&'
                    && self.position + 1 < len
                    && (chars[self.position + 1].eq_ignore_ascii_case(&'H')
                        || chars[self.position + 1].eq_ignore_ascii_case(&'O')))
            {
                let start = self.position;
                let token = self.read_number(&chars)?;
                tokens.push(SpannedToken::new(
                    token,
                    self.span_for_char_range(start, self.position),
                ));
                continue;
            }

            if ch.is_alphabetic() || ch == '_' || ch == '$' {
                let start = self.position;
                let token = self.read_identifier_or_keyword(&chars)?;
                tokens.push(SpannedToken::new(
                    token,
                    self.span_for_char_range(start, self.position),
                ));
                continue;
            }

            // Handle type suffixes that might appear standalone (should be treated as operators)
            if (ch == '%' || ch == '!' || ch == '#' || ch == '&')
                && self.position > 0
                && !chars[self.position - 1].is_alphanumeric()
            {
                let start = self.position;
                let token = self.read_operator_or_punctuation(&chars)?;
                tokens.push(SpannedToken::new(
                    token,
                    self.span_for_char_range(start, self.position),
                ));
                continue;
            }

            let start = self.position;
            let token = self.read_operator_or_punctuation(&chars)?;
            tokens.push(SpannedToken::new(
                token,
                self.span_for_char_range(start, self.position),
            ));
        }

        tokens.push(SpannedToken::new(Token::Eof, self.eof_span()));
        Ok(tokens)
    }

    fn span_for_char_range(&self, start: usize, end: usize) -> TokenSpan {
        let safe_start = start.min(self.byte_offsets.len().saturating_sub(1));
        let safe_end = end.min(self.byte_offsets.len().saturating_sub(1));
        let byte_start = self.byte_offsets[safe_start];
        let byte_end = self.byte_offsets[safe_end];
        TokenSpan::new(byte_start, byte_end.saturating_sub(byte_start))
    }

    fn eof_span(&self) -> TokenSpan {
        TokenSpan::new(self.input.len(), 0)
    }

    fn syntax_error(&self, message: impl Into<String>, start: usize, end: usize) -> QError {
        let span = self.span_for_char_range(start, end.max(start + 1));
        QError::syntax_at(message, span.offset, span.len)
    }

    fn check_keyword_start(&self, chars: &[char], keyword: &str) -> bool {
        let start = self.position;
        chars
            .get(start..start + keyword.len())
            .map(|c| c.iter().collect::<String>().eq_ignore_ascii_case(keyword))
            .unwrap_or(false)
    }

    fn is_identifier_char(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }

    fn is_identifier_suffix_char(ch: char) -> bool {
        matches!(ch, '%' | '!' | '#' | '$' | '&' | '~' | '`')
    }

    fn is_rem_comment_start(&self, chars: &[char]) -> bool {
        if !chars
            .get(self.position)
            .is_some_and(|ch| ch.eq_ignore_ascii_case(&'R'))
            || !self.check_keyword_start(chars, "REM")
        {
            return false;
        }

        let next = chars.get(self.position + 3).copied();
        !next.is_some_and(|ch| Self::is_identifier_char(ch) || Self::is_identifier_suffix_char(ch))
    }

    fn peek_next_is_digit(&self, chars: &[char]) -> bool {
        self.position + 1 < chars.len() && chars[self.position + 1].is_ascii_digit()
    }

    fn line_continuation_length(&self, chars: &[char]) -> Option<usize> {
        if chars.get(self.position) != Some(&'_') {
            return None;
        }

        match (chars.get(self.position + 1), chars.get(self.position + 2)) {
            (Some('\n'), _) => Some(1),
            (Some('\r'), Some('\n')) => Some(2),
            _ => None,
        }
    }

    fn skip_comment(&mut self, chars: &[char]) {
        while self.position < chars.len() && chars[self.position] != '\n' {
            self.position += 1;
        }
    }

    fn read_string_literal(&mut self, chars: &[char]) -> QResult<Token> {
        self.position += 1;
        let mut value = String::new();

        while self.position < chars.len() {
            let ch = chars[self.position];
            if ch == '"' {
                self.position += 1;
                break;
            }
            value.push(ch);
            self.position += 1;
        }

        Ok(Token::StringLiteral(value))
    }

    fn read_number(&mut self, chars: &[char]) -> QResult<Token> {
        let start = self.position;

        // Check if this is a based number (&H or &O)
        if chars[start] == '&' && self.position + 1 < chars.len() {
            let next_char = chars[self.position + 1].to_ascii_uppercase();
            if next_char == 'H' || next_char == 'O' {
                return self.read_based_number(chars, start);
            }
        }

        while self.position < chars.len() {
            let ch = chars[self.position];
            if ch.is_ascii_digit() || ch == '.' {
                self.position += 1;
            } else if ch == 'E' || ch == 'D' || ch == 'e' || ch == 'd' {
                if self.position + 1 < chars.len() {
                    let next = chars[self.position + 1];
                    if next.is_ascii_digit() || next == '+' || next == '-' {
                        self.position += 1; // consume E/D
                        if chars[self.position] == '+' || chars[self.position] == '-' {
                            self.position += 1; // consume sign
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else if ch == '&' {
                if start > 0 && chars[start] == '&' {
                    break;
                }
                self.position += 1;
                if self.position < chars.len() && chars[self.position] == '&' {
                    self.position += 1;
                }
                break;
            } else if ch == '%' {
                self.position += 1;
                if self.position < chars.len() && matches!(chars[self.position], '%' | '&') {
                    self.position += 1;
                }
                break;
            } else if ch == '!' || ch == '#' {
                self.position += 1;
                if self.position < chars.len() && chars[self.position] == ch {
                    self.position += 1;
                }
                break;
            } else {
                break;
            }
        }

        let num_str: String = chars[start..self.position].iter().collect();

        // Check for empty number string
        if num_str.is_empty() {
            return Err(self.syntax_error("Invalid number: empty number", start, self.position));
        }

        if num_str.ends_with('%') {
            let val: i16 = num_str.trim_end_matches('%').parse().map_err(|_| {
                self.syntax_error(format!("Invalid integer: {num_str}"), start, self.position)
            })?;
            return Ok(Token::IntegerLiteral(val));
        }
        if num_str.ends_with('&') {
            let val: i32 = num_str.trim_end_matches('&').parse().map_err(|_| {
                self.syntax_error(format!("Invalid long: {num_str}"), start, self.position)
            })?;
            return Ok(Token::LongLiteral(val));
        }
        if num_str.ends_with('!') {
            let val: f32 = num_str.trim_end_matches('!').parse().map_err(|_| {
                self.syntax_error(format!("Invalid single: {num_str}"), start, self.position)
            })?;
            return Ok(Token::SingleLiteral(val));
        }
        if num_str.ends_with('#') {
            let val: f64 = num_str.trim_end_matches('#').parse().map_err(|_| {
                self.syntax_error(format!("Invalid double: {num_str}"), start, self.position)
            })?;
            return Ok(Token::DoubleLiteral(val));
        }

        if num_str.contains('.') || num_str.contains('E') || num_str.contains('D') {
            if num_str.contains('D') {
                let val: f64 = num_str.replace('D', "E").parse().map_err(|_| {
                    self.syntax_error(format!("Invalid double: {num_str}"), start, self.position)
                })?;
                Ok(Token::DoubleLiteral(val))
            } else {
                let val: f32 = num_str.replace('E', "e").parse().map_err(|_| {
                    self.syntax_error(format!("Invalid single: {num_str}"), start, self.position)
                })?;
                Ok(Token::SingleLiteral(val))
            }
        } else if let Ok(val) = num_str.parse::<i16>() {
            Ok(Token::IntegerLiteral(val))
        } else if let Ok(val) = num_str.parse::<i32>() {
            Ok(Token::LongLiteral(val))
        } else {
            let val: f64 = num_str.parse().map_err(|_| {
                self.syntax_error(format!("Invalid number: {num_str}"), start, self.position)
            })?;
            Ok(Token::DoubleLiteral(val))
        }
    }

    fn read_based_number(&mut self, chars: &[char], start: usize) -> QResult<Token> {
        if start + 1 >= chars.len() {
            return Err(self.syntax_error("Invalid number", start, start + 1));
        }

        let base_char = chars[start + 1].to_ascii_uppercase();

        let num_start = if base_char == 'H' || base_char == 'O' {
            start + 2
        } else {
            start + 1
        };

        self.position = num_start;

        let mut num_str = String::new();
        while self.position < chars.len() {
            let c = chars[self.position];
            if (base_char == 'H' && c.is_ascii_hexdigit())
                || (base_char == 'O' && ('0'..='7').contains(&c))
            {
                num_str.push(c);
                self.position += 1;
            } else {
                break;
            }
        }

        if num_str.is_empty() {
            return Err(self.syntax_error(
                "Invalid based number missing digits",
                start,
                self.position.max(start + 2),
            ));
        }

        let radix = match base_char {
            'H' => 16,
            'O' => 8,
            _ => return Err(self.syntax_error("Invalid number base", start, start + 2)),
        };

        let val = u64::from_str_radix(&num_str, radix).map_err(|_| {
            self.syntax_error(
                format!(
                    "Invalid {}: {}",
                    if base_char == 'H' { "hex" } else { "octal" },
                    num_str
                ),
                start,
                self.position,
            )
        })?;

        if val <= u16::MAX as u64 {
            Ok(Token::IntegerLiteral(val as u16 as i16))
        } else if val <= u32::MAX as u64 {
            Ok(Token::LongLiteral(val as u32 as i32))
        } else {
            Ok(Token::DoubleLiteral(val as f64))
        }
    }

    fn read_identifier_or_keyword(&mut self, chars: &[char]) -> QResult<Token> {
        let start = self.position;
        let has_dollar_prefix = chars.get(self.position) == Some(&'$');

        if has_dollar_prefix {
            self.position += 1;
        }

        while self.position < chars.len() {
            let ch = chars[self.position];
            if Self::is_identifier_char(ch) {
                self.position += 1;
            } else {
                break;
            }
        }

        if has_dollar_prefix && self.position == start + 1 {
            return Ok(Token::Dollar);
        }

        let ident: String = chars[start..self.position].iter().collect();

        if has_dollar_prefix {
            return Ok(Token::Identifier(ident));
        }

        let suffix = self.read_identifier_suffix(chars);
        let full_ident = format!("{}{}", ident, suffix);
        let upper = ident.to_uppercase();

        if suffix.is_empty() && upper == "AND" {
            return Ok(Token::And);
        } else if suffix.is_empty() && upper == "OR" {
            return Ok(Token::Or);
        } else if suffix.is_empty() && upper == "NOT" {
            return Ok(Token::Not);
        }

        // Check for type suffix before keyword conversion
        // Special handling for function names with $ suffix like DATE$, TIME$, etc.
        if suffix == "$" {
            // These are built-in functions, return as identifier
            return Ok(Token::Identifier(full_ident));
        }

        // Other suffixes on identifiers
        if !suffix.is_empty() {
            return Ok(Token::Identifier(full_ident));
        }

        if let Some(keyword) = Keyword::from_keyword(&upper) {
            // Check for multi-word END variants
            if matches!(keyword, Keyword::End) {
                let saved_pos = self.position;

                // Skip whitespace without consuming actual token yet
                let mut temp_pos = saved_pos;
                while temp_pos < chars.len() && (chars[temp_pos] == ' ' || chars[temp_pos] == '\t')
                {
                    temp_pos += 1;
                }

                let next_start = temp_pos;
                while temp_pos < chars.len()
                    && (chars[temp_pos].is_alphanumeric() || chars[temp_pos] == '_')
                {
                    temp_pos += 1;
                }

                let next_ident: String = chars[next_start..temp_pos].iter().collect();
                let next_upper = next_ident.to_uppercase();

                if next_upper == "IF" {
                    self.position = temp_pos;
                    return Ok(Token::Keyword(Keyword::EndIf));
                } else if next_upper == "SELECT" {
                    self.position = temp_pos;
                    return Ok(Token::Keyword(Keyword::EndSelect));
                } else if next_upper == "SUB" {
                    self.position = temp_pos;
                    return Ok(Token::Keyword(Keyword::EndSub));
                } else if next_upper == "FUNCTION" {
                    self.position = temp_pos;
                    return Ok(Token::Keyword(Keyword::EndFunction));
                } else if next_upper == "TYPE" {
                    self.position = temp_pos;
                    return Ok(Token::Keyword(Keyword::EndType));
                }
            }

            return Ok(Token::Keyword(keyword));
        }

        Ok(Token::Identifier(ident))
    }

    fn read_identifier_suffix(&mut self, chars: &[char]) -> String {
        if self.position >= chars.len() {
            return String::new();
        }

        let suffix_start = self.position;
        let ch = chars[self.position];

        match ch {
            '%' | '#' => {
                let next = chars.get(self.position + 1).copied();
                if next.is_some_and(Self::is_identifier_char) {
                    return String::new();
                }
                self.position += 1;
                if (next == Some(ch) || (ch == '%' && next == Some('&')))
                    && !chars
                        .get(self.position + 1)
                        .copied()
                        .is_some_and(Self::is_identifier_char)
                {
                    self.position += 1;
                }
            }
            '!' | '$' => {
                let next = chars.get(self.position + 1).copied();
                if next.is_some_and(Self::is_identifier_char) {
                    return String::new();
                }
                self.position += 1;
            }
            '&' => {
                let next = chars.get(self.position + 1).copied();
                if next.is_some_and(Self::is_identifier_char) {
                    return String::new();
                }
                self.position += 1;
                if next == Some('&')
                    && !chars
                        .get(self.position + 1)
                        .copied()
                        .is_some_and(Self::is_identifier_char)
                {
                    self.position += 1;
                }
            }
            '~' => {
                self.position += 1;
                if self.position < chars.len() {
                    match chars[self.position] {
                        '%' => {
                            self.position += 1;
                            if self.position < chars.len()
                                && matches!(chars[self.position], '%' | '&')
                            {
                                self.position += 1;
                            }
                        }
                        '&' => {
                            self.position += 1;
                            if self.position < chars.len() && chars[self.position] == '&' {
                                self.position += 1;
                            }
                        }
                        '`' => {
                            self.position += 1;
                            while self.position < chars.len()
                                && chars[self.position].is_ascii_digit()
                            {
                                self.position += 1;
                            }
                        }
                        _ => {
                            self.position = suffix_start;
                            return String::new();
                        }
                    }
                } else {
                    self.position = suffix_start;
                    return String::new();
                }

                if chars
                    .get(self.position)
                    .copied()
                    .is_some_and(Self::is_identifier_char)
                {
                    self.position = suffix_start;
                    return String::new();
                }
            }
            '`' => {
                self.position += 1;
                while self.position < chars.len() && chars[self.position].is_ascii_digit() {
                    self.position += 1;
                }

                if chars
                    .get(self.position)
                    .copied()
                    .is_some_and(Self::is_identifier_char)
                {
                    self.position = suffix_start;
                    return String::new();
                }
            }
            _ => return String::new(),
        }

        chars[suffix_start..self.position].iter().collect()
    }

    fn read_operator_or_punctuation(&mut self, chars: &[char]) -> QResult<Token> {
        let ch = chars[self.position];
        self.position += 1;

        match ch {
            '+' => Ok(Token::Plus),
            '-' => Ok(Token::Minus),
            '*' => Ok(Token::Multiply),
            '/' => Ok(Token::Divide),
            '\\' => Ok(Token::IntegerDivide),
            '^' => Ok(Token::Power),
            '=' => {
                if self.position < chars.len() {
                    let next = chars[self.position];
                    if next == '<' {
                        self.position += 1;
                        return Ok(Token::LessOrEqual);
                    } else if next == '>' {
                        self.position += 1;
                        return Ok(Token::GreaterOrEqual);
                    }
                }
                Ok(Token::Equal)
            }
            '<' => {
                if self.position < chars.len() {
                    let next = chars[self.position];
                    if next == '>' {
                        self.position += 1;
                        return Ok(Token::NotEqual);
                    } else if next == '=' {
                        self.position += 1;
                        return Ok(Token::LessOrEqual);
                    }
                }
                Ok(Token::LessThan)
            }
            '>' => {
                if self.position < chars.len() && chars[self.position] == '=' {
                    self.position += 1;
                    return Ok(Token::GreaterOrEqual);
                }
                Ok(Token::GreaterThan)
            }
            '(' => Ok(Token::OpenParen),
            ')' => Ok(Token::CloseParen),
            ',' => Ok(Token::Comma),
            ';' => Ok(Token::Semicolon),
            ':' => Ok(Token::Colon),
            '$' => Ok(Token::Dollar),
            '&' => Ok(Token::Ampersand),
            '#' => Ok(Token::Hash),
            '%' => Ok(Token::Percent),
            '!' => Ok(Token::Exclamation),
            '.' => Ok(Token::Dot),
            _ => Err(self.syntax_error(
                format!("Unexpected character: {ch}"),
                self.position - 1,
                self.position,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_tokenize() {
        let mut scanner = Scanner::new("123".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::IntegerLiteral(123));
    }

    #[test]
    fn test_string_tokenize() {
        let mut scanner = Scanner::new("\"Hello\"".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::StringLiteral("Hello".to_string()));
    }

    #[test]
    fn test_keyword_tokenize() {
        let mut scanner = Scanner::new("PRINT".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Keyword(Keyword::Print));
    }

    #[test]
    fn test_hex_number() {
        let mut scanner = Scanner::new("&HFF".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::IntegerLiteral(255));
    }

    #[test]
    fn test_large_hex_number_promotes_to_long() {
        let mut scanner = Scanner::new("&HFFFF00".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::LongLiteral(16_776_960));
    }

    #[test]
    fn test_decimal_number_not_dot() {
        // Decimal numbers should NOT produce a Dot token
        let mut scanner = Scanner::new("3.14".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert!(matches!(tokens[0], Token::SingleLiteral(_)));
    }

    #[test]
    fn test_dot_token() {
        // Standalone dot should produce Dot token
        // Note: "field" and "name" are QBASIC keywords, so the parser handles both cases
        let mut scanner = Scanner::new("obj.x".to_string());
        let tokens = scanner.tokenize().unwrap();
        // Skip any newlines at the beginning if present
        let mut idx = 0;
        while idx < tokens.len() && matches!(tokens[idx], Token::Newline) {
            idx += 1;
        }
        // First token should be identifier "obj"
        assert!(
            matches!(tokens[idx], Token::Identifier(_)),
            "Expected Identifier, got {:?}",
            tokens[idx]
        );
        // Second token should be Dot
        assert_eq!(tokens[idx + 1], Token::Dot, "Expected Dot");
        // Third token can be either Identifier or Keyword (QBASIC allows keywords as field names)
        assert!(
            matches!(tokens[idx + 2], Token::Identifier(_) | Token::Keyword(_)),
            "Expected Identifier or Keyword after dot, got {:?}",
            tokens[idx + 2]
        );
    }

    #[test]
    fn test_dot_after_closing_paren() {
        // obj.method() should work correctly
        let mut scanner = Scanner::new("obj.method()".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert!(matches!(tokens[0], Token::Identifier(_)));
        assert_eq!(tokens[1], Token::Dot);
        assert!(matches!(tokens[2], Token::Identifier(_)));
        assert_eq!(tokens[3], Token::OpenParen);
        assert_eq!(tokens[4], Token::CloseParen);
    }

    #[test]
    fn test_qb64_metacommand_tokenize() {
        let mut scanner = Scanner::new("$CONSOLE\nPRINT 1".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Identifier("$CONSOLE".to_string()));
        assert_eq!(tokens[1], Token::Newline);
        assert_eq!(tokens[2], Token::Keyword(Keyword::Print));
    }

    #[test]
    fn test_line_continuation_is_treated_as_whitespace() {
        let mut scanner = Scanner::new("\"QB\" + _\n\"64\"".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::StringLiteral("QB".to_string()));
        assert_eq!(tokens[1], Token::Plus);
        assert_eq!(tokens[2], Token::StringLiteral("64".to_string()));
        assert_eq!(tokens[3], Token::Eof);
    }

    #[test]
    fn test_rem_is_only_a_comment_when_standalone() {
        let mut scanner =
            Scanner::new("FUNCTION RemoveFileExtension$ (f$)\nEND FUNCTION".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Keyword(Keyword::Function));
        assert_eq!(
            tokens[1],
            Token::Identifier("RemoveFileExtension$".to_string())
        );
    }

    #[test]
    fn test_qb64_unsigned_suffixes_are_part_of_identifier() {
        let mut scanner = Scanner::new(
            "constval~&& n1~%% n1~% n1~& n1~%& n1` n1`4 i2&& n1%% n1## n1%&".to_string(),
        );
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Identifier("constval~&&".to_string()));
        assert_eq!(tokens[1], Token::Identifier("n1~%%".to_string()));
        assert_eq!(tokens[2], Token::Identifier("n1~%".to_string()));
        assert_eq!(tokens[3], Token::Identifier("n1~&".to_string()));
        assert_eq!(tokens[4], Token::Identifier("n1~%&".to_string()));
        assert_eq!(tokens[5], Token::Identifier("n1`".to_string()));
        assert_eq!(tokens[6], Token::Identifier("n1`4".to_string()));
        assert_eq!(tokens[7], Token::Identifier("i2&&".to_string()));
        assert_eq!(tokens[8], Token::Identifier("n1%%".to_string()));
        assert_eq!(tokens[9], Token::Identifier("n1##".to_string()));
        assert_eq!(tokens[10], Token::Identifier("n1%&".to_string()));
    }

    #[test]
    fn test_large_integer_literal_falls_back_to_double() {
        let mut scanner = Scanner::new("9999999999".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::DoubleLiteral(9_999_999_999.0));
    }

    #[test]
    fn test_double_literals_accept_qb64_double_hash_suffix() {
        let mut scanner = Scanner::new("3.141592653589793##".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert_eq!(tokens[0], Token::DoubleLiteral(std::f64::consts::PI));
    }

    #[test]
    fn test_qbasic_legacy_relational_operators_are_tokenized() {
        let mut scanner = Scanner::new("IF a =< b THEN IF c => d THEN".to_string());
        let tokens = scanner.tokenize().unwrap();
        assert!(tokens.contains(&Token::LessOrEqual));
        assert!(tokens.contains(&Token::GreaterOrEqual));
    }
}

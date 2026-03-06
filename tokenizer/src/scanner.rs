use crate::tokens::{Keyword, Token};
use core_types::QResult;

pub struct Scanner {
    input: String,
    position: usize,
    line: usize,
    column: usize,
}

impl Scanner {
    pub fn new(input: String) -> Self {
        Self {
            input,
            position: 0,
            line: 1,
            column: 0,
        }
    }

    pub fn tokenize(&mut self) -> QResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = self.input.chars().collect();
        let len = chars.len();

        while self.position < len {
            let ch = chars[self.position];

            if ch == '\n' {
                tokens.push(Token::Newline);
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

            if ch == '_' && self.position + 1 < len && chars[self.position + 1] == '\n' {
                tokens.push(Token::LineContinuation);
                self.position += 2;
                self.column = 0;
                continue;
            }

            if ch == '\''
                || (ch.eq_ignore_ascii_case(&'R') && self.check_keyword_start(&chars, "REM"))
            {
                self.skip_comment(&chars);
                continue;
            }

            if ch == '"' {
                tokens.push(self.read_string_literal(&chars)?);
                continue;
            }

            if ch.is_ascii_digit()
                || (ch == '.' && self.peek_next_is_digit(&chars))
                || (ch == '&'
                    && self.position + 1 < len
                    && (chars[self.position + 1].eq_ignore_ascii_case(&'H')
                        || chars[self.position + 1].eq_ignore_ascii_case(&'O')))
            {
                tokens.push(self.read_number(&chars)?);
                continue;
            }

            if ch.is_alphabetic() || ch == '_' || ch == '$' {
                tokens.push(self.read_identifier_or_keyword(&chars)?);
                continue;
            }

            // Handle type suffixes that might appear standalone (should be treated as operators)
            if (ch == '%' || ch == '!' || ch == '#' || ch == '&')
                && self.position > 0
                && !chars[self.position - 1].is_alphanumeric()
            {
                tokens.push(self.read_operator_or_punctuation(&chars)?);
                continue;
            }

            tokens.push(self.read_operator_or_punctuation(&chars)?);
        }

        tokens.push(Token::Eof);
        Ok(tokens)
    }

    fn check_keyword_start(&self, chars: &[char], keyword: &str) -> bool {
        let start = self.position;
        chars
            .get(start..start + keyword.len())
            .map(|c| c.iter().collect::<String>().eq_ignore_ascii_case(keyword))
            .unwrap_or(false)
    }

    fn peek_next_is_digit(&self, chars: &[char]) -> bool {
        self.position + 1 < chars.len() && chars[self.position + 1].is_ascii_digit()
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
            } else if ch == '%' || ch == '!' || ch == '#' {
                self.position += 1;
                break;
            } else {
                break;
            }
        }

        let num_str: String = chars[start..self.position].iter().collect();

        // Check for empty number string
        if num_str.is_empty() {
            return Err(core_types::QError::Syntax(format!(
                "Invalid number: empty number at position {}",
                start
            )));
        }

        if num_str.ends_with('%') {
            let val: i16 = num_str
                .trim_end_matches('%')
                .parse()
                .map_err(|_| core_types::QError::Syntax(format!("Invalid integer: {}", num_str)))?;
            return Ok(Token::IntegerLiteral(val));
        }
        if num_str.ends_with('&') {
            let val: i32 = num_str
                .trim_end_matches('&')
                .parse()
                .map_err(|_| core_types::QError::Syntax(format!("Invalid long: {}", num_str)))?;
            return Ok(Token::LongLiteral(val));
        }
        if num_str.ends_with('!') {
            let val: f32 = num_str
                .trim_end_matches('!')
                .parse()
                .map_err(|_| core_types::QError::Syntax(format!("Invalid single: {}", num_str)))?;
            return Ok(Token::SingleLiteral(val));
        }
        if num_str.ends_with('#') {
            let val: f64 = num_str
                .trim_end_matches('#')
                .parse()
                .map_err(|_| core_types::QError::Syntax(format!("Invalid double: {}", num_str)))?;
            return Ok(Token::DoubleLiteral(val));
        }

        if num_str.contains('.') || num_str.contains('E') || num_str.contains('D') {
            if num_str.contains('D') {
                let val: f64 = num_str.replace('D', "E").parse().map_err(|_| {
                    core_types::QError::Syntax(format!("Invalid double: {}", num_str))
                })?;
                Ok(Token::DoubleLiteral(val))
            } else {
                let val: f32 = num_str.replace('E', "e").parse().map_err(|_| {
                    core_types::QError::Syntax(format!("Invalid single: {}", num_str))
                })?;
                Ok(Token::SingleLiteral(val))
            }
        } else if let Ok(val) = num_str.parse::<i16>() {
            Ok(Token::IntegerLiteral(val))
        } else {
            let val: i32 = num_str
                .parse()
                .map_err(|_| core_types::QError::Syntax(format!("Invalid number: {}", num_str)))?;
            Ok(Token::LongLiteral(val))
        }
    }

    fn read_based_number(&mut self, chars: &[char], start: usize) -> QResult<Token> {
        if start + 1 >= chars.len() {
            return Err(core_types::QError::Syntax("Invalid number".to_string()));
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
            return Err(core_types::QError::Syntax(
                "Invalid based number missing digits".to_string(),
            ));
        }

        if base_char == 'H' {
            let val = u16::from_str_radix(&num_str, 16)
                .map_err(|_| core_types::QError::Syntax(format!("Invalid hex: {}", num_str)))?;
            Ok(Token::IntegerLiteral(val as i16))
        } else if base_char == 'O' {
            let val = u16::from_str_radix(&num_str, 8)
                .map_err(|_| core_types::QError::Syntax(format!("Invalid octal: {}", num_str)))?;
            Ok(Token::IntegerLiteral(val as i16))
        } else {
            Err(core_types::QError::Syntax(
                "Invalid number base".to_string(),
            ))
        }
    }

    fn read_identifier_or_keyword(&mut self, chars: &[char]) -> QResult<Token> {
        let start = self.position;

        while self.position < chars.len() {
            let ch = chars[self.position];
            if ch.is_alphanumeric() || ch == '_' {
                self.position += 1;
            } else {
                break;
            }
        }

        let ident: String = chars[start..self.position].iter().collect();

        let mut has_suffix = false;
        let mut suffix = '\0';

        if self.position < chars.len() {
            let ch = chars[self.position];
            if ch == '%' || ch == '!' || ch == '#' || ch == '$' || ch == '&' {
                if self.position + 1 < chars.len() && chars[self.position + 1].is_alphanumeric() {
                } else {
                    has_suffix = true;
                    suffix = ch;
                    self.position += 1;
                }
            }
        }

        let upper = ident.to_uppercase();

        if upper == "AND" {
            return Ok(Token::And);
        } else if upper == "OR" {
            return Ok(Token::Or);
        } else if upper == "NOT" {
            return Ok(Token::Not);
        }

        // Check for type suffix before keyword conversion
        // Special handling for function names with $ suffix like DATE$, TIME$, etc.
        if has_suffix && suffix == '$' {
            let full_name = format!("{}{}", ident, suffix);
            // These are built-in functions, return as identifier
            return Ok(Token::Identifier(full_name));
        }

        // Other suffixes on identifiers
        if has_suffix {
            return Ok(Token::Identifier(format!("{}{}", ident, suffix)));
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
            '=' => Ok(Token::Equal),
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
            _ => Err(core_types::QError::Syntax(format!(
                "Unexpected character: {}",
                ch
            ))),
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
}

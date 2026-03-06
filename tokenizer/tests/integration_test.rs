use tokenizer::{Scanner, Token};

#[test]
fn test_tokenize_print_statement() {
    let mut scanner = Scanner::new("PRINT \"Hello\"".to_string());
    let tokens = scanner.tokenize().unwrap();
    assert!(tokens.len() > 1);
}

#[test]
fn test_tokenize_variable_assignment() {
    let mut scanner = Scanner::new("x = 42".to_string());
    let tokens = scanner.tokenize().unwrap();
    assert!(tokens.len() >= 3);
}

#[test]
fn test_tokenize_for_loop() {
    let mut scanner = Scanner::new("FOR i = 1 TO 10".to_string());
    let tokens = scanner.tokenize().unwrap();
    assert!(tokens.len() >= 5);
}

#[test]
fn test_tokenize_numbers() {
    let mut scanner = Scanner::new("123 3.14 -5".to_string());
    let tokens = scanner.tokenize().unwrap();
    assert!(tokens.len() >= 3);
}

#[test]
fn test_tokenize_strings() {
    let mut scanner = Scanner::new("\"Hello World\"".to_string());
    let tokens = scanner.tokenize().unwrap();
    assert!(tokens.len() >= 1);
}

#[test]
fn test_tokenize_operators() {
    let mut scanner = Scanner::new("+ - * / = < > <= >= <>".to_string());
    let tokens = scanner.tokenize().unwrap();
    assert!(tokens.len() >= 10);
}

#[test]
fn test_tokenize_comments() {
    let mut scanner = Scanner::new("' This is a comment\nPRINT 42".to_string());
    let tokens = scanner.tokenize().unwrap();
    // Comments should be filtered out
    assert!(tokens.iter().any(|t| matches!(t, Token::Keyword(_))));
}

#[test]
fn test_tokenize_empty_input() {
    let mut scanner = Scanner::new("".to_string());
    let tokens = scanner.tokenize().unwrap();
    // Empty input may have EOF token
    assert!(tokens.is_empty() || tokens.len() == 1);
}

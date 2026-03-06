use core_types::QType;
use syntax_tree::{ast_nodes::Statement, Parser};

#[test]
fn test_parse_print_statement() {
    let mut parser = Parser::new("PRINT \"Hello\"".to_string()).unwrap();
    let program = parser.parse();
    assert!(program.is_ok());
    let prog = program.unwrap();
    assert!(!prog.statements.is_empty());
}

#[test]
fn test_parse_variable_assignment() {
    let mut parser = Parser::new("x = 42".to_string()).unwrap();
    let program = parser.parse();
    assert!(program.is_ok());
}

#[test]
fn test_parse_for_loop() {
    let mut parser = Parser::new("FOR i = 1 TO 10\nPRINT i\nNEXT i".to_string()).unwrap();
    let program = parser.parse();
    assert!(program.is_ok());
}

#[test]
fn test_parse_if_statement() {
    let mut parser = Parser::new("IF x > 0 THEN\nPRINT \"positive\"\nEND IF".to_string()).unwrap();
    let program = parser.parse();
    assert!(program.is_ok());
}

#[test]
fn test_parse_dim_statement() {
    let mut parser = Parser::new("DIM x AS INTEGER".to_string()).unwrap();
    let program = parser.parse();
    assert!(program.is_ok());
}

#[test]
fn test_parse_function_call() {
    let mut parser = Parser::new("x = SQR(16)".to_string()).unwrap();
    let program = parser.parse();
    assert!(program.is_ok());
}

#[test]
fn test_parse_empty_program() {
    let mut parser = Parser::new("".to_string()).unwrap();
    let program = parser.parse();
    assert!(program.is_ok());
}

#[test]
fn test_parse_advanced_graphics_statements() {
    let source = "\
VIEW (1, 2)-(30, 40), 5, 6
WINDOW (0, 0)-(319, 199)
GET (0, 0)-(10, 10), sprite
PUT (20, 20), sprite, XOR
VIEW
WINDOW";

    let mut parser = Parser::new(source.to_string()).unwrap();
    let program = parser.parse().unwrap();

    assert!(matches!(program.statements[0], Statement::View { .. }));
    assert!(matches!(program.statements[1], Statement::Window { .. }));

    match &program.statements[2] {
        Statement::GetImage { variable, .. } => {
            assert!(matches!(
                variable,
                syntax_tree::ast_nodes::Expression::Variable(var) if var.name == "sprite"
            ));
        }
        other => panic!("expected GetImage, got {other:?}"),
    }

    match &program.statements[3] {
        Statement::PutImage {
            action: Some(syntax_tree::ast_nodes::Expression::Literal(QType::String(action))),
            ..
        } => {
            assert_eq!(action, "XOR");
        }
        other => panic!("expected PutImage with XOR action, got {other:?}"),
    }

    assert!(matches!(program.statements[4], Statement::ViewReset));
    assert!(matches!(program.statements[5], Statement::WindowReset));
}

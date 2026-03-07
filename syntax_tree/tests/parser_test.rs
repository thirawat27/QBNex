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

#[test]
fn test_parse_view_print_reset() {
    let mut parser = Parser::new("VIEW PRINT 1 TO 5\nVIEW PRINT".to_string()).unwrap();
    let program = parser.parse().unwrap();

    assert!(matches!(
        program.statements[0],
        Statement::ViewPrint {
            top: Some(_),
            bottom: Some(_)
        }
    ));
    assert!(matches!(
        program.statements[1],
        Statement::ViewPrint {
            top: None,
            bottom: None
        }
    ));
}

#[test]
fn test_parse_tron_troff_as_compatibility_noops() {
    let mut parser = Parser::new("TRON\nTROFF".to_string()).unwrap();
    let program = parser.parse().unwrap();

    assert!(matches!(
        program.statements[0],
        Statement::Print {
            ref expressions,
            newline: false
        } if expressions.is_empty()
    ));
    assert!(matches!(
        program.statements[1],
        Statement::Print {
            ref expressions,
            newline: false
        } if expressions.is_empty()
    ));
}

#[test]
fn test_parse_shared_statement_as_compatibility_noop() {
    let mut parser =
        Parser::new("SUB Demo()\nSHARED GlobalVal, SharedArr\nEND SUB".to_string()).unwrap();
    let program = parser.parse().unwrap();
    let sub = program.subs.get("Demo").unwrap();

    assert!(matches!(
        sub.body[0],
        Statement::Print {
            ref expressions,
            newline: false
        } if expressions.is_empty()
    ));
}

#[test]
fn test_parse_input_dollar_with_file_number() {
    let mut parser = Parser::new("PRINT INPUT$(5, #1)".to_string()).unwrap();
    let program = parser.parse().unwrap();

    match &program.statements[0] {
        Statement::Print { expressions, .. } => {
            assert!(matches!(
                &expressions[0],
                syntax_tree::ast_nodes::Expression::ArrayAccess { name, indices, .. }
                if name == "INPUT$" && indices.len() == 2
            ));
        }
        other => panic!("expected Print, got {other:?}"),
    }
}

#[test]
fn test_parse_open_random_with_len() {
    let mut parser =
        Parser::new("OPEN \"rand.dat\" FOR RANDOM AS #1 LEN = 14".to_string()).unwrap();
    let program = parser.parse().unwrap();

    match &program.statements[0] {
        Statement::Open { record_len, .. } => {
            assert!(record_len.is_some());
        }
        other => panic!("expected Open, got {other:?}"),
    }
}

#[test]
fn test_parse_redim_preserve_with_type_and_bounds() {
    let mut parser =
        Parser::new("REDIM PRESERVE dynamicArr(1 TO 10) AS INTEGER".to_string()).unwrap();
    let program = parser.parse().unwrap();

    match &program.statements[0] {
        Statement::Redim {
            preserve,
            variables,
        } => {
            assert!(*preserve);
            assert_eq!(variables.len(), 1);
            assert!(variables[0].1.is_some());
            assert_eq!(variables[0].0.type_suffix, Some('%'));
        }
        other => panic!("expected Redim, got {other:?}"),
    }
}

#[test]
fn test_parse_lset_and_rset() {
    let mut parser =
        Parser::new("LSET F1$ = MKS$(12.34)\nRSET F2$ = \"TEST\"".to_string()).unwrap();
    let program = parser.parse().unwrap();

    assert!(matches!(program.statements[0], Statement::LSet { .. }));
    assert!(matches!(program.statements[1], Statement::RSet { .. }));
}

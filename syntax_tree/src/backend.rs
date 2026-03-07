use crate::ast_nodes::{Expression, Program, Statement};
use core_types::{QError, QResult};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Vm,
    Native,
}

impl Backend {
    pub fn display_name(self) -> &'static str {
        match self {
            Backend::Vm => "interpreter",
            Backend::Native => "native compiler",
        }
    }
}

pub fn unsupported_statements(program: &Program, backend: Backend) -> Vec<&'static str> {
    let mut unsupported = BTreeSet::new();

    collect_unsupported(&program.statements, backend, &mut unsupported);

    for sub in program.subs.values() {
        collect_unsupported(&sub.body, backend, &mut unsupported);
    }

    for func in program.functions.values() {
        collect_unsupported(&func.body, backend, &mut unsupported);
    }

    if backend == Backend::Native && native_supports_top_level_control_flow(program) {
        unsupported.remove("GOTO");
        unsupported.remove("GOSUB");
        unsupported.remove("RETURN");
        unsupported.remove("ON ... GOTO/GOSUB");
        unsupported.remove("ON ERROR");
        unsupported.remove("ON ERROR RESUME NEXT");
        unsupported.remove("ERROR");
        unsupported.remove("RESUME");
        unsupported.remove("RESUME NEXT");
        unsupported.remove("RESUME <label>");
        unsupported.remove("ON TIMER");
        unsupported.remove("TIMER ON");
        unsupported.remove("TIMER OFF");
        unsupported.remove("TIMER STOP");
    }

    unsupported.into_iter().collect()
}

pub fn validate_program(program: &Program, backend: Backend) -> QResult<()> {
    let unsupported = unsupported_statements(program, backend);
    if unsupported.is_empty() {
        return Ok(());
    }

    Err(QError::UnsupportedFeature(format!(
        "{} does not support: {}",
        backend.display_name(),
        unsupported.join(", ")
    )))
}

fn collect_unsupported(
    statements: &[Statement],
    backend: Backend,
    unsupported: &mut BTreeSet<&'static str>,
) {
    for statement in statements {
        if let Some(name) = unsupported_statement(statement, backend) {
            unsupported.insert(name);
        }

        match statement {
            Statement::IfBlock {
                then_branch,
                else_branch,
                ..
            } => {
                collect_unsupported(then_branch, backend, unsupported);
                if let Some(branch) = else_branch {
                    collect_unsupported(branch, backend, unsupported);
                }
            }
            Statement::IfElseBlock {
                then_branch,
                else_ifs,
                else_branch,
                ..
            } => {
                collect_unsupported(then_branch, backend, unsupported);
                for (_, branch) in else_ifs {
                    collect_unsupported(branch, backend, unsupported);
                }
                if let Some(branch) = else_branch {
                    collect_unsupported(branch, backend, unsupported);
                }
            }
            Statement::ForLoop { body, .. }
            | Statement::WhileLoop { body, .. }
            | Statement::DoLoop { body, .. }
            | Statement::ForEach { body, .. } => collect_unsupported(body, backend, unsupported),
            Statement::Select { cases, .. } => {
                for (_, branch) in cases {
                    collect_unsupported(branch, backend, unsupported);
                }
            }
            _ => {}
        }
    }
}

fn unsupported_statement(statement: &Statement, backend: Backend) -> Option<&'static str> {
    match backend {
        Backend::Vm => None,
        Backend::Native => match statement {
            Statement::PrintUsing { .. } => None,
            Statement::Goto { .. } => Some("GOTO"),
            Statement::Gosub { .. } => Some("GOSUB"),
            Statement::Return => Some("RETURN"),
            Statement::Get { variable, .. } => {
                if native_get_supported(variable.as_ref()) {
                    None
                } else {
                    Some("GET")
                }
            }
            Statement::Put { variable, .. } => {
                if native_put_supported(variable.as_ref()) {
                    None
                } else {
                    Some("PUT")
                }
            }
            Statement::Sound { .. } => None,
            Statement::Play { .. } => None,
            Statement::ForEach { .. } => None,
            Statement::OnError { .. } => Some("ON ERROR"),
            Statement::OnErrorResumeNext => Some("ON ERROR RESUME NEXT"),
            Statement::Error { .. } => Some("ERROR"),
            Statement::Resume => Some("RESUME"),
            Statement::ResumeNext => Some("RESUME NEXT"),
            Statement::ResumeLabel { .. } => Some("RESUME <label>"),
            Statement::Chain { .. } => None,
            Statement::Shell { .. } => None,
            Statement::OnTimer { .. } => Some("ON TIMER"),
            Statement::OnGotoGosub { .. } => Some("ON ... GOTO/GOSUB"),
            Statement::TimerOn => Some("TIMER ON"),
            Statement::TimerOff => Some("TIMER OFF"),
            Statement::TimerStop => Some("TIMER STOP"),
            _ => None,
        },
    }
}

fn native_get_supported(variable: Option<&Expression>) -> bool {
    match variable {
        None => true,
        Some(expr) => native_get_put_target_supported(expr),
    }
}

fn native_get_put_target_supported(expr: &Expression) -> bool {
    match expr {
        Expression::Variable(_) => true,
        Expression::ArrayAccess { indices, .. } => indices.len() == 1,
        Expression::FieldAccess { object, .. } => native_get_put_target_supported(object),
        _ => false,
    }
}

fn native_put_supported(variable: Option<&Expression>) -> bool {
    match variable {
        None => true,
        Some(expr) => native_get_put_target_supported(expr),
    }
}

fn native_supports_top_level_control_flow(program: &Program) -> bool {
    if has_disallowed_nested_control_flow(&program.statements, false, false) {
        return false;
    }

    for sub in program.subs.values() {
        if has_disallowed_nested_control_flow(&sub.body, false, false) {
            return false;
        }
    }

    for func in program.functions.values() {
        if has_disallowed_nested_control_flow(&func.body, false, false) {
            return false;
        }
    }

    true
}

#[allow(clippy::only_used_in_recursion)]
fn has_disallowed_nested_control_flow(
    statements: &[Statement],
    in_loop: bool,
    nested: bool,
) -> bool {
    for statement in statements {
        if nested
            && matches!(
                statement,
                Statement::Label { .. } | Statement::LineNumber { .. }
            )
        {
            return true;
        }

        match statement {
            Statement::IfBlock {
                then_branch,
                else_branch,
                ..
            } => {
                if has_disallowed_nested_control_flow(then_branch, in_loop, true)
                    || else_branch.as_ref().is_some_and(|branch| {
                        has_disallowed_nested_control_flow(branch, in_loop, true)
                    })
                {
                    return true;
                }
            }
            Statement::IfElseBlock {
                then_branch,
                else_ifs,
                else_branch,
                ..
            } => {
                if has_disallowed_nested_control_flow(then_branch, in_loop, true)
                    || else_ifs.iter().any(|(_, branch)| {
                        has_disallowed_nested_control_flow(branch, in_loop, true)
                    })
                    || else_branch.as_ref().is_some_and(|branch| {
                        has_disallowed_nested_control_flow(branch, in_loop, true)
                    })
                {
                    return true;
                }
            }
            Statement::ForLoop { body, .. }
            | Statement::WhileLoop { body, .. }
            | Statement::DoLoop { body, .. }
            | Statement::ForEach { body, .. } => {
                if has_disallowed_nested_control_flow(body, true, true) {
                    return true;
                }
            }
            Statement::Select { cases, .. } => {
                if cases
                    .iter()
                    .any(|(_, branch)| has_disallowed_nested_control_flow(branch, in_loop, true))
                {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{unsupported_statements, Backend};
    use crate::Parser;

    fn parse(source: &str) -> Program {
        let mut parser = Parser::new(source.to_string()).unwrap();
        parser.parse().unwrap()
    }

    use crate::ast_nodes::{Program, Statement as ProgramStatement};

    #[test]
    fn vm_report_lists_missing_runtime_features() {
        let program = parse("FIELD #1, 4 AS A$\nLSET A$ = \"X\"\nWIDTH 80");
        let unsupported = unsupported_statements(&program, Backend::Vm);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_loop_nested_timer() {
        let program =
            parse("FOR I = 1 TO 3\n  ON TIMER(1) GOSUB tick\nNEXT\ntick:\nTIMER OFF\nRETURN");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_random_file_string_forms() {
        let program = parse(
            "FIELD #1, 4 AS A$\nLSET A$ = \"X\"\nPUT #1, 1\nGET #1, 1\nGET #1, 1, B$\nPUT #1, 1, B$",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_numeric_binary_get_put_forms() {
        let program =
            parse("GET #1, 1, N\nPUT #1, 1, N\nGET #1, 1, I%\nPUT #1, 1, D#\nGET #1, 1, A(1)\nPUT #1, 1, A(1)");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_string_array_binary_get_put_forms() {
        let program = parse("DIM B$(2)\nGET #1, 1, B$(1)\nPUT #1, 1, B$(1)");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn parser_preserves_fixed_length_string_metadata_on_dim() {
        let program = parse("DIM A$ AS STRING * 4\nDIM B$(2) AS STRING * 6");
        match &program.statements[0] {
            ProgramStatement::Dim { variables, .. } => {
                assert_eq!(variables[0].0.fixed_length, Some(4));
            }
            _ => panic!("expected DIM statement"),
        }
        match &program.statements[1] {
            ProgramStatement::Dim { variables, .. } => {
                assert_eq!(variables[0].0.fixed_length, Some(6));
            }
            _ => panic!("expected DIM statement"),
        }
    }

    #[test]
    fn parser_preserves_fixed_length_string_metadata_on_function_return() {
        let program =
            parse("PRINT FIXED$\nFUNCTION FIXED$() AS STRING * 4\nFIXED$ = \"ABCD\"\nEND FUNCTION");
        let func = program.functions.get("FIXED$").expect("missing function");
        assert_eq!(func.return_fixed_length, Some(4));
    }

    #[test]
    fn native_supports_udt_record_statements() {
        let program = parse(
            "TYPE PERSON\nNAME AS STRING * 4\nAGE AS INTEGER\nEND TYPE\nDIM rec AS PERSON\nrec.name = \"AB\"\nrec.age = 23\nPUT #1, 1, rec\nGET #1, 1, rec",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_nested_udt_record_statements() {
        let program = parse(
            "TYPE ADDRESS\nSTREET AS STRING * 4\nZIP AS INTEGER\nEND TYPE\nTYPE PERSON\nADDR AS ADDRESS\nAGE AS INTEGER\nEND TYPE\nDIM rec AS PERSON\nrec.addr.street = \"AB\"\nrec.addr.zip = 42\nPUT #1, 1, rec.addr\nGET #1, 1, rec.addr.street\nPUT #1, 7, rec.addr.zip",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_udt_array_record_statements() {
        let program = parse(
            "TYPE ADDRESS\nSTREET AS STRING * 4\nZIP AS INTEGER\nEND TYPE\nTYPE PERSON\nADDR AS ADDRESS\nAGE AS INTEGER\nEND TYPE\nDIM recs(2) AS PERSON\nrecs(1).addr.street = \"AB\"\nrecs(1).addr.zip = 42\nrecs(1).age = 7\nPUT #1, 1, recs(1)\nGET #1, 1, recs(2)\nPUT #1, 7, recs(1).addr\nGET #1, 7, recs(2).addr.street",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_udt_array_subrecord_and_lset_rset_statements() {
        let program = parse(
            "TYPE ADDRESS\nSTREET AS STRING * 4\nZIP AS INTEGER\nEND TYPE\nTYPE PERSON\nADDR AS ADDRESS\nAGE AS INTEGER\nEND TYPE\nDIM recs(2) AS PERSON\nLSET recs(1).addr.street = \"AB\"\nRSET recs(1).addr.street = \"Z\"\nPUT #1, 1, recs(1).addr\nGET #1, 1, recs(2).addr\nGET #1, 7, recs(2).addr.zip",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_print_using_sound_and_shell() {
        let program =
            parse("PRINT USING \"##.##\"; 1.5\nSOUND 440, 5\nPLAY \"CDE\"\nSHELL \"DIR /B\"");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_top_level_goto_gosub_return() {
        let program = parse("GOSUB work\nPRINT \"done\"\nEND\nwork:\nPRINT \"x\"\nRETURN");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_top_level_on_goto_gosub() {
        let program = parse("ON 2 GOSUB first, second\nPRINT \"done\"\nEND\nfirst:\nRETURN\nsecond:\nPRINT \"x\"\nRETURN");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_chain() {
        let program = parse("CHAIN \"next.bas\"");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_for_each() {
        let program = parse("DIM arr(3)\nFOR EACH item IN arr\nPRINT item\nNEXT");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_top_level_timer_statements() {
        let program =
            parse("ON TIMER(1) GOSUB tick\nTIMER ON\nPRINT \"x\"\nEND\ntick:\nTIMER OFF\nRETURN");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_top_level_on_error_resume() {
        let program = parse(
            "ON ERROR GOTO handler\nOPEN \"missing.txt\" FOR INPUT AS #1\nPRINT \"done\"\nEND\nhandler:\nPRINT ERR\nRESUME NEXT",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_control_flow_in_sub_body() {
        let program = parse(
            "DECLARE SUB WORK()\nCALL WORK\nEND\nSUB WORK\nIF 1 THEN GOTO done\nPRINT \"bad\"\ndone:\nPRINT \"ok\"\nEND SUB",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_control_flow_in_function_body() {
        let program = parse(
            "DECLARE FUNCTION F!()\nPRINT F!\nEND\nFUNCTION F!\nON 2 GOTO miss, done\nmiss:\nF! = 1\nEXIT FUNCTION\ndone:\nF! = 2\nEND FUNCTION",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_nested_goto_in_if() {
        let program = parse("IF 1 THEN\n  GOTO done\nEND IF\ndone:\nPRINT \"ok\"");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_nested_on_goto_in_if() {
        let program = parse("IF 1 THEN\n  ON 1 GOTO a\nEND IF\na:\nPRINT \"ok\"");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_nested_timer_in_if() {
        let program = parse("IF 1 THEN\n  ON TIMER(1) GOSUB tick\nEND IF\ntick:\nRETURN");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_nested_on_error_in_if() {
        let program = parse("IF 1 THEN\n  ON ERROR GOTO handler\nEND IF\nhandler:\nRESUME NEXT");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_nested_goto_in_select() {
        let program =
            parse("SELECT CASE 2\nCASE 1\n  PRINT \"bad\"\nCASE 2\n  GOTO done\nEND SELECT\ndone:\nPRINT \"ok\"");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_loop_nested_goto() {
        let program = parse("FOR I = 1 TO 3\n  GOTO done\nNEXT\ndone:\nPRINT \"ok\"");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_loop_nested_on_goto() {
        let program = parse("DO\n  ON 1 GOTO done\nLOOP\ndone:\nPRINT \"ok\"");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_loop_nested_gosub() {
        let program = parse("WHILE -1\n  GOSUB work\nWEND\nwork:\nRETURN");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_supports_loop_nested_on_error() {
        let program = parse("DO\n  ON ERROR GOTO handler\nLOOP\nhandler:\nRESUME NEXT");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.is_empty());
    }

    #[test]
    fn native_still_rejects_expression_get_put_forms() {
        let program = parse("GET #1, 1, 1 + 2\nPUT #1, 1, 1 + 2");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert_eq!(unsupported, vec!["GET", "PUT"]);
    }

    #[test]
    fn native_rejects_multidimensional_array_get_put_targets() {
        let program = parse("GET #1, 1, A(1, 2)\nPUT #1, 1, B$(1, 2)");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert_eq!(unsupported, vec!["GET", "PUT"]);
    }

    #[test]
    fn native_rejects_multidimensional_udt_array_get_put_targets() {
        let program = parse(
            "TYPE PERSON\n AGE AS INTEGER\nEND TYPE\nGET #1, 1, recs(1, 2).age\nPUT #1, 1, recs(1, 2).age",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert_eq!(unsupported, vec!["GET", "PUT"]);
    }
}

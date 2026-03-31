use crate::ast_nodes::{Expression, FunctionCall, Program, Statement};
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
    let known_functions = known_function_names(program);

    collect_unsupported(
        &program.statements,
        backend,
        &known_functions,
        &mut unsupported,
    );

    for sub in program.subs.values() {
        collect_unsupported(&sub.body, backend, &known_functions, &mut unsupported);
    }

    for func in program.functions.values() {
        collect_unsupported(&func.body, backend, &known_functions, &mut unsupported);
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
        unsupported.remove("ON PLAY");
        unsupported.remove("TIMER ON");
        unsupported.remove("TIMER OFF");
        unsupported.remove("TIMER STOP");
        unsupported.remove("PLAY ON");
        unsupported.remove("PLAY OFF");
        unsupported.remove("PLAY STOP");
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
    known_functions: &BTreeSet<String>,
    unsupported: &mut BTreeSet<&'static str>,
) {
    for statement in statements {
        if let Some(name) = unsupported_statement(statement, backend) {
            unsupported.insert(name);
        }
        collect_unsupported_expressions_in_statement(
            statement,
            backend,
            known_functions,
            unsupported,
        );

        match statement {
            Statement::IfBlock {
                then_branch,
                else_branch,
                ..
            } => {
                collect_unsupported(then_branch, backend, known_functions, unsupported);
                if let Some(branch) = else_branch {
                    collect_unsupported(branch, backend, known_functions, unsupported);
                }
            }
            Statement::IfElseBlock {
                then_branch,
                else_ifs,
                else_branch,
                ..
            } => {
                collect_unsupported(then_branch, backend, known_functions, unsupported);
                for (_, branch) in else_ifs {
                    collect_unsupported(branch, backend, known_functions, unsupported);
                }
                if let Some(branch) = else_branch {
                    collect_unsupported(branch, backend, known_functions, unsupported);
                }
            }
            Statement::ForLoop { body, .. }
            | Statement::WhileLoop { body, .. }
            | Statement::DoLoop { body, .. }
            | Statement::ForEach { body, .. } => {
                collect_unsupported(body, backend, known_functions, unsupported)
            }
            Statement::Select { cases, .. } => {
                for (_, branch) in cases {
                    collect_unsupported(branch, backend, known_functions, unsupported);
                }
            }
            _ => {}
        }
    }
}

fn collect_unsupported_expressions_in_statement(
    statement: &Statement,
    backend: Backend,
    known_functions: &BTreeSet<String>,
    unsupported: &mut BTreeSet<&'static str>,
) {
    match statement {
        Statement::Print { expressions, .. }
        | Statement::LPrint { expressions, .. }
        | Statement::Write { expressions }
        | Statement::FunctionCall(FunctionCall {
            args: expressions, ..
        }) => {
            for expr in expressions {
                collect_unsupported_expression(expr, backend, known_functions, unsupported);
            }
        }
        Statement::PrintUsing {
            format,
            expressions,
            ..
        }
        | Statement::LPrintUsing {
            format,
            expressions,
            ..
        } => {
            collect_unsupported_expression(format, backend, known_functions, unsupported);
            for expr in expressions {
                collect_unsupported_expression(expr, backend, known_functions, unsupported);
            }
        }
        Statement::PrintFile {
            file_number,
            expressions,
            ..
        } => {
            collect_unsupported_expression(file_number, backend, known_functions, unsupported);
            for expr in expressions {
                collect_unsupported_expression(expr, backend, known_functions, unsupported);
            }
        }
        Statement::WriteFile {
            file_number,
            expressions,
        } => {
            collect_unsupported_expression(file_number, backend, known_functions, unsupported);
            for expr in expressions {
                collect_unsupported_expression(expr, backend, known_functions, unsupported);
            }
        }
        Statement::PrintFileUsing {
            file_number,
            format,
            expressions,
            ..
        } => {
            collect_unsupported_expression(file_number, backend, known_functions, unsupported);
            collect_unsupported_expression(format, backend, known_functions, unsupported);
            for expr in expressions {
                collect_unsupported_expression(expr, backend, known_functions, unsupported);
            }
        }
        Statement::Assignment { target, value }
        | Statement::LSet { target, value }
        | Statement::RSet { target, value } => {
            collect_unsupported_expression(target, backend, known_functions, unsupported);
            collect_unsupported_expression(value, backend, known_functions, unsupported);
        }
        Statement::IfBlock { condition, .. }
        | Statement::IfElseBlock { condition, .. }
        | Statement::WhileLoop { condition, .. } => {
            collect_unsupported_expression(condition, backend, known_functions, unsupported);
        }
        Statement::ForLoop {
            start, end, step, ..
        } => {
            collect_unsupported_expression(start, backend, known_functions, unsupported);
            collect_unsupported_expression(end, backend, known_functions, unsupported);
            if let Some(step) = step {
                collect_unsupported_expression(step, backend, known_functions, unsupported);
            }
        }
        Statement::Cls { mode } => {
            if let Some(mode) = mode {
                collect_unsupported_expression(mode, backend, known_functions, unsupported);
            }
        }
        Statement::Locate {
            row,
            col,
            cursor,
            start,
            stop,
        } => {
            for expr in [row, col, cursor, start, stop].into_iter().flatten() {
                collect_unsupported_expression(expr, backend, known_functions, unsupported);
            }
        }
        Statement::Width { columns, rows } => {
            collect_unsupported_expression(columns, backend, known_functions, unsupported);
            if let Some(rows) = rows {
                collect_unsupported_expression(rows, backend, known_functions, unsupported);
            }
        }
        Statement::DoLoop { condition, .. } => {
            if let Some(condition) = condition {
                collect_unsupported_expression(condition, backend, known_functions, unsupported);
            }
        }
        Statement::Select { expression, .. }
        | Statement::ForEach {
            array: expression, ..
        }
        | Statement::Screen {
            mode: Some(expression),
        }
        | Statement::Draw {
            commands: expression,
        }
        | Statement::Play { melody: expression }
        | Statement::Color {
            foreground: Some(expression),
            ..
        }
        | Statement::Color {
            background: Some(expression),
            ..
        }
        | Statement::Sleep {
            duration: Some(expression),
        }
        | Statement::Error { code: expression }
        | Statement::ChDir { path: expression }
        | Statement::MkDir { path: expression }
        | Statement::RmDir { path: expression }
        | Statement::Kill {
            filename: expression,
        }
        | Statement::Files {
            pattern: Some(expression),
        }
        | Statement::Chain {
            filename: expression,
            ..
        }
        | Statement::Shell {
            command: Some(expression),
        }
        | Statement::Randomize {
            seed: Some(expression),
        }
        | Statement::Palette {
            attribute: expression,
            ..
        }
        | Statement::Key {
            key_num: expression,
            ..
        } => collect_unsupported_expression(expression, backend, known_functions, unsupported),
        Statement::Open {
            filename,
            file_number,
            record_len,
            ..
        } => {
            collect_unsupported_expression(filename, backend, known_functions, unsupported);
            collect_unsupported_expression(file_number, backend, known_functions, unsupported);
            if let Some(record_len) = record_len {
                collect_unsupported_expression(record_len, backend, known_functions, unsupported);
            }
        }
        Statement::Close { file_numbers } => {
            for file_number in file_numbers {
                collect_unsupported_expression(file_number, backend, known_functions, unsupported);
            }
        }
        Statement::Get {
            file_number,
            record,
            variable,
        }
        | Statement::Put {
            file_number,
            record,
            variable,
        } => {
            collect_unsupported_expression(file_number, backend, known_functions, unsupported);
            if let Some(record) = record {
                collect_unsupported_expression(record, backend, known_functions, unsupported);
            }
            if let Some(variable) = variable {
                collect_unsupported_expression(variable, backend, known_functions, unsupported);
            }
        }
        Statement::GetImage { coords, variable } => {
            collect_unsupported_expression(&coords.0 .0, backend, known_functions, unsupported);
            collect_unsupported_expression(&coords.0 .1, backend, known_functions, unsupported);
            collect_unsupported_expression(&coords.1 .0, backend, known_functions, unsupported);
            collect_unsupported_expression(&coords.1 .1, backend, known_functions, unsupported);
            collect_unsupported_expression(variable, backend, known_functions, unsupported);
        }
        Statement::PutImage {
            coords,
            variable,
            action,
        } => {
            collect_unsupported_expression(&coords.0, backend, known_functions, unsupported);
            collect_unsupported_expression(&coords.1, backend, known_functions, unsupported);
            collect_unsupported_expression(variable, backend, known_functions, unsupported);
            if let Some(action) = action {
                collect_unsupported_expression(action, backend, known_functions, unsupported);
            }
        }
        Statement::Input {
            prompt, variables, ..
        } => {
            if let Some(prompt) = prompt {
                collect_unsupported_expression(prompt, backend, known_functions, unsupported);
            }
            for variable in variables {
                collect_unsupported_expression(variable, backend, known_functions, unsupported);
            }
        }
        Statement::InputFile {
            file_number,
            variables,
        } => {
            collect_unsupported_expression(file_number, backend, known_functions, unsupported);
            for variable in variables {
                collect_unsupported_expression(variable, backend, known_functions, unsupported);
            }
        }
        Statement::LineInput { prompt, variable } => {
            if let Some(prompt) = prompt {
                collect_unsupported_expression(prompt, backend, known_functions, unsupported);
            }
            collect_unsupported_expression(variable, backend, known_functions, unsupported);
        }
        Statement::LineInputFile {
            file_number,
            variable,
        } => {
            collect_unsupported_expression(file_number, backend, known_functions, unsupported);
            collect_unsupported_expression(variable, backend, known_functions, unsupported);
        }
        Statement::Call { args, .. } => {
            for arg in args {
                collect_unsupported_expression(arg, backend, known_functions, unsupported);
            }
        }
        Statement::Dim { variables, .. } | Statement::Redim { variables, .. } => {
            for (_, dimensions) in variables {
                if let Some(dimensions) = dimensions {
                    for dimension in dimensions {
                        if let Some(lower_bound) = &dimension.lower_bound {
                            collect_unsupported_expression(
                                lower_bound,
                                backend,
                                known_functions,
                                unsupported,
                            );
                        }
                        collect_unsupported_expression(
                            &dimension.upper_bound,
                            backend,
                            known_functions,
                            unsupported,
                        );
                    }
                }
            }
        }
        Statement::Read { .. }
        | Statement::Restore { .. }
        | Statement::Data { .. }
        | Statement::Goto { .. }
        | Statement::Gosub { .. }
        | Statement::Return
        | Statement::Label { .. }
        | Statement::LineNumber { .. }
        | Statement::Beep
        | Statement::End
        | Statement::Stop
        | Statement::Clear
        | Statement::Resume
        | Statement::ResumeNext
        | Statement::ViewReset
        | Statement::WindowReset
        | Statement::KeyOn
        | Statement::KeyOff
        | Statement::KeyList
        | Statement::TrOn
        | Statement::TrOff
        | Statement::TimerOn
        | Statement::TimerOff
        | Statement::TimerStop
        | Statement::PlayOn
        | Statement::PlayOff
        | Statement::PlayStop
        | Statement::OptionBase { .. }
        | Statement::Declare { .. }
        | Statement::DefType { .. }
        | Statement::Erase { .. }
        | Statement::Const { .. }
        | Statement::OnError { .. }
        | Statement::OnErrorResumeNext
        | Statement::ResumeLabel { .. }
        | Statement::System
        | Statement::NameFile { .. }
        | Statement::Field { .. }
        | Statement::DefFn { .. }
        | Statement::DefSeg { .. }
        | Statement::Poke { .. }
        | Statement::Wait { .. }
        | Statement::BLoad { .. }
        | Statement::BSave { .. }
        | Statement::Out { .. }
        | Statement::Exit { .. }
        | Statement::Swap { .. }
        | Statement::ViewPrint { .. }
        | Statement::View { .. }
        | Statement::Window { .. }
        | Statement::Pset { .. }
        | Statement::Preset { .. }
        | Statement::Line { .. }
        | Statement::Circle { .. }
        | Statement::Paint { .. }
        | Statement::Sound { .. }
        | Statement::OnTimer { .. }
        | Statement::OnPlay { .. }
        | Statement::OnGotoGosub { .. }
        | Statement::Seek { .. }
        | Statement::Shell { command: None }
        | Statement::Files { pattern: None }
        | Statement::Randomize { seed: None }
        | Statement::Screen { mode: None }
        | Statement::Sleep { duration: None }
        | Statement::Color {
            foreground: None,
            background: None,
        } => {}
    }
}

fn collect_unsupported_expression(
    expr: &Expression,
    backend: Backend,
    _known_functions: &BTreeSet<String>,
    _unsupported: &mut BTreeSet<&'static str>,
) {
    if backend != Backend::Native {
        return;
    }

    match expr {
        Expression::ArrayAccess { indices, .. } => {
            for index in indices {
                collect_unsupported_expression(index, backend, _known_functions, _unsupported);
            }
        }
        Expression::FieldAccess { object, .. } => {
            collect_unsupported_expression(object, backend, _known_functions, _unsupported);
        }
        Expression::BinaryOp { left, right, .. } => {
            collect_unsupported_expression(left, backend, _known_functions, _unsupported);
            collect_unsupported_expression(right, backend, _known_functions, _unsupported);
        }
        Expression::UnaryOp { operand, .. } => {
            collect_unsupported_expression(operand, backend, _known_functions, _unsupported);
        }
        Expression::FunctionCall(func) => {
            for arg in &func.args {
                collect_unsupported_expression(arg, backend, _known_functions, _unsupported);
            }
        }
        Expression::TypeCast { expression, .. } => {
            collect_unsupported_expression(expression, backend, _known_functions, _unsupported);
        }
        Expression::CaseRange { start, end } => {
            collect_unsupported_expression(start, backend, _known_functions, _unsupported);
            collect_unsupported_expression(end, backend, _known_functions, _unsupported);
        }
        Expression::CaseIs { value, .. } => {
            collect_unsupported_expression(value, backend, _known_functions, _unsupported);
        }
        Expression::Literal(_) | Expression::Variable(_) | Expression::CaseElse => {}
    }
}

fn known_function_names(program: &Program) -> BTreeSet<String> {
    let mut names = program
        .functions
        .keys()
        .map(|name| name.to_ascii_uppercase())
        .collect::<BTreeSet<_>>();

    for statement in &program.statements {
        if let Statement::Declare {
            name,
            is_function: true,
            ..
        } = statement
        {
            names.insert(name.to_ascii_uppercase());
        }
    }

    names
}

fn unsupported_statement(statement: &Statement, backend: Backend) -> Option<&'static str> {
    match backend {
        Backend::Vm => None,
        Backend::Native => match statement {
            Statement::PrintUsing { .. } => None,
            Statement::DefType { .. } => Some("DEFxxx default-type coercion"),
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
            Statement::OnPlay { .. } => Some("ON PLAY"),
            Statement::OnGotoGosub { .. } => Some("ON ... GOTO/GOSUB"),
            Statement::TimerOn => Some("TIMER ON"),
            Statement::TimerOff => Some("TIMER OFF"),
            Statement::TimerStop => Some("TIMER STOP"),
            Statement::PlayOn => Some("PLAY ON"),
            Statement::PlayOff => Some("PLAY OFF"),
            Statement::PlayStop => Some("PLAY STOP"),
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
        Expression::ArrayAccess { .. } => true,
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
    use core_types::QType;

    fn parse(source: &str) -> Program {
        let mut parser = Parser::new(source.to_string()).unwrap();
        parser.parse().unwrap()
    }

    use crate::ast_nodes::{Expression, Program, Statement as ProgramStatement};

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
    fn parser_supports_qb64_dim_common_type_prefix() {
        let program = parse("DIM AS LONG i, j\nDIM AS STRING label$, value");
        match &program.statements[0] {
            ProgramStatement::Dim { variables, .. } => {
                assert_eq!(variables.len(), 2);
                assert_eq!(variables[0].0.name, "i");
                assert_eq!(variables[0].0.type_suffix, Some('&'));
                assert_eq!(variables[1].0.name, "j");
                assert_eq!(variables[1].0.type_suffix, Some('&'));
            }
            _ => panic!("expected DIM statement"),
        }
        match &program.statements[1] {
            ProgramStatement::Dim { variables, .. } => {
                assert_eq!(variables.len(), 2);
                assert_eq!(variables[0].0.name, "label$");
                assert_eq!(variables[0].0.type_suffix, Some('$'));
                assert_eq!(variables[1].0.name, "value");
                assert_eq!(variables[1].0.type_suffix, Some('$'));
            }
            _ => panic!("expected DIM statement"),
        }
    }

    #[test]
    fn parser_ignores_qb64_metacommands_and_preserves_line_continuations() {
        let program = parse(
            "$CONSOLE\n$SCREENHIDE\nmessage$ = \"QB\" + _\n           \"64\"\nPRINT message$",
        );
        assert_eq!(program.statements.len(), 2);
        assert!(matches!(
            program.statements[0],
            ProgramStatement::Assignment { .. }
        ));
        assert!(matches!(
            program.statements[1],
            ProgramStatement::Print { .. }
        ));
    }

    #[test]
    fn parser_supports_implicit_sub_calls_without_call_keyword() {
        let program = parse("regid\nemit 1, 2\nflush(3)");
        match &program.statements[0] {
            ProgramStatement::Call { name, args } => {
                assert_eq!(name, "regid");
                assert!(args.is_empty());
            }
            _ => panic!("expected implicit call"),
        }
        match &program.statements[1] {
            ProgramStatement::Call { name, args } => {
                assert_eq!(name, "emit");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("expected implicit call with arguments"),
        }
        match &program.statements[2] {
            ProgramStatement::Call { name, args } => {
                assert_eq!(name, "flush");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("expected parenthesized implicit call"),
        }
    }

    #[test]
    fn parser_supports_single_line_if_with_colon_else_sequences() {
        let program = parse("IF x THEN a = 1: b = 2 ELSE c = 3: d = 4");
        match &program.statements[0] {
            ProgramStatement::IfBlock {
                then_branch,
                else_branch,
                ..
            } => {
                assert_eq!(then_branch.len(), 2);
                assert_eq!(else_branch.as_ref().map(Vec::len), Some(2));
            }
            _ => panic!("expected single-line IF block"),
        }
    }

    #[test]
    fn parser_supports_qb64_type_field_as_prefix_style() {
        let program = parse("TYPE Sample\nAS LONG a, b\nAS _BYTE flag\nEND TYPE");
        let user_type = program.user_types.get("Sample").expect("missing type");
        assert_eq!(user_type.fields.len(), 3);
        assert_eq!(user_type.fields[0].name, "a");
        assert_eq!(user_type.fields[1].name, "b");
        assert_eq!(user_type.fields[2].name, "flag");
    }

    #[test]
    fn parser_supports_binary_get_put_without_explicit_record_argument() {
        let program = parse("GET #1, , A$\nPUT #1, , B(1, 2)");
        match &program.statements[0] {
            ProgramStatement::Get { record, .. } => assert!(record.is_none()),
            _ => panic!("expected GET statement"),
        }
        match &program.statements[1] {
            ProgramStatement::Put { record, .. } => assert!(record.is_none()),
            _ => panic!("expected PUT statement"),
        }
    }

    #[test]
    fn parser_supports_qb64_qualified_variable_names() {
        let program =
            parse("DIM SHARED path.exe$, path.source$\nREAD path.exe$\nERASE path.source$");

        match &program.statements[0] {
            ProgramStatement::Dim { variables, .. } => {
                assert_eq!(variables[0].0.name, "path.exe$");
                assert_eq!(variables[1].0.name, "path.source$");
            }
            _ => panic!("expected DIM statement"),
        }

        match &program.statements[1] {
            ProgramStatement::Read { variables } => {
                assert_eq!(variables[0].name, "path.exe$");
            }
            _ => panic!("expected READ statement"),
        }

        match &program.statements[2] {
            ProgramStatement::Erase { variables } => {
                assert_eq!(variables[0].name, "path.source$");
            }
            _ => panic!("expected ERASE statement"),
        }
    }

    #[test]
    fn parser_supports_qb64_qualified_const_names() {
        let program = parse("CONST idesystem2.w = 20");
        match &program.statements[0] {
            ProgramStatement::Const { name, .. } => assert_eq!(name, "idesystem2.w"),
            _ => panic!("expected CONST statement"),
        }
    }

    #[test]
    fn parser_supports_declare_library_blocks() {
        let program = parse("DECLARE LIBRARY\n    FUNCTION getpid& ()\n    SUB setf (BYVAL hwnd AS _OFFSET)\nEND DECLARE\nx = getpid&");

        match &program.statements[0] {
            ProgramStatement::Declare {
                name,
                is_function,
                params,
                ..
            } => {
                assert_eq!(name, "getpid&");
                assert!(*is_function);
                assert!(params.is_empty());
            }
            _ => panic!("expected FUNCTION declaration"),
        }

        match &program.statements[1] {
            ProgramStatement::Declare {
                name,
                is_function,
                params,
                ..
            } => {
                assert_eq!(name, "setf");
                assert!(!*is_function);
                assert_eq!(params.len(), 1);
            }
            _ => panic!("expected SUB declaration"),
        }
    }

    #[test]
    fn parser_supports_declare_library_blocks_inside_subs() {
        let program = parse(
            "SUB debugmode\nDECLARE LIBRARY\nSUB set_foreground_window (BYVAL hwnd AS _OFFSET)\nEND DECLARE\nset_foreground_window 0\nEND SUB",
        );
        let sub = program.subs.get("debugmode").expect("missing sub");
        assert!(matches!(sub.body[0], ProgramStatement::Declare { .. }));
        assert!(matches!(sub.body[1], ProgramStatement::Call { .. }));
    }

    #[test]
    fn parser_collects_type_definitions_inside_subs() {
        let program = parse(
            "SUB debugmode\nTYPE ui\nAS INTEGER x, y, w, h\nEND TYPE\nDIM Button AS ui\nEND SUB",
        );
        assert!(program.user_types.contains_key("ui"));
        let user_type = program.user_types.get("ui").expect("missing type");
        assert_eq!(user_type.fields.len(), 4);
    }

    #[test]
    fn parser_supports_qb64_shell_modifiers() {
        let program = parse("SHELL _HIDE _DONTWAIT \"cmd /c echo ok\"\nSHELL _HIDE command$");
        assert!(matches!(
            program.statements[0],
            ProgramStatement::Shell { .. }
        ));
        assert!(matches!(
            program.statements[1],
            ProgramStatement::Shell { .. }
        ));
    }

    #[test]
    fn parser_supports_open_lock_and_access_clauses() {
        let program = parse(
            "OPEN \"a\" FOR OUTPUT LOCK WRITE AS #1\nOPEN \"b\" FOR BINARY ACCESS READ WRITE AS #2",
        );

        match &program.statements[0] {
            ProgramStatement::Open { lock, .. } => {
                assert!(matches!(lock, Some(crate::ast_nodes::OpenLock::LockWrite)));
            }
            _ => panic!("expected OPEN statement"),
        }

        match &program.statements[1] {
            ProgramStatement::Open { access, .. } => {
                assert!(matches!(
                    access,
                    Some(crate::ast_nodes::OpenAccess::ReadWrite)
                ));
            }
            _ => panic!("expected OPEN statement"),
        }
    }

    #[test]
    fn parser_supports_qb64_implicit_calls_with_on_off_arguments() {
        let program = parse("_CONSOLE OFF\n_CONSOLE ON");

        match &program.statements[0] {
            ProgramStatement::Call { name, args } => {
                assert_eq!(name, "_CONSOLE");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("expected implicit call"),
        }

        match &program.statements[1] {
            ProgramStatement::Call { name, args } => {
                assert_eq!(name, "_CONSOLE");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("expected implicit call"),
        }
    }

    #[test]
    fn parser_supports_line_input_with_leading_semicolon_prompt() {
        let program = parse("LINE INPUT ; \"COMPILE (.bas)>\", f$");
        match &program.statements[0] {
            ProgramStatement::LineInput { prompt, .. } => assert!(prompt.is_some()),
            _ => panic!("expected LINE INPUT statement"),
        }
    }

    #[test]
    fn parser_supports_qb64_option_forms_and_keyword_named_variables() {
        let program = parse("OPTION _EXPLICIT\noptionbase = 0\nPRINT optionbase");
        assert!(program.option_explicit);
        assert!(matches!(
            program.statements[0],
            ProgramStatement::Assignment { .. }
        ));
        assert!(matches!(
            program.statements[1],
            ProgramStatement::Print { .. }
        ));
    }

    #[test]
    fn parser_marks_local_dims_inside_static_procedures_as_static() {
        let program = parse(
            "SUB Bump STATIC\nDIM count AS INTEGER\nEND SUB\nFUNCTION NextCount# STATIC\nDIM total AS DOUBLE\nEND FUNCTION",
        );
        let sub = program.subs.get("Bump").expect("missing sub");
        let func = program.functions.get("NextCount#").expect("missing function");

        assert!(sub.is_static);
        assert!(func.is_static);
        assert!(matches!(
            sub.body[0],
            ProgramStatement::Dim { is_static: true, .. }
        ));
        assert!(matches!(
            func.body[0],
            ProgramStatement::Dim { is_static: true, .. }
        ));
    }

    #[test]
    fn parser_supports_qb64_cv_type_arguments() {
        let program = parse("x = _CV(_UNSIGNED _INTEGER64, e$)\ny = _CV(_FLOAT, e$)");
        assert!(matches!(
            program.statements[0],
            ProgramStatement::Assignment { .. }
        ));
        assert!(matches!(
            program.statements[1],
            ProgramStatement::Assignment { .. }
        ));
    }

    #[test]
    fn parser_supports_sub_calls_with_parenthesized_first_arguments() {
        let program = parse("_PRINTSTRING (1, 1), menubar$");
        match &program.statements[0] {
            ProgramStatement::Call { name, args } => {
                assert_eq!(name, "_PRINTSTRING");
                assert_eq!(args.len(), 3);
            }
            _ => panic!("expected subroutine call"),
        }
    }

    #[test]
    fn parser_supports_if_goto_shorthand_without_then() {
        let program = parse("IF idemode GOTO ideret4 ELSE GOTO skipide4");
        match &program.statements[0] {
            ProgramStatement::IfBlock {
                then_branch,
                else_branch,
                ..
            } => {
                assert_eq!(then_branch.len(), 1);
                assert_eq!(else_branch.as_ref().map(Vec::len), Some(1));
            }
            _ => panic!("expected IF shorthand"),
        }
    }

    #[test]
    fn parser_supports_select_everycase() {
        let program = parse("SELECT EVERYCASE mode\nCASE 1\nPRINT 1\nEND SELECT");
        assert!(matches!(
            program.statements[0],
            ProgramStatement::Select { .. }
        ));
    }

    #[test]
    fn parser_supports_exit_while() {
        let program = parse("WHILE x < 10\nEXIT WHILE\nWEND");
        match &program.statements[0] {
            ProgramStatement::WhileLoop { body, .. } => {
                assert!(matches!(
                    body[0],
                    ProgramStatement::Exit {
                        exit_type: crate::ast_nodes::ExitType::While
                    }
                ));
            }
            _ => panic!("expected WHILE loop"),
        }
    }

    #[test]
    fn parser_supports_screen_with_omitted_optional_arguments() {
        let program = parse("SCREEN _NEWIMAGE(80, 25, 0), , 0, 0");
        assert!(matches!(
            program.statements[0],
            ProgramStatement::Screen { .. }
        ));
    }

    #[test]
    fn parser_supports_screen_statement_with_omitted_mode() {
        let program = parse("SCREEN , , 3, 0");
        match &program.statements[0] {
            ProgramStatement::Screen { mode } => assert!(mode.is_none()),
            _ => panic!("expected SCREEN statement"),
        }
    }

    #[test]
    fn parser_supports_array_parameters_in_signatures() {
        let program =
            parse("DECLARE SUB pad (p, num() AS STRING)\nSUB pad (p, num() AS STRING)\nEND SUB");
        match &program.statements[0] {
            ProgramStatement::Declare { params, .. } => {
                assert_eq!(params.len(), 2);
                assert!(params[1].indices.is_empty());
            }
            _ => panic!("expected DECLARE statement"),
        }

        let sub = program.subs.get("pad").expect("missing SUB");
        assert_eq!(sub.params.len(), 2);
    }

    #[test]
    fn parser_preserves_fixed_length_string_metadata_on_function_return() {
        let program =
            parse("PRINT FIXED$\nFUNCTION FIXED$() AS STRING * 4\nFIXED$ = \"ABCD\"\nEND FUNCTION");
        let func = program.functions.get("FIXED$").expect("missing function");
        assert_eq!(func.return_fixed_length, Some(4));
    }

    #[test]
    fn parser_preserves_declare_signature_metadata() {
        let program = parse("DECLARE FUNCTION PAD(BYVAL A$ AS STRING * 4, BYREF B%) AS STRING * 6");

        match &program.statements[0] {
            ProgramStatement::Declare {
                name,
                is_function,
                params,
                return_type,
                return_fixed_length,
            } => {
                assert_eq!(name, "PAD");
                assert!(*is_function);
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "A$");
                assert!(params[0].by_val);
                assert_eq!(params[0].fixed_length, Some(4));
                assert_eq!(params[1].name, "B%");
                assert!(!params[1].by_val);
                assert!(matches!(return_type, Some(QType::String(_))));
                assert_eq!(*return_fixed_length, Some(6));
            }
            _ => panic!("expected DECLARE statement"),
        }
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
    fn native_rejects_def_type_programs_for_vm_fallback() {
        let program = parse("DEFINT A-Z\nA = 1.5\nPRINT A");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(unsupported.contains(&"DEFxxx default-type coercion"));
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
    fn native_accepts_multidimensional_array_get_put_targets() {
        let program = parse("GET #1, 1, A(1, 2)\nPUT #1, 1, B$(1, 2)");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(!unsupported.contains(&"GET"));
        assert!(!unsupported.contains(&"PUT"));
    }

    #[test]
    fn native_accepts_multidimensional_udt_array_get_put_targets() {
        let program = parse(
            "TYPE PERSON\n AGE AS INTEGER\nEND TYPE\nGET #1, 1, recs(1, 2).age\nPUT #1, 1, recs(1, 2).age",
        );
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(!unsupported.contains(&"GET"));
        assert!(!unsupported.contains(&"PUT"));
    }

    #[test]
    fn parser_preserves_explicit_and_multidimensional_array_bounds() {
        let program = parse("DIM M(1 TO 2, 3 TO 4)");
        match &program.statements[0] {
            ProgramStatement::Dim { variables, .. } => {
                let dimensions = variables[0].1.as_ref().expect("array dimensions");
                assert_eq!(dimensions.len(), 2);
                assert!(matches!(
                    dimensions[0].lower_bound,
                    Some(Expression::Literal(QType::Integer(1)))
                ));
                assert!(matches!(
                    dimensions[0].upper_bound,
                    Expression::Literal(QType::Integer(2))
                ));
                assert!(matches!(
                    dimensions[1].lower_bound,
                    Some(Expression::Literal(QType::Integer(3)))
                ));
                assert!(matches!(
                    dimensions[1].upper_bound,
                    Expression::Literal(QType::Integer(4))
                ));
            }
            _ => panic!("expected DIM statement"),
        }
    }

    #[test]
    fn native_accepts_option_base_and_advanced_array_bounds() {
        let program = parse("OPTION BASE 1\nDIM M(1 TO 2, 3 TO 4)");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(!unsupported.contains(&"OPTION BASE"));
        assert!(!unsupported.contains(&"advanced array bounds"));
    }

    #[test]
    fn native_accepts_nonfirst_lbound_ubound_dimension_argument() {
        let program = parse("DIM A(10)\nPRINT UBOUND(A, 2)");
        let unsupported = unsupported_statements(&program, Backend::Native);
        assert!(!unsupported.contains(&"LBOUND/UBOUND dimension argument"));
    }
}

use syntax_tree::ast_nodes::{Program, Statement};

pub fn has_graphics_or_sound(program: &Program) -> bool {
    // Check main statements
    if program.statements.iter().any(check_statement) {
        return true;
    }

    // Check subroutines
    if program
        .subs
        .values()
        .any(|sub| sub.body.iter().any(check_statement))
    {
        return true;
    }

    // Check functions
    if program
        .functions
        .values()
        .any(|func| func.body.iter().any(check_statement))
    {
        return true;
    }

    false
}

fn check_statement(stmt: &Statement) -> bool {
    match stmt {
        Statement::Screen { .. }
        | Statement::Pset { .. }
        | Statement::Preset { .. }
        | Statement::Line { .. }
        | Statement::Circle { .. }
        | Statement::Paint { .. }
        | Statement::GetImage { .. }
        | Statement::PutImage { .. }
        | Statement::Sound { .. }
        | Statement::Play { .. }
        | Statement::Beep
        | Statement::View { .. }
        | Statement::ViewReset
        | Statement::Window { .. }
        | Statement::WindowReset
        | Statement::Draw { .. }
        | Statement::Palette { .. } => true,

        Statement::Assignment { target, value } => {
            check_expression(target) || check_expression(value)
        }
        Statement::Print { expressions, .. } | Statement::Write { expressions } => {
            expressions.iter().any(check_expression)
        }
        Statement::Input {
            prompt, variables, ..
        } => {
            prompt.as_ref().is_some_and(check_expression) || variables.iter().any(check_expression)
        }
        Statement::Open {
            filename,
            file_number,
            record_len,
            ..
        } => {
            check_expression(filename)
                || check_expression(file_number)
                || record_len.as_ref().is_some_and(check_expression)
        }
        Statement::Close { file_numbers } => file_numbers.iter().any(check_expression),
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
            check_expression(file_number)
                || record.as_ref().is_some_and(check_expression)
                || variable.as_ref().is_some_and(check_expression)
        }
        Statement::IfBlock {
            condition,
            then_branch,
            else_branch,
        } => {
            check_expression(condition)
                || then_branch.iter().any(check_statement)
                || else_branch
                    .as_ref()
                    .is_some_and(|b| b.iter().any(check_statement))
        }
        Statement::IfElseBlock {
            condition,
            then_branch,
            else_ifs,
            else_branch,
            ..
        } => {
            check_expression(condition)
                || then_branch.iter().any(check_statement)
                || else_ifs
                    .iter()
                    .any(|(expr, b)| check_expression(expr) || b.iter().any(check_statement))
                || else_branch
                    .as_ref()
                    .is_some_and(|b| b.iter().any(check_statement))
        }
        Statement::ForLoop {
            start,
            end,
            step,
            body,
            ..
        } => {
            check_expression(start)
                || check_expression(end)
                || step.as_ref().is_some_and(check_expression)
                || body.iter().any(check_statement)
        }
        Statement::WhileLoop { condition, body } => {
            check_expression(condition) || body.iter().any(check_statement)
        }
        Statement::DoLoop {
            condition, body, ..
        } => condition.as_ref().is_some_and(check_expression) || body.iter().any(check_statement),
        Statement::ForEach { array, body, .. } => {
            check_expression(array) || body.iter().any(check_statement)
        }
        Statement::Call { args, .. } => args.iter().any(check_expression),
        Statement::FunctionCall(func) => check_function_call(func),
        Statement::Dim { variables, .. } | Statement::Redim { variables, .. } => {
            variables.iter().any(|(var, dimensions)| {
                check_variable(var)
                    || dimensions.as_ref().is_some_and(|dimensions| {
                        dimensions.iter().any(|dimension| {
                            dimension.lower_bound.as_ref().is_some_and(check_expression)
                                || check_expression(&dimension.upper_bound)
                        })
                    })
            })
        }
        Statement::Const { value, .. } => check_expression(value),
        Statement::Randomize { seed } => seed.as_ref().is_some_and(check_expression),
        Statement::Cls { mode } => mode.as_ref().is_some_and(check_expression),
        Statement::Locate {
            row,
            col,
            cursor,
            start,
            stop,
        } => {
            row.as_ref().is_some_and(check_expression)
                || col.as_ref().is_some_and(check_expression)
                || cursor.as_ref().is_some_and(check_expression)
                || start.as_ref().is_some_and(check_expression)
                || stop.as_ref().is_some_and(check_expression)
        }
        Statement::Error { code } => check_expression(code),
        Statement::Chain { filename, delete } => {
            check_expression(filename) || delete.as_ref().is_some_and(check_expression)
        }
        Statement::Shell { command } => command.as_ref().is_some_and(check_expression),
        Statement::Swap { var1, var2 } => check_expression(var1) || check_expression(var2),
        Statement::Sleep { duration } => duration.as_ref().is_some_and(check_expression),
        Statement::Kill { filename }
        | Statement::ChDir { path: filename }
        | Statement::MkDir { path: filename }
        | Statement::RmDir { path: filename } => check_expression(filename),
        Statement::NameFile { old_name, new_name } => {
            check_expression(old_name) || check_expression(new_name)
        }
        Statement::Files { pattern } => pattern.as_ref().is_some_and(check_expression),
        Statement::Field {
            file_number,
            fields,
        } => {
            check_expression(file_number)
                || fields
                    .iter()
                    .any(|(width, field)| check_expression(width) || check_expression(field))
        }
        Statement::LSet { target, value } | Statement::RSet { target, value } => {
            check_expression(target) || check_expression(value)
        }
        Statement::Color {
            foreground,
            background,
        } => {
            foreground.as_ref().is_some_and(check_expression)
                || background.as_ref().is_some_and(check_expression)
        }
        Statement::Width { columns, rows } => {
            check_expression(columns) || rows.as_ref().is_some_and(check_expression)
        }
        Statement::ViewPrint { top, bottom } => {
            top.as_ref().is_some_and(check_expression)
                || bottom.as_ref().is_some_and(check_expression)
        }
        Statement::Key {
            key_num,
            key_string,
        } => check_expression(key_num) || check_expression(key_string),
        Statement::OnTimer { interval, .. } => check_expression(interval),
        Statement::OnPlay { queue_limit, .. } => check_expression(queue_limit),
        Statement::OnGotoGosub { expression, .. } => check_expression(expression),
        Statement::InputFile {
            file_number,
            variables,
        } => check_expression(file_number) || variables.iter().any(check_expression),
        Statement::LineInput { prompt, variable } => {
            prompt.as_ref().is_some_and(check_expression) || check_expression(variable)
        }
        Statement::LineInputFile {
            file_number,
            variable,
        } => check_expression(file_number) || check_expression(variable),
        Statement::WriteFile {
            file_number,
            expressions,
        } => check_expression(file_number) || expressions.iter().any(check_expression),
        Statement::Seek {
            file_number,
            position,
        } => check_expression(file_number) || check_expression(position),
        Statement::DefFn { body, .. } => check_expression(body),
        Statement::DefSeg { segment } => {
            segment.as_ref().is_some_and(|expr| check_expression(expr))
        }
        Statement::Poke { address, value } => check_expression(address) || check_expression(value),
        Statement::Wait {
            address,
            and_mask,
            xor_mask,
        } => {
            check_expression(address)
                || check_expression(and_mask)
                || xor_mask.as_ref().is_some_and(check_expression)
        }
        Statement::BLoad { filename, offset } => {
            check_expression(filename) || offset.as_ref().is_some_and(check_expression)
        }
        Statement::BSave {
            filename,
            offset,
            length,
        } => check_expression(filename) || check_expression(offset) || check_expression(length),
        Statement::Out { port, value } => check_expression(port) || check_expression(value),
        Statement::Select { expression, cases } => {
            check_expression(expression)
                || cases.iter().any(|(expr, stmts)| {
                    check_expression(expr) || stmts.iter().any(check_statement)
                })
        }
        _ => false,
    }
}

fn check_variable(var: &syntax_tree::ast_nodes::Variable) -> bool {
    var.indices.iter().any(check_expression)
}

fn check_function_call(func: &syntax_tree::ast_nodes::FunctionCall) -> bool {
    matches!(func.name.to_ascii_uppercase().as_str(), "POINT" | "PMAP")
        || func.args.iter().any(check_expression)
}

fn check_expression(expr: &syntax_tree::ast_nodes::Expression) -> bool {
    use syntax_tree::ast_nodes::Expression;

    match expr {
        Expression::Literal(_) | Expression::CaseElse => false,
        Expression::Variable(var) => check_variable(var),
        Expression::ArrayAccess { name, indices, .. } => {
            matches!(name.to_ascii_uppercase().as_str(), "POINT" | "PMAP")
                || indices.iter().any(check_expression)
        }
        Expression::FieldAccess { object, .. } => check_expression(object),
        Expression::FunctionCall(func) => check_function_call(func),
        Expression::TypeCast { expression, .. } => check_expression(expression),
        Expression::BinaryOp { left, right, .. } => {
            check_expression(left) || check_expression(right)
        }
        Expression::UnaryOp { operand, .. } => check_expression(operand),
        Expression::CaseRange { start, end } => check_expression(start) || check_expression(end),
        Expression::CaseIs { value, .. } => check_expression(value),
    }
}

#[cfg(test)]
mod tests {
    use super::has_graphics_or_sound;
    use syntax_tree::Parser;

    fn parse(source: &str) -> syntax_tree::ast_nodes::Program {
        let mut parser = Parser::new(source.to_string()).unwrap();
        parser.parse().unwrap()
    }

    #[test]
    fn text_mode_statements_do_not_force_graphics_pipeline() {
        let program = parse("CLS\nCOLOR 7, 0\nLOCATE 1, 1\nPRINT \"ok\"");
        assert!(!has_graphics_or_sound(&program));
    }

    #[test]
    fn actual_graphics_statements_still_enable_graphics_pipeline() {
        let program = parse("SCREEN 13\nPSET (1, 1), 15\nDRAW \"R5\"");
        assert!(has_graphics_or_sound(&program));
    }

    #[test]
    fn graphics_builtin_expressions_enable_graphics_pipeline() {
        let program = parse("x = POINT(1, 1)\nPRINT PMAP(0, 0)");
        assert!(has_graphics_or_sound(&program));
    }
}

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

        Statement::IfBlock {
            then_branch,
            else_branch,
            ..
        } => {
            then_branch.iter().any(check_statement)
                || else_branch
                    .as_ref()
                    .is_some_and(|b| b.iter().any(check_statement))
        }
        Statement::IfElseBlock {
            then_branch,
            else_ifs,
            else_branch,
            ..
        } => {
            then_branch.iter().any(check_statement)
                || else_ifs.iter().any(|(_, b)| b.iter().any(check_statement))
                || else_branch
                    .as_ref()
                    .is_some_and(|b| b.iter().any(check_statement))
        }
        Statement::ForLoop { body, .. } => body.iter().any(check_statement),
        Statement::WhileLoop { body, .. } => body.iter().any(check_statement),
        Statement::DoLoop { body, .. } => body.iter().any(check_statement),
        Statement::ForEach { body, .. } => body.iter().any(check_statement),
        Statement::Select { cases, .. } => cases
            .iter()
            .any(|(_, stmts)| stmts.iter().any(check_statement)),
        Statement::DefFn { body: _, .. } => {
            // DefFn body is an expression, but if it contained statements...
            // Actually DefFn body is usually an expression in QBasic, but here struct has body: Expression.
            // Wait, looking at Statement::DefFn in ast_nodes.rs, it has `body: Expression`.
            // Expressions don't contain statements (usually), so no graphics there.
            false
        }
        _ => false,
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
}

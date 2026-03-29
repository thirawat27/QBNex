use crate::ast_nodes::{BinaryOp, Expression, Program, Statement, UnaryOp, Variable};
use chumsky::prelude::*;
use core_types::{QError, QResult, QType};

type ParseError = Simple<char>;

fn inline_ws() -> impl Parser<char, (), Error = ParseError> + Clone {
    filter(|ch: &char| matches!(ch, ' ' | '\t' | '\r'))
        .repeated()
        .ignored()
}

fn keyword(word: &'static str) -> impl Parser<char, (), Error = ParseError> + Clone {
    text::ident().try_map(move |ident: String, span| {
        if ident.eq_ignore_ascii_case(word) {
            Ok(())
        } else {
            Err(Simple::custom(span, format!("expected keyword {word}")))
        }
    })
}

fn identifier() -> impl Parser<char, String, Error = ParseError> + Clone {
    let head = filter(|ch: &char| ch.is_ascii_alphabetic() || *ch == '_');
    let tail = filter(|ch: &char| {
        ch.is_ascii_alphanumeric() || matches!(*ch, '_' | '$' | '%' | '!' | '#' | '&')
    })
    .repeated()
    .collect::<String>();

    head.then(tail).map(|(first, rest)| {
        let mut name = String::new();
        name.push(first);
        name.push_str(&rest);
        name
    })
}

fn string_literal() -> impl Parser<char, Expression, Error = ParseError> + Clone {
    just('"')
        .ignore_then(
            filter(|ch: &char| *ch != '"' && *ch != '\n' && *ch != '\r')
                .repeated()
                .collect::<String>(),
        )
        .then_ignore(just('"'))
        .map(|value| Expression::Literal(QType::String(value)))
}

fn number_literal() -> impl Parser<char, Expression, Error = ParseError> + Clone {
    let digits = filter(|ch: &char| ch.is_ascii_digit())
        .repeated()
        .at_least(1)
        .collect::<String>();

    digits
        .clone()
        .then(just('.').then(digits.clone()).or_not())
        .map(|(whole, fraction)| {
            if let Some((dot, frac)) = fraction {
                let mut text = whole;
                text.push(dot);
                text.push_str(&frac);
                Expression::Literal(QType::Double(text.parse::<f64>().unwrap_or_default()))
            } else if let Ok(value) = whole.parse::<i16>() {
                Expression::Literal(QType::Integer(value))
            } else if let Ok(value) = whole.parse::<i32>() {
                Expression::Literal(QType::Long(value))
            } else {
                Expression::Literal(QType::Double(whole.parse::<f64>().unwrap_or_default()))
            }
        })
}

fn expression() -> impl Parser<char, Expression, Error = ParseError> + Clone {
    recursive(|expr| {
        let atom = choice((
            number_literal(),
            string_literal(),
            identifier().map(|name| Expression::Variable(Variable::new(name))),
            expr.clone().delimited_by(
                just('(').padded_by(inline_ws()),
                just(')').padded_by(inline_ws()),
            ),
        ))
        .padded_by(inline_ws());

        let unary = just('-')
            .padded_by(inline_ws())
            .repeated()
            .then(atom)
            .foldr(|_, operand| Expression::UnaryOp {
                op: UnaryOp::Negate,
                operand: Box::new(operand),
            });

        let product = unary
            .clone()
            .then(
                choice((
                    just('*').to(BinaryOp::Multiply),
                    just('/').to(BinaryOp::Divide),
                ))
                .padded_by(inline_ws())
                .then(unary.clone())
                .repeated(),
            )
            .foldl(|left, (op, right)| Expression::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            });

        product
            .clone()
            .then(
                choice((
                    just('+').to(BinaryOp::Add),
                    just('-').to(BinaryOp::Subtract),
                ))
                .padded_by(inline_ws())
                .then(product)
                .repeated(),
            )
            .foldl(|left, (op, right)| Expression::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            })
    })
}

fn separator() -> impl Parser<char, (), Error = ParseError> + Clone {
    choice((just(':'), just('\n')))
        .padded_by(inline_ws())
        .repeated()
        .at_least(1)
        .ignored()
}

fn print_statement() -> impl Parser<char, Statement, Error = ParseError> + Clone {
    keyword("PRINT")
        .padded_by(inline_ws())
        .ignore_then(expression().or_not())
        .map(|expr| {
            let expressions = expr.into_iter().collect::<Vec<_>>();
            let separators = vec![None; expressions.len()];
            Statement::Print {
                expressions,
                separators,
                newline: true,
            }
        })
}

fn assignment_statement() -> impl Parser<char, Statement, Error = ParseError> + Clone {
    keyword("LET")
        .padded_by(inline_ws())
        .or_not()
        .ignore_then(
            identifier()
                .padded_by(inline_ws())
                .then_ignore(just('=').padded_by(inline_ws()))
                .then(expression()),
        )
        .map(|(name, value)| Statement::Assignment {
            target: Expression::Variable(Variable::new(name)),
            value,
        })
}

fn end_statement() -> impl Parser<char, Statement, Error = ParseError> + Clone {
    keyword("END").padded_by(inline_ws()).to(Statement::End)
}

fn byte_offset(input: &str, char_index: usize) -> usize {
    input
        .char_indices()
        .nth(char_index)
        .map(|(offset, _)| offset)
        .unwrap_or(input.len())
}

fn error_to_qerror(input: &str, error: ParseError) -> QError {
    let span = error.span();
    let start = byte_offset(input, span.start);
    let end = byte_offset(input, span.end);
    let found = match error.found() {
        Some(found) => format!("unexpected token '{found}'"),
        None => "unexpected end of input".to_string(),
    };

    QError::syntax_at(
        format!(
            "experimental chumsky frontend could not parse this program: {found}. Supported subset currently includes PRINT, LET/assignment, END, variables, strings, and basic arithmetic"
        ),
        start,
        end.saturating_sub(start).max(1),
    )
}

pub fn parse_program(input: String) -> QResult<Program> {
    let stmt = choice((print_statement(), assignment_statement(), end_statement()));
    let parser = inline_ws()
        .ignore_then(stmt.separated_by(separator()).allow_trailing())
        .then_ignore(inline_ws())
        .then_ignore(end());

    match parser.parse(input.as_str()) {
        Ok(statements) => {
            let mut program = Program::new();
            program.statements = statements;
            Ok(program)
        }
        Err(errors) => Err(error_to_qerror(
            &input,
            errors
                .into_iter()
                .next()
                .unwrap_or_else(|| Simple::custom(0..0, "unknown chumsky parse error".to_string())),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_program;

    #[test]
    fn parses_simple_programs_with_assignments_and_print() {
        let program = parse_program("LET total = 40 + 2\nPRINT total\nEND\n".to_string()).unwrap();
        assert_eq!(program.statements.len(), 3);
    }

    #[test]
    fn reports_source_spans_for_invalid_programs() {
        let error = parse_program("PRINT @\n".to_string()).unwrap_err();
        assert!(error.source_span().is_some());
    }
}

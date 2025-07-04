use super::token::Token;
use crate::entry::Entry;
use crate::prelude::*;

use time::OffsetDateTime;

#[cfg(test)]
use crate::{
    app::App,
    datecalc::parse::{parse_date, parse_exp},
};

/// Iterate over stack and calculate the result.
pub fn eval(
    queue: &[Token],
    ts: OffsetDateTime,
    stack: &mut Vec<Token>,
    issue: &Entry,
) -> Result<Token> {
    use Token::*;

    for tok in queue {
        let res = match tok {
            Duration(_) | Date(_) | Bool(_) | Regex(_) | String(_) => tok.clone(),
            Reference(field) => field.as_token(issue),
            Add(false) => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.sum(rhs)?,
                _ => bail!("'+' operator haven't got enough arguments"),
            },
            Add(true) => match stack.pop() {
                Some(rhs) => rhs,
                _ => bail!("Unary '+' operator haven't got enough arguments"),
            },
            Sub(false) => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.sub(rhs)?,
                _ => bail!("'-' operator haven't got enough arguments"),
            },
            Sub(true) => match stack.pop() {
                Some(rhs) => rhs.neg()?,
                _ => bail!("Unary '-' operator haven't got enough arguments"),
            },
            Mul => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.mul(rhs)?,
                _ => bail!("'*' operator haven't got enough arguments"),
            },
            Div => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.div(rhs)?,
                _ => bail!("'/' operator haven't got enough arguments"),
            },
            Mod => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.modulo(rhs)?,
                _ => bail!("'%' operator haven't got enough arguments"),
            },
            At => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.at(rhs, ts)?,
                _ => bail!("'@' operator haven't got enough arguments"),
            },
            And => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.and(rhs),
                _ => bail!("'and' ('&&') operator haven't got enough arguments"),
            },
            Or => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.or(rhs),
                _ => bail!("'or' ('||') operator haven't got enough arguments"),
            },
            Func(funcref) => funcref.exec(stack, issue)?,
            Greater(false) => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.greater(rhs, ts)?,
                _ => bail!("'>' operator haven't got enough arguments"),
            },
            GreaterEq(false) => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.greater_eq(rhs, ts)?,
                _ => bail!("'>=' operator haven't got enough arguments"),
            },
            Less(false) => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.less(rhs, ts)?,
                _ => bail!("'<' operator haven't got enough arguments"),
            },
            LessEq(false) => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.less_eq(rhs, ts)?,
                _ => bail!("'<' operator haven't got enough arguments"),
            },

            Not => match stack.pop() {
                Some(val) => val.not(),
                _ => bail!("'not' ('!') operator haven't got the argument"),
            },

            Less(true) => Token::unary_less(stack, ts)?,
            LessEq(true) => Token::unary_less_eq(stack, ts)?,
            Greater(true) => Token::unary_greater(stack, ts)?,
            GreaterEq(true) => Token::unary_greater_eq(stack, ts)?,

            Eq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.eq(rhs, issue)?,
                _ => bail!("'eq' ('==') operator haven't got enough arguments"),
            },
            NotEq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.not_eq(rhs, issue)?,
                _ => bail!("'eq' ('!=') operator haven't got enough arguments"),
            },
            Contains => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.contains(&rhs, issue, ts)?,
                _ => bail!("'has' (':') operator haven't got enough arguments"),
            },
            In => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => rhs.contains(&lhs, issue, ts)?,
                _ => bail!("'in' operator haven't got enough arguments"),
            },

            If => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.r#if(rhs),
                _ => bail!("'if' operator haven't got enough arguments"),
            },
            Else => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => lhs.r#else(rhs),
                _ => bail!("'else' operator haven't got enough arguments"),
            },
            LParen | RParen => {
                panic!()
            }
        };
        stack.push(res);
    }

    let last = stack.last();
    last.context("Expression didn't produced any result")
        .cloned()
}

#[test]
fn full_exp_parsing() {
    let (app, entry) = (&App::default(), &Entry::default());
    assert_eq!(parse_date("1.5h+2h", app, entry).unwrap(), Some(12600));
    assert_eq!(
        parse_date("1s+2s*3", app, entry).unwrap(),
        Some(7),
        "op precedence"
    );
}

#[test]
fn relative_dates() {
    let (app, issue) = (&App::default(), &Entry::default());
    let monday = parse_date("monday", app, issue).unwrap().unwrap();
    let tuesday = parse_date("tuesday", app, issue).unwrap().unwrap();
    assert_eq!(tuesday - monday, 86400);
}

#[test]
fn unexpected_tokens() {
    let (app, issue) = (&App::default(), &Entry::default());
    assert_eq!(parse_date("1d+", app, issue).is_err(), true);
    assert_eq!(parse_date("1d2d", app, issue).is_err(), true);
    assert_eq!(parse_date("1d(2d)", app, issue).is_err(), true);
    assert_eq!(parse_date("(", app, issue).is_err(), true);
    assert_eq!(parse_date(")", app, issue).is_err(), true);

    assert_eq!(parse_date("sqrt 5", app, issue).is_err(), true);
    assert_eq!(parse_date("4 5", app, issue).is_err(), true);
    assert_eq!(parse_date("4 ( 5 )", app, issue).is_err(), true);
}

#[cfg(test)]
fn eval_test(expr: &str) -> Result<Token> {
    let (app, issue) = (&App::default(), &Entry::default());

    let mut output = Vec::new();
    let mut op_stack = Vec::new();
    let local = app.local_time().unwrap();
    parse_exp(expr, local, &mut output).unwrap();

    eval(&output, local, &mut op_stack, &issue)
}

#[test]
fn comparisons() {
    let res = eval_test("1d < 2d");
    assert!(matches!(res, Ok(Token::Bool(true))));

    let res = eval_test("today < tomorrow");
    assert!(matches!(res, Ok(Token::Bool(true))));

    let res = eval_test("tomorrow < 2d");
    assert!(matches!(res, Ok(Token::Bool(true))));

    let res = eval_test("2d < tomorrow");
    assert!(matches!(res, Ok(Token::Bool(false))));

    let res = eval_test("2d > 1d");
    assert!(matches!(res, Ok(Token::Bool(true))));

    let res = eval_test("tomorrow > today");
    assert!(matches!(res, Ok(Token::Bool(true))));
}

/// Check ':' op precedence over unary '-'
#[test]
fn op_precedence() {
    let res = eval_test("1d:-1d");
    assert!(matches!(res, Ok(Token::Bool(false))));

    let res = eval_test("1d:+1d");
    assert!(matches!(res, Ok(Token::Bool(true))));

    let res = eval_test("-1d:-1d");
    assert!(matches!(res, Ok(Token::Bool(true))));
}

/// Freeze some operations behaviour.
///
/// Logical operators behaviour is similar to Ruby - only 'false'
/// is considered to be 'false', non-boolean values are treated as 'true'.
#[test]
fn op_behaviour() {
    let res = eval_test("0 and true");
    assert!(
        matches!(res, Ok(Token::Bool(true))),
        "Zero numeric values are interpreted as 'true'"
    );

    let res = eval_test("'' and true");
    assert!(
        matches!(res, Ok(Token::Bool(true))),
        "String values are interpreted as 'true'"
    );

    let res = eval_test("due and true");
    assert!(
        matches!(res, Ok(Token::Bool(false))),
        "Null values are interpreted as 'false'"
    );

    // TODO: P2: add test for referenced fields

    let res = eval_test("'' and false or true");
    assert!(
        matches!(res, Ok(Token::Bool(true))),
        "And/or doesn't always work as ternary operator"
    );

    let res = eval_test("false if true else true");
    assert!(
        matches!(res, Ok(Token::Bool(false))),
        "If/else works as ternary operator"
    );

    let res = eval_test("(100 if false) and true");
    assert!(
        matches!(res, Ok(Token::Else)),
        "'If' operator result is treated as 'false' (and propagated next)"
    );

    let res = eval_test("not now");
    assert!(
        matches!(res, Ok(Token::Bool(false))),
        "'!' converts date into 'false'"
    );

    let res = eval_test("not 0");
    assert!(
        matches!(res, Ok(Token::Bool(false))),
        "'!' converts number into 'false'"
    );

    let res = eval_test("not 'value'");
    assert!(
        matches!(res, Ok(Token::Bool(false))),
        "'!' converts string into 'false'"
    );

    let res = eval_test("not tag");
    assert!(
        matches!(res, Ok(Token::Bool(true))),
        "'!' converts empty tag list into 'true'"
    );
}

#[test]
fn functions() {
    let res = eval_test("sqrt(4)");
    assert!(matches!(res, Ok(Token::Duration(2.))));

    let res = eval_test("len(tag)");
    assert!(matches!(res, Ok(Token::Duration(0.))));

    let res = eval_test("len(tag) == 0");
    assert!(matches!(res, Ok(Token::Bool(true))));

    let res = eval_test("+2-sqrt(4)*15");
    assert!(matches!(res, Ok(Token::Duration(-28.))));
}

#[test]
fn date_comparisons() {
    let res = eval_test("(today+10h):(today+12h)");
    assert!(matches!(res, Ok(Token::Bool(true))));

    let res = eval_test("(today+10h):(today+25h)");
    assert!(matches!(res, Ok(Token::Bool(false))));

    let res = eval_test("1h:2h");
    assert!(matches!(res, Ok(Token::Bool(true))));

    let res = eval_test("1h:25h");
    assert!(matches!(res, Ok(Token::Bool(false))));
}

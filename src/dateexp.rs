use std::ops::Range;
use std::rc::Rc;

use logos::Logos as _;
use time::OffsetDateTime;

use crate::entry::{Entry, FieldRef};
use crate::{app::App, prelude::*, token::Token};

/// Parse date expression and produce the timestamp.
/// Convert the incoming token stream using shunting yard algorithm into RPN and eval it.
pub fn parse_date(input: &str, app: &App, issue: &Entry) -> Result<Option<i64>> {
    let local = app.local_time()?;

    let mut exp = Vec::<Token>::new();
    parse_exp(input, local, &mut exp)?;

    let mut arg_stack = Vec::<Token>::new();
    let res = eval(&exp, local, &mut arg_stack, issue)?;

    match res {
        Token::Date(date) => Ok(Some(date)),
        Token::Duration(rel) => Ok(Some(app.ts + rel as i64)),
        Token::Bool(false) => Ok(None),
        Token::Bool(true) => bail!("Date expression returned 'true'"),
        _ => bail!(
            "Date expression should return date, duration or 'false' (got {})",
            res.ttype()
        ),
    }
}

/// Parse and append to filter expression. If experssion wasn't empty,
/// add new conditions behind '&&' operator.
///
/// If only one token was found in expression, check if it's string or regex,
/// and add comparison to the title.
pub fn parse_filter(input: &str, app: &App, output: &mut Vec<Token>) -> Result<()> {
    let before = output.len();
    let res = parse_local_exp(input, app, output);
    let delta = output.len() - before;

    if delta == 0 {
        return res;
    } else if delta == 1 {
        match output.last() {
            Some(Token::String(_)) | Some(Token::Regex(_)) => {
                output.insert(output.len() - 1, Token::Reference(FieldRef::Title));
                output.push(Token::FuzzyEq);
            }
            _ => {}
        }
    }

    if before != 0 {
        output.push(Token::And);
    }

    res
}

/// Pass app local timestamp and parse the expresion.
pub fn parse_local_exp(input: &str, app: &App, output: &mut Vec<Token>) -> Result<()> {
    let local = app.local_time()?;
    parse_exp(input, local, output)
}

/// Produce parsed ASP tree ready for evaluation from the input.
pub fn parse_exp(mut input: &str, ts: OffsetDateTime, output: &mut Vec<Token>) -> Result<()> {
    use Token::*;

    let initial = output.len();
    let mut op_stack = Vec::<Token>::new();
    let mut mode = Mode::Arg;

    'outer: loop {
        let lexer = Token::lexer_with_extras(input, ts);
        for (tok, Range { start, end }) in lexer.spanned() {
            let tok =
                tok.with_context(|| format!("Unable to process token at position {}", start))?;

            match tok {
                Duration(_) | Date(_) | Bool(_) | Regex(_) | String(_) | Reference(_) => {
                    if !mode.expects_arg() {
                        bail!(
                            "Expected {}, got argument at position {}",
                            mode.expected(),
                            start
                        );
                    }
                    output.push(tok);
                    mode = Mode::Op;
                }
                Func(_) => {
                    op_stack.push(tok);
                    mode = Mode::FnParen;
                }
                Add(_) | Sub(_) | Mul | Div | Mod | At | Eq | FuzzyEq | Less | LessEq | Greater
                | GreaterEq | NotEq | And | Or | Not => {
                    let (prec, left_assoc) = tok.prec_and_assoc();
                    let (tok, left_assoc) = if mode.expects_arg() {
                        if let Div = tok {
                            let (regex, remainder) = parse_regex(&input[end..])?;
                            output.push(Regex(regex));
                            mode = Mode::Op;
                            input = remainder;
                            continue 'outer;
                        }

                        let (tok, left_assoc) = tok.to_unary();
                        if left_assoc {
                            bail!(
                                "Expected {}, got operator '{}' at position {}",
                                mode.expected(),
                                &input[start..end],
                                start
                            );
                        }
                        (tok, left_assoc)
                    } else {
                        (tok, left_assoc)
                    };

                    while let Some(top) = op_stack.pop_if(|top| {
                        if let LParen = top {
                            return false;
                        }
                        let (top_prec, _) = top.prec_and_assoc();
                        (top_prec > prec) || (top_prec == prec && left_assoc)
                    }) {
                        output.push(top)
                    }
                    op_stack.push(tok);
                    mode = Mode::Arg;
                }
                LParen => {
                    if mode.expects_paren() {
                        op_stack.push(tok);
                        mode = Mode::Arg;
                        continue;
                    }
                    bail!(
                        "Expected {}, got '(' at position {}",
                        mode.expected(),
                        start
                    );
                }
                RParen => {
                    if !tilt(&mut op_stack, output) {
                        bail!("Mismatched ')' at position {}", end);
                    }
                    op_stack.pop_if(|top| {
                        if matches!(top, Func(_)) {
                            output.push(top.clone());
                            return true;
                        }
                        false
                    });
                    mode = Mode::Op;
                }
            }
        }
        break;
    }
    if tilt(&mut op_stack, output) {
        bail!("Mismatched opening bracket");
    }

    if output.len() != initial && !mode.expects_op() {
        bail!("Dangling operator at the end of expression");
    }

    Ok(())
}

/// Token parser expected token state.
enum Mode {
    Arg,
    Op,
    FnParen,
}

impl Mode {
    #[inline]
    fn expects_op(&self) -> bool {
        matches!(self, Self::Op)
    }

    #[inline]
    fn expects_arg(&self) -> bool {
        matches!(self, Self::Arg)
    }

    #[inline]
    fn expects_paren(&self) -> bool {
        matches!(self, Self::Arg | Self::FnParen)
    }

    /// Show what to expect in each mode.
    fn expected(&self) -> &'static str {
        match self {
            Self::Arg => "argument, unary op or '('",
            Self::Op => "operator",
            Self::FnParen => "'('",
        }
    }
}

/// Move elements from op stack to output until left parenthesis is found.
/// Return true if there's some leftover.
fn tilt(stack: &mut Vec<Token>, output: &mut Vec<Token>) -> bool {
    while let Some(top) = stack.pop() {
        if let Token::LParen = top {
            return true;
        }
        output.push(top);
    }
    false
}

/// Iterate over stack and calculate the result.
pub fn eval(
    queue: &[Token],
    ts: OffsetDateTime,
    stack: &mut Vec<Token>,
    issue: &Entry,
) -> Result<Token> {
    use Token::*;

    for tok in queue {
        match tok {
            Duration(_) | Date(_) | Bool(_) | Regex(_) | String(_) => stack.push(tok.clone()),
            Reference(field) => stack.push(field.as_token(issue)),
            Add(false) => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.sum(rhs)?),
                _ => bail!("'+' operator haven't got enough arguments"),
            },
            Add(true) => match stack.pop() {
                Some(rhs) => stack.push(rhs),
                _ => bail!("Unary '+' operator haven't got enough arguments"),
            },
            Sub(false) => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.sub(rhs)?),
                _ => bail!("'-' operator haven't got enough arguments"),
            },
            Sub(true) => match stack.pop() {
                Some(rhs) => stack.push(rhs.neg()?),
                _ => bail!("Unary '-' operator haven't got enough arguments"),
            },
            Mul => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.mul(rhs)?),
                _ => bail!("'*' operator haven't got enough arguments"),
            },
            Div => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.div(rhs)?),
                _ => bail!("'/' operator haven't got enough arguments"),
            },
            Mod => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.modulo(rhs)?),
                _ => bail!("'%' operator haven't got enough arguments"),
            },
            At => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.at(rhs, ts)?),
                _ => bail!("'@' operator haven't got enough arguments"),
            },
            And => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.and(rhs)?),
                _ => bail!("'and' ('&&') operator haven't got enough arguments"),
            },
            Or => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.or(rhs)?),
                _ => bail!("'or' ('||') operator haven't got enough arguments"),
            },
            Eq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.eq(rhs, issue)?),
                _ => bail!("'eq' ('==') operator haven't got enough arguments"),
            },
            NotEq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.not_eq(rhs, issue)?),
                _ => bail!("'eq' ('!=') operator haven't got enough arguments"),
            },
            Not => match stack.pop() {
                Some(val) => stack.push(val.not()?),
                _ => bail!("'not' ('!') operator haven't got the argument"),
            },
            Func(funcref) => {
                let out = funcref.exec(stack, issue)?;
                stack.push(out);
            }
            Greater => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.greater(rhs, ts)?),
                _ => bail!("'>' operator haven't got enough arguments"),
            },
            GreaterEq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.greater_eq(rhs, ts)?),
                _ => bail!("'>=' operator haven't got enough arguments"),
            },
            Less => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.less(rhs, ts)?),
                _ => bail!("'<' operator haven't got enough arguments"),
            },
            LessEq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.less_eq(rhs, ts)?),
                _ => bail!("'<' operator haven't got enough arguments"),
            },
            FuzzyEq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.fuzzy_eq(&rhs, issue, ts)?),
                _ => bail!("':' operator haven't got enough arguments"),
            },
            LParen | RParen => {
                panic!()
            }
        }
    }

    let last = stack.last();
    last.context("Expression didn't produced any result")
        .cloned()
}

/// Find the the boundaries of regex.
fn parse_regex(input: &str) -> Result<(Rc<regex::Regex>, &str)> {
    // TODO: P2: support escaped backslashes
    let end = unwrap_some_or!(input.find("/"), {
        bail!("Non-terminated regex: {}", input);
    });

    let regex = Rc::new(regex::Regex::new(&input[..end])?);
    Ok((regex, &input[end + 1..]))
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

use logos::Logos as _;
use time::OffsetDateTime;

use crate::issue::Issue;
use crate::{app::App, prelude::*, token::Token};

/// Parse date expression and produce the timestamp.
/// Convert the incoming token stream using shunting yard algorithm into RPN and eval it.
pub fn parse_date(input: &str, app: &App, issue: &Issue) -> Result<i64> {
    let local = app.local_time()?;

    let mut exp = Vec::<Token>::new();
    parse_exp(input, local, &mut exp)?;

    let mut arg_stack = Vec::<Token>::new();
    let res = eval(&exp, local, &mut arg_stack, issue)?;

    match res {
        Token::Date(date) => Ok(date),
        Token::Duration(rel) => Ok(app.ts + rel as i64),
        Token::Bool(val) => bail!("Date expression returned boolean ({val})"),
        Token::Regex(_) => bail!("Date expression returned regular expression"),
        _ => panic!(),
    }
}

/// Parse and append to filter expression, return number of token added.
pub fn parse_local_exp(input: &str, app: &App, output: &mut Vec<Token>) -> Result<()> {
    let local = app.local_time()?;
    parse_exp(input, local, output)
}

/// Produce parsed ASP tree ready for evaluation from the input.
pub fn parse_exp(input: &str, ts: OffsetDateTime, output: &mut Vec<Token>) -> Result<()> {
    use Token::*;

    let mut op_stack = Vec::<Token>::new();
    let lexer = Token::lexer_with_extras(input, ts);

    let mut mode = Mode::Arg;

    for (tok, span) in lexer.spanned() {
        let tok =
            tok.with_context(|| format!("Unable to process token at position {}", span.start))?;

        match tok {
            Duration(_) | Date(_) | Bool(_) | Regex(_) | String(_) | Reference(_) => {
                if !mode.expects_arg() {
                    bail!("Unexpected argument");
                }
                output.push(tok);
                mode = Mode::Op;
            }
            Add(_) | Sub(_) | Mul | Div | Mod | At | Eq | FuzzyEq | Less | LessEq | Greater
            | GreaterEq | NotEq | And | Or | Not | Sqrt | Ln | Abs | Sig => {
                let (prec, left_assoc) = tok.prec_and_assoc();
                let (tok, left_assoc) = if !mode.expects_op() {
                    let (tok, left_assoc) = tok.to_unary();
                    if left_assoc {
                        bail!("Unexpected operator at position {}", span.start);
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
                if !mode.expects_arg() {
                    bail!("Unexpected opening bracket at position {}", span.start);
                }
                op_stack.push(tok);
                mode = Mode::Arg;
            }
            RParen => {
                if !tilt(&mut op_stack, output) {
                    bail!("Mismatched closing bracket at position {}", span.end);
                }
                mode = Mode::Op;
            }
        }
    }
    if tilt(&mut op_stack, output) {
        bail!("Mismatched opening bracket");
    }

    if !output.is_empty() && !mode.expects_op() {
        bail!("Dangling operator at the end of expression");
    }

    Ok(())
}

/// Token parser expected token state.
enum Mode {
    Arg,
    Op,
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
    issue: &Issue,
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
                (Some(rhs), Some(lhs)) => stack.push(lhs.eq(rhs)?),
                _ => bail!("'eq' ('==') operator haven't got enough arguments"),
            },
            Not => match stack.pop() {
                Some(val) => stack.push(val.not()?),
                _ => bail!("'not' ('!') operator haven't got the argument"),
            },
            Sqrt => match stack.pop() {
                Some(val) => stack.push(
                    val.unary_op(|v| v.sqrt())
                        .context("'sqrt' can only be applied to numbers")?,
                ),
                _ => bail!("'sqrt' operator haven't got the argument"),
            },
            Ln => match stack.pop() {
                Some(val) => stack.push(
                    val.unary_op(|v| v.ln())
                        .context("'ln' can only be applied to numbers")?,
                ),
                _ => bail!("'log' operator haven't got the argument"),
            },
            Abs => match stack.pop() {
                Some(val) => stack.push(
                    val.unary_op(|v| v.abs())
                        .context("'abs' can only be applied to numbers")?,
                ),
                _ => bail!("'abs' operator haven't got the argument"),
            },
            Sig => match stack.pop() {
                Some(val) => stack.push(
                    val.unary_op(|v| sigmoid(v))
                        .context("'sig' can only be applied to numbers")?,
                ),
                _ => bail!("'sig' operator haven't got the argument"),
            },
            Greater => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.greater(rhs)?),
                _ => bail!("'>' operator haven't got enough arguments"),
            },
            GreaterEq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.greater_eq(rhs)?),
                _ => bail!("'>=' operator haven't got enough arguments"),
            },
            Less => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.less(rhs)?),
                _ => bail!("'<' operator haven't got enough arguments"),
            },
            LessEq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.less_eq(rhs)?),
                _ => bail!("'<' operator haven't got enough arguments"),
            },
            FuzzyEq => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.fuzzy_eq(rhs, issue)?),
                _ => bail!("':' operator haven't got enough arguments"),
            },
            NotEq => {
                // TODO: P3: implement not equal
                todo!()
            }
            LParen | RParen => {
                panic!()
            }
        }
    }

    let last = stack.last();
    last.context("Expression didn't produced any result")
        .cloned()
}

/// Sigmoid function for urgency values normalization.
/// TODO: P1: move to utils mod.
#[inline]
fn sigmoid(input: f64) -> f64 {
    use std::f64::consts::E;
    1_f64 / (1_f64 + E.powf(-input))
}

#[test]
fn full_exp_parsing() {
    let (app, issue) = (&App::default(), &Issue::default());
    assert_eq!(parse_date("1.5h+2h", app, issue).unwrap(), 12600);
    assert_eq!(
        parse_date("1s+2s*3", app, issue).unwrap(),
        7,
        "op precedence"
    );
}

#[test]
fn relative_dates() {
    let (app, issue) = (&App::default(), &Issue::default());
    let monday = parse_date("monday", app, issue).unwrap();
    let tuesday = parse_date("tuesday", app, issue).unwrap();
    assert_eq!(tuesday - monday, 86400);
}

#[test]
fn unexpected_tokens() {
    let (app, issue) = (&App::default(), &Issue::default());
    assert_eq!(parse_date("1d+", app, issue).is_err(), true);
    assert_eq!(parse_date("1d2d", app, issue).is_err(), true);
    assert_eq!(parse_date("1d(2d)", app, issue).is_err(), true);
    assert_eq!(parse_date("(", app, issue).is_err(), true);
    assert_eq!(parse_date(")", app, issue).is_err(), true);
}

use logos::Logos as _;
use time::OffsetDateTime;

use crate::{App, prelude::*, token::Token};

/// Parse date expression and produce the timestamp.
/// Convert the incoming token stream using shunting yard algorithm into RPN and eval it.
pub fn parse_date(input: &str, app: &App) -> Result<i64> {
    let local = app.local_time()?;
    let exp = parse_exp(input, local)?;

    let mut arg_stack = Vec::<Token>::new();
    let res = eval(&exp, local, &mut arg_stack)?;

    match res {
        Token::Date(date) => Ok(date),
        Token::Duration(rel) => Ok(app.ts + rel as i64),
        Token::Bool(val) => bail!("Date expression returned boolean ({val})"),
        Token::Regex => bail!("Date expression returned regular expression"),
        _ => panic!(),
    }
}

/// Produce parsed ASP tree ready for evaluation from the input.
fn parse_exp(input: &str, ts: OffsetDateTime) -> Result<Vec<Token>> {
    use Token::*;

    let mut output = Vec::<Token>::new();
    let mut op_stack = Vec::<Token>::new();

    let lexer = Token::lexer_with_extras(input, ts);

    let mut mode = Mode::Any;

    for tok in lexer {
        let tok = tok?;

        match tok {
            Duration(_) | Date(_) | Bool(_) | Regex => {
                if !mode.expects_arg() {
                    bail!("Unexpected date argument");
                }
                output.push(tok);
                mode = Mode::Op;
            }
            Add | Sub | Mul | Div | At | Eq | PartialEq | Less | LessEq | Greater | GreaterEq
            | NotEq | And | Or | Not => {
                let (prec, left_assoc) = tok.prec_and_assoc();
                if !mode.expects_op() && left_assoc {
                    bail!("Unexpected operator");
                }
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
                    bail!("Unexpected opening bracket");
                }
                op_stack.push(tok);
                mode = Mode::Any;
            }
            RParen => {
                if !tilt(&mut op_stack, &mut output) {
                    bail!("Mismatched closing bracket");
                }
                mode = Mode::Op;
            }
            Symbol => panic!(),
        }
    }
    if tilt(&mut op_stack, &mut output) {
        bail!("Mismatched opening bracket");
    }

    if !mode.expects_op() {
        bail!("Dangling operator at the end of expression");
    }

    Ok(output)
}

/// Token parser expected token state.
enum Mode {
    Any,
    Arg,
    Op,
}

impl Mode {
    #[inline]
    fn expects_op(&self) -> bool {
        matches!(self, Self::Any | Self::Op)
    }

    #[inline]
    fn expects_arg(&self) -> bool {
        matches!(self, Self::Any | Self::Arg)
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
fn eval(queue: &Vec<Token>, ts: OffsetDateTime, stack: &mut Vec<Token>) -> Result<Token> {
    use Token::*;

    for tok in queue {
        match tok {
            Duration(_) | Date(_) | Bool(_) | Regex => stack.push(*tok),
            Add => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.sum(rhs)?),
                (Some(rhs), None) => stack.push(rhs),
                _ => bail!("'+' operator haven't got enough arguments"),
            },
            Sub => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.sub(rhs)?),
                (Some(rhs), None) => stack.push(rhs.neg()?),
                _ => bail!("'-' operator haven't got enough arguments"),
            },
            Mul => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.mul(rhs)?),
                _ => bail!("'*' operator haven't got enough arguments"),
            },
            Div => match (stack.pop(), stack.pop()) {
                (Some(rhs), Some(lhs)) => stack.push(lhs.div(rhs)?),
                _ => bail!("'/' operator haven't got enough arguments"),
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
            PartialEq | Less | LessEq | Greater | GreaterEq | NotEq => {
                todo!()
            }
            LParen | RParen | Symbol => {
                panic!()
            }
        }
    }

    let last = stack.last();
    last.context("Expression didn't produced any result")
        .copied()
}

#[test]
fn full_exp_parsing() {
    let app = App::default();
    assert_eq!(parse_date("1.5h+2h", &app).unwrap(), 12600);
    assert_eq!(parse_date("1s+2s*3", &app).unwrap(), 7, "op precedence");
}

#[test]
fn relative_dates() {
    let app = App::default();
    let monday = parse_date("monday", &app).unwrap();
    let tuesday = parse_date("tuesday", &app).unwrap();
    assert_eq!(tuesday - monday, 86400);
}

#[test]
fn unexpected_tokens() {
    let app = App::default();
    assert_eq!(parse_date("1d+", &app).is_err(), true);
    assert_eq!(parse_date("1d2d", &app).is_err(), true);
    assert_eq!(parse_date("1d(2d)", &app).is_err(), true);
    assert_eq!(parse_date("(", &app).is_err(), true);
    assert_eq!(parse_date(")", &app).is_err(), true);
}

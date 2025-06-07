use nom::branch::alt;
use nom::character::complete::{alpha0, digit1};
use nom::character::complete::{anychar, char};
use nom::combinator::iterator;
use nom::combinator::{map_res, opt, recognize};
use nom::{IResult, Parser};

use crate::prelude::*;

/// Parse date expression and produce the timestamp.
/// Convert the incoming token stream using shunting yard algorithm into RPN and eval it.
pub fn parse_date(input: &str) -> Result<i64> {
    use Token::*;

    let mut output = Vec::<Token>::new();
    let mut op_stack = Vec::<Token>::new();

    for tok in iterator(input, alt((parse_number, parse_op))) {
        match tok {
            Duration(_) | Date(_) => output.push(tok),
            Add | Sub | Mul | Div => {
                while let Some(top) = op_stack.pop_if(|top| {
                    if let LParen = top {
                        return false;
                    }
                    let (top_prec, _) = top.prec_and_assoc();
                    let (prec, left_assoc) = top.prec_and_assoc();

                    (top_prec > prec) || (top_prec == prec && left_assoc)
                }) {
                    output.push(top)
                }
                op_stack.push(tok);
            }
            LParen => op_stack.push(tok),
            RParen => {
                if !tilt(&mut op_stack, &mut output) {
                    bail!("Mismatched closing bracket");
                }
            }
        }
    }
    if tilt(&mut op_stack, &mut output) {
        bail!("Mismatched opening bracket");
    }

    eval(&output)
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

/// Convert float point string into token.
fn parse_number(input: &str) -> IResult<&str, Token> {
    map_res((recognize_float, alpha0), |(s, suffix): (&str, &str)| {
        match_suffix(s.parse::<f64>()?, suffix)
    })
    .parse(input)
}

/// Convert one of the supported date formats into token.
fn _parse_date(_input: &str) -> IResult<&str, Token> {
    todo!()
}

/// Check for one of possible operations.
fn parse_op(input: &str) -> IResult<&str, Token> {
    map_res(anychar, |c| match c {
        '+' => Ok(Token::Add),
        '-' => Ok(Token::Sub),
        '*' => Ok(Token::Mul),
        '/' => Ok(Token::Div),
        '(' => Ok(Token::LParen),
        ')' => Ok(Token::RParen),
        _ => bail!("Unknown character: {}", c),
    })
    .parse(input)
}

/// Recognize float point number pattern.
fn recognize_float(input: &str) -> IResult<&str, &str> {
    recognize((digit1, opt((char('.'), digit1)))).parse(input)
}

/// Convert number suffix to the seconds.
fn match_suffix(literal: f64, suffix: &str) -> Result<Token> {
    use Token::*;

    match suffix {
        "" | "s" => Ok(Duration((literal * 1.) as i64)),
        "m" => Ok(Duration((literal * 60.) as i64)),
        "h" => Ok(Duration((literal * 3600.) as i64)),
        "d" | "D" => Ok(Duration((literal * 86400.) as i64)),
        "w" | "W" => Ok(Duration((literal * 604800.) as i64)),
        "M" => Ok(Duration((literal * 2592000.) as i64)),
        "y" | "Y" => Ok(Duration((literal * 946080000.) as i64)),
        "st" | "nd" | "rd" | "th" => Ok(Date(0)),
        _ => bail!("Unknown number suffix: {}", suffix),
    }
}

/// Parsed token types.
#[derive(Clone, Copy, Debug)]
enum Token {
    #[allow(unused)]
    Duration(i64),
    #[allow(unused)]
    Date(i64),

    Add,
    Sub,
    Mul,
    Div,

    LParen,
    RParen,
}

impl Token {
    /// Check token precedence and if it's left associative.
    fn prec_and_assoc(&self) -> (u8, bool) {
        match self {
            Token::Add => (1, true),
            Token::Sub => (1, true),
            Token::Mul => (2, true),
            Token::Div => (2, true),
            _ => panic!("Token {:?} is not operator", self),
        }
    }
}

/// Iterate over stack and calculate the result.
fn eval(queue: &Vec<Token>) -> Result<i64> {
    use Token::*;

    let mut arg_stack = Vec::<Token>::new();

    for tok in queue {
        match tok {
            Duration(_) | Date(_) => arg_stack.push(*tok),
            _ => {}
        }
    }

    Ok(0)
}

use std::num::ParseFloatError;

use nom::character::complete::char;
use nom::character::complete::{alpha0, digit1};
use nom::combinator::iterator;
use nom::combinator::{map_res, opt, recognize};
use nom::{IResult, Parser};

use crate::prelude::*;

/// Parse date expression and produce the timestamp.
pub fn parse_date(input: &str) -> Result<i64> {
    let mut output = Vec::<Token>::new();
    let _op_stack = Vec::<Token>::new();

    for tok in iterator(input, parse_number) {
        match tok {
            Token::Duration(_) | Token::_Date(_) => output.push(tok),
            _ => {}
        }
    }

    let out = output.iter().cloned().reduce(|v, s| {
        if let Token::Duration(v) = v {
            if let Token::Duration(s) = s {
                return Token::Duration(v + s);
            }
        }
        Token::Duration(0.)
    });

    if let Token::Duration(out) = out.unwrap_or(Token::Duration(0.)) {
        return Ok(out as i64);
    }
    bail!("Not a number");
}

/// Convert float point string into f64.
fn parse_number(input: &str) -> IResult<&str, Token> {
    map_res((recognize_float, alpha0), |(s, suffix): (&str, &str)| {
        Ok::<_, ParseFloatError>(match_suffix(s.parse::<f64>()?, suffix))
    })
    .parse(input)
}

/// Recognize float point number pattern.
fn recognize_float(input: &str) -> IResult<&str, &str> {
    recognize((digit1, opt((char('.'), digit1)))).parse(input)
}

/// Convert number suffix to the seconds.
fn match_suffix(literal: f64, suffix: &str) -> Token {
    match suffix {
        "" => Token::Duration(literal * 1.),
        "s" => Token::Duration(literal * 1.),
        "m" => Token::Duration(literal * 60.),
        "h" => Token::Duration(literal * 3600.),
        "d" | "D" => Token::Duration(literal * 86400.),
        "w" | "W" => Token::Duration(literal * 604800.),
        "M" => Token::Duration(literal * 2592000.),
        "y" | "Y" => Token::Duration(literal * 946080000.),
        _ => panic!("Unknown number suffix: {}", suffix),
    }
}

/// Parsed token types.
#[derive(Clone, Copy)]
enum Token {
    Duration(f64),
    _Date(i64),
    _Add,
    _Sub,
    _Mul,
    _Div,
}

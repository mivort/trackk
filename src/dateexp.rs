use nom::branch::alt;
use nom::character::complete::{alpha0, digit1};
use nom::character::complete::{anychar, char};
use nom::combinator::iterator;
use nom::combinator::{map_res, opt, recognize};
use nom::{IResult, Parser};

use logos::{Lexer, Logos};

use crate::{App, prelude::*};

/// Parse date expression and produce the timestamp.
/// Convert the incoming token stream using shunting yard algorithm into RPN and eval it.
pub fn parse_date(input: &str, app: &App) -> Result<i64> {
    use Token::*;

    let mut output = Vec::<Token>::new();
    let mut op_stack = Vec::<Token>::new();

    let lexer = Token::lexer_with_extras(input, app.ts);
    for tok in lexer {
        let tok = unwrap_ok_or!(tok, _, { bail!("Unknown token") });

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

    if false {
        for tok in iterator(input, alt((parse_rfc3339, parse_number, parse_op))) {
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

/// Convert RFC3339 date into token.
fn parse_rfc3339(input: &str) -> IResult<&str, Token> {
    map_res(recognize_rfc3339, |_date| -> Result<Token> {
        Ok(Token::Date(0))
    })
    .parse(input)
}

/// Check for one of possible operations.
fn parse_op(input: &str) -> IResult<&str, Token> {
    use Token::*;

    map_res(anychar, |c| match c {
        '+' => Ok(Add),
        '-' => Ok(Sub),
        '*' => Ok(Mul),
        '/' => Ok(Div),
        '(' => Ok(LParen),
        ')' => Ok(RParen),
        _ => bail!("Unknown character: {}", c),
    })
    .parse(input)
}

/// Recognize float point number pattern.
fn recognize_float(input: &str) -> IResult<&str, &str> {
    recognize((digit1, opt((char('.'), digit1)))).parse(input)
}

/// Recognize RFC3339.
fn recognize_rfc3339(input: &str) -> IResult<&str, &str> {
    let with_year = (digit1, char('-'), digit1, char('-'), digit1);
    let no_year = (digit1, char('-'), digit1);
    alt((recognize(with_year), recognize(no_year))).parse(input)
}

/// Convert number suffix to the seconds.
fn match_suffix(literal: f64, suffix: &str) -> Result<Token> {
    use Token::*;

    match suffix {
        "" | "s" => Ok(Duration(literal * 1.)),
        "m" => Ok(Duration(literal * 60.)),
        "h" => Ok(Duration(literal * 3600.)),
        "d" | "D" => Ok(Duration(literal * 86400.)),
        "w" | "W" => Ok(Duration(literal * 604800.)),
        "M" => Ok(Duration(literal * 2592000.)),
        "y" | "Y" => Ok(Duration(literal * 946080000.)),
        "st" | "nd" | "rd" | "th" => Ok(Date(0)),
        _ => bail!("Unknown number suffix: {}", suffix),
    }
}

/// Parse no-suffix duration.
#[inline]
fn parse_no_suffix_span(lex: &Lexer<Token>) -> Option<f64> {
    lex.slice().parse().ok()
}

/// Exclude suffix and parse with multiplier.
#[inline]
fn parse_suffix_span(lex: &Lexer<Token>, width: usize, mlt: f64) -> Option<f64> {
    let slice = lex.slice();
    slice[..slice.len() - width]
        .parse()
        .ok()
        .map(|v: f64| v * mlt)
}

/// Parse the closest month day.
fn parse_st_nd_rd_th(lex: &Lexer<Token>) -> Option<i64> {
    let slice = lex.slice();
    slice[..slice.len() - 2].parse::<i64>().ok().map(|v| v)
}

/// Parse date in `[month]-[day]` format.
fn parse_short_date(_: &Lexer<Token>) -> Option<i64> {
    None
}

/// Parse date in `[year]-[month]-[day]` format.
fn parse_full_date(_: &Lexer<Token>) -> Option<i64> {
    None
}

/// Parse date and time in `[year]-[month]-[day]T[hour]:[minute]:[second] format.
fn parse_date_time(_: &Lexer<Token>) -> Option<i64> {
    None
}

/// Parsed token types.
#[derive(Clone, Copy, Debug, Logos)]
#[logos(skip r"[ \t\n\f]+", extras = i64)]
enum Token {
    #[regex(r"\d+", parse_no_suffix_span)]
    #[regex(r"\d+s", |l| parse_suffix_span(l, 1, 1.))]
    #[regex(r"\d+m", |l| parse_suffix_span(l, 1, 60.))]
    #[regex(r"\d+h", |l| parse_suffix_span(l, 1, 3600.))]
    #[regex(r"\d+[Dd]", |l| parse_suffix_span(l, 1, 86400.))]
    #[regex(r"\d+[Ww]", |l| parse_suffix_span(l, 1, 604800.))]
    #[regex(r"\d+M", |l| parse_suffix_span(l, 1, 2592000.))]
    #[regex(r"\d+[Yy]", |l| parse_suffix_span(l, 1, 946080000.))]
    Duration(f64),

    #[regex(r"\d+(st|nd|rd|th)", parse_st_nd_rd_th)]
    #[regex(r"\d{2}-\d{2}", parse_short_date)]
    #[regex(r"\d{4,}-\d{2}-\d{2}", parse_full_date)]
    #[regex(r"\d{4,}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}", parse_date_time)]
    Date(i64),

    #[token("+")]
    Add,

    #[token("-")]
    Sub,

    #[token("*")]
    Mul,

    #[token("/")]
    Div,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,
}

impl Token {
    /// Check token precedence and if it's left associative.
    fn prec_and_assoc(&self) -> (u8, bool) {
        use Token::*;

        match self {
            Add => (1, true),
            Sub => (1, true),
            Mul => (2, true),
            Div => (2, true),
            _ => panic!("Token {:?} is not operator", self),
        }
    }

    /// Produce a sum of durations or durations and dates, error for
    /// incompatible arguments.
    fn sum(self, rhs: Self) -> Result<Self> {
        match self {
            Self::Duration(lhs) => match rhs {
                Self::Date(rhs) => Ok(Self::Date(lhs as i64 + rhs)),
                Self::Duration(rhs) => Ok(Self::Duration(lhs + rhs)),
                _ => bail!("Unsupported arguments"),
            },
            Self::Date(lhs) => match rhs {
                Self::Duration(rhs) => Ok(Self::Date(lhs + rhs as i64)),
                _ => bail!("Unsupported arguments"),
            },
            _ => panic!("Non-literal arguments"),
        }
    }

    /// Produce a substraction of durations and dates.
    fn sub(self, rhs: Self) -> Result<Self> {
        match self {
            Self::Duration(lhs) => match rhs {
                Self::Duration(rhs) => Ok(Self::Duration(lhs - rhs)),
                Self::Date(_) => bail!("Date can't be negative"),
                _ => bail!("Unsupported arguments"),
            },
            Self::Date(lhs) => match rhs {
                Self::Duration(rhs) => Ok(Self::Date(lhs - rhs as i64)),
                _ => bail!("Unsupported arguments"),
            },
            _ => panic!("Non-literal arguments"),
        }
    }

    /// Produce a multiplication of two duration values.
    fn mul(self, rhs: Self) -> Result<Self> {
        match self {
            Self::Duration(lhs) => match rhs {
                Self::Duration(rhs) => Ok(Self::Duration(lhs * rhs)),
                _ => bail!("Unsupported arguments"),
            },
            Self::Date(_) => bail!("Unsupported arguments"),
            _ => panic!("Non-literal arguments"),
        }
    }

    /// Produce a multiplication of two duration values.
    fn div(self, rhs: Self) -> Result<Self> {
        match self {
            Self::Duration(lhs) => match rhs {
                Self::Duration(rhs) => Ok(Self::Duration(lhs / rhs)),
                _ => bail!("Unsupported arguments"),
            },
            Self::Date(_) => bail!("Unsupported arguments"),
            _ => panic!("Non-literal arguments"),
        }
    }

    /// Produce a negative result of a value.
    fn neg(self) -> Result<Self> {
        match self {
            Self::Duration(duration) => Ok(Self::Duration(-duration)),
            Self::Date(_) => bail!("Date can't be negative"),
            _ => panic!("Operator applied to the wrong token"),
        }
    }

    /// Convert date or duration to i64 timestamp.
    fn as_i64(&self) -> i64 {
        match self {
            Self::Duration(offset) => *offset as i64,
            Self::Date(date) => *date,
            _ => panic!("Non-literal token"),
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
            Add => match (arg_stack.pop(), arg_stack.pop()) {
                (Some(rhs), Some(lhs)) => arg_stack.push(lhs.sum(rhs)?),
                (Some(rhs), None) => arg_stack.push(rhs),
                _ => bail!("'+' operator haven't got enough arguments"),
            },
            Sub => match (arg_stack.pop(), arg_stack.pop()) {
                (Some(rhs), Some(lhs)) => arg_stack.push(lhs.sub(rhs)?),
                (Some(rhs), None) => arg_stack.push(rhs.neg()?),
                _ => bail!("'-' operator haven't got enough arguments"),
            },
            Mul => match (arg_stack.pop(), arg_stack.pop()) {
                (Some(rhs), Some(lhs)) => arg_stack.push(lhs.mul(rhs)?),
                _ => bail!("'*' operator haven't got enough arguments"),
            },
            Div => match (arg_stack.pop(), arg_stack.pop()) {
                (Some(rhs), Some(lhs)) => arg_stack.push(lhs.div(rhs)?),
                _ => bail!("'/' operator haven't got enough arguments"),
            },
            _ => {}
        }
    }

    let last = arg_stack.last();
    last.context("Expression didn't produced any result")
        .map(|t| t.as_i64())
}

#[test]
fn full_exp_parsing() {
    let app = App::default();
    assert_eq!(matches!(parse_date("1h+2h", &app), Ok(10800)), true);
}

use nom::character::complete::char;
use nom::character::complete::digit1;
use nom::combinator::{map_res, opt, recognize};
use nom::{IResult, Parser};

use crate::prelude::*;

/// Parse date expression and produce the timestamp.
pub fn parse_date(input: &str) -> Result<i64> {
    let (_, num) = unwrap_ok_or!(parse_number(input), e, {
        bail!("Date parsing error: {:?}", e)
    });
    Ok((num * 10.) as i64)
}

/// Convert float point string into f64.
fn parse_number(input: &str) -> IResult<&str, f64> {
    map_res(recognize_float, |s: &str| s.parse::<f64>()).parse(input)
}

/// Recognize float point number pattern.
fn recognize_float(input: &str) -> IResult<&str, &str> {
    recognize((digit1, opt((char('.'), digit1)))).parse(input)
}

use std::num::ParseFloatError;

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
    Ok(num as i64)
}

/// Convert float point string into f64.
fn parse_number(input: &str) -> IResult<&str, f64> {
    map_res(
        (recognize_float, nom::character::complete::alpha0),
        |(s, suffix): (&str, &str)| {
            Ok::<_, ParseFloatError>(s.parse::<f64>()? * match_suffix(suffix))
        },
    )
    .parse(input)
}

/// Recognize float point number pattern.
fn recognize_float(input: &str) -> IResult<&str, &str> {
    recognize((digit1, opt((char('.'), digit1)))).parse(input)
}

/// Convert number suffix to the seconds.
fn match_suffix(suffix: &str) -> f64 {
    match suffix {
        "s" => 1.,
        "m" => 60.,
        "h" => 3600.,
        "d" | "D" => 86400.,
        "w" | "W" => 604800.,
        "M" => 2592000.,
        "y" | "Y" => 946080000.,
        _ => 1.,
    }
}

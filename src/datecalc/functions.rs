use time::OffsetDateTime;

use super::token::Token;
use crate::entry::{Entry, FieldRef};
use crate::prelude::*;

/// Supported built-in functions.
#[derive(Debug, Clone)]
#[allow(unused)]
pub enum FuncRef {
    Abs,
    Empty,
    Len,
    Lines,
    Ln,
    Sig,
    Sqrt,
    Weekday,
}

impl FuncRef {
    /// Take arguments from the stack and produce the result.
    pub fn exec(&self, stack: &mut Vec<Token>, entry: &Entry, ts: OffsetDateTime) -> Result<Token> {
        use FuncRef::*;

        match self {
            Abs => unary_func(stack.pop(), f64::abs),
            Ln => unary_func(stack.pop(), f64::ln),
            Sig => unary_func(stack.pop(), sigmoid),
            Sqrt => unary_func(stack.pop(), f64::sqrt),

            Empty => empty(stack.pop(), entry),
            Len => length(stack.pop(), entry),
            Lines => lines(stack.pop(), entry),

            Weekday => weekday(stack.pop(), ts),
        }
    }
}

/// Apply single-argument math function.
#[inline]
fn unary_func(tok: Option<Token>, f: impl Fn(f64) -> f64) -> Result<Token> {
    match tok {
        Some(Token::Duration(val)) => Ok(Token::Duration(f(val))),
        Some(tok) => bail!("Unary function got incompatible argument ({})", tok.ttype()),
        None => bail!(""),
    }
}

/// Sigmoid function for urgency values normalization.
#[inline]
fn sigmoid(input: f64) -> f64 {
    use std::f64::consts::E;
    1_f64 / (1_f64 + E.powf(-input))
}

/// Convert various values to boolean. Strings become 'false' if empty,
/// dates - if equal to 'now'.
fn empty(tok: Option<Token>, entry: &Entry) -> Result<Token> {
    let tok = unwrap_some_or!(tok, { bail!("'empty' requires argument") });

    Ok(Token::Bool(token_length(tok, entry)? == 0))
}

/// Produce length of the token value.
fn length(tok: Option<Token>, entry: &Entry) -> Result<Token> {
    let tok = unwrap_some_or!(tok, { bail!("'len' requires argument") });

    Ok(Token::Duration(token_length(tok, entry)? as f64))
}

/// Produce length value for supported token types.
fn token_length(tok: Token, entry: &Entry) -> Result<usize> {
    use Token::*;

    match tok {
        String(val) => Ok(val.len()),
        Reference(field) => Ok(field.length(entry)),
        Bool(false) => Ok(0),
        _ => bail!(
            "Value type ({}) doesn't have length to check with 'len' or 'empty'",
            tok.ttype()
        ),
    }
}

/// Calculate number of lines in string values - useful to filter entries with notes.
fn lines(tok: Option<Token>, entry: &Entry) -> Result<Token> {
    use Token::*;

    let tok = unwrap_some_or!(tok, { bail!("'lines' requires argument") });

    match tok {
        String(val) => Ok(Duration(val.lines().count() as f64)),
        Reference(FieldRef::Desc) => Ok(Duration(entry.desc.lines().count() as f64)),
        Reference(FieldRef::Tag) => Ok(Duration(entry.tags.len() as f64)),
        Reference(FieldRef::Status) => Ok(Duration(1.0)),
        _ => bail!(
            "'lines' function got incompatible argument ({})",
            tok.ttype()
        ),
    }
}

/// Find day of week for provided date or duration (0 is Monday, 6 is Sunday).
fn weekday(tok: Option<Token>, ts: OffsetDateTime) -> Result<Token> {
    let tok = unwrap_some_or!(tok, { bail!("'weekday' requires argument") });
    let date = match tok {
        Token::Date(d) => d,
        Token::Duration(d) => super::duration_to_date(d, ts),
        _ => bail!(
            "'weekday' takes number or date as argument (got {})",
            tok.ttype()
        ),
    };

    const NEG: i64 = 86400 * 365 * 1000;
    let utc = date + ts.offset().whole_seconds() as i64 + NEG;
    let weekday = (utc % (86400 * 7) / 86400 + 4) % 7;

    Ok(Token::Duration(weekday as f64))
}

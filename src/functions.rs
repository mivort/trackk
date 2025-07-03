use crate::datecalc::token::{SOMEDAY, Token};
use crate::entry::{Entry, FieldRef};
use crate::prelude::*;

/// Supported built-in functions.
#[derive(Debug, Clone)]
#[allow(unused)]
pub enum FuncRef {
    Abs,
    Has,
    Len,
    Lines,
    Ln,
    Sig,
    Sqrt,
}

impl FuncRef {
    /// Take arguments from the stack and produce the result.
    pub fn exec(&self, stack: &mut Vec<Token>, entry: &Entry) -> Result<Token> {
        use FuncRef::*;

        match self {
            Abs => unary_func(stack.pop(), f64::abs),
            Ln => unary_func(stack.pop(), f64::ln),
            Sig => unary_func(stack.pop(), sigmoid),
            Sqrt => unary_func(stack.pop(), f64::sqrt),

            Has => has(stack.pop(), entry),
            Len => length(stack.pop(), entry),
            Lines => lines(stack.pop(), entry),
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

/// Convert various values to boolean. Strings and arrays are 'true' if not
/// empty, dates if less than 'someday'.
fn has(tok: Option<Token>, entry: &Entry) -> Result<Token> {
    use Token::*;

    let tok = unwrap_some_or!(tok, { bail!("'has' requires argument") });

    match tok {
        String(val) => Ok(Bool(!val.is_empty())),
        Bool(val) => Ok(Bool(val)),
        Date(val) => Ok(Bool(val <= SOMEDAY)),
        Reference(field) => Ok(Bool(field.has(entry))),
        _ => bail!("'has' function got incompatible argument ({})", tok.ttype()),
    }
}

/// Produce length of the token value.
fn length(tok: Option<Token>, entry: &Entry) -> Result<Token> {
    use Token::*;

    let tok = unwrap_some_or!(tok, { bail!("'has' requires argument") });

    match tok {
        String(val) => Ok(Duration(val.len() as f64)),
        Reference(field) => Ok(Duration(field.length(entry))),
        _ => bail!("'len' function got incompatible argument ({})", tok.ttype()),
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

use crate::issue::Issue;
use crate::prelude::*;
use crate::token::{SOMEDAY, Token};

/// Supported built-in functions.
#[derive(Debug, Clone)]
#[allow(unused)]
pub enum FuncRef {
    Abs,
    Has,
    Len,
    Ln,
    Sig,
    Sqrt,
}

impl FuncRef {
    /// Take arguments from the stack and produce the result.
    pub fn exec(&self, stack: &mut Vec<Token>, entry: &Issue) -> Result<Token> {
        use FuncRef::*;

        match self {
            Abs => unary_func(stack.pop(), f64::abs),
            Ln => unary_func(stack.pop(), f64::ln),
            Sig => unary_func(stack.pop(), sigmoid),
            Sqrt => unary_func(stack.pop(), f64::sqrt),

            Has => has(stack.pop(), entry),
            Len => length(stack.pop(), entry),
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
fn has(tok: Option<Token>, entry: &Issue) -> Result<Token> {
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
fn length(tok: Option<Token>, entry: &Issue) -> Result<Token> {
    use Token::*;

    let tok = unwrap_some_or!(tok, { bail!("'has' requires argument") });

    match tok {
        String(val) => Ok(Duration(val.len() as f64)),
        Reference(field) => Ok(Duration(field.length(entry))),
        _ => bail!("'len' function got incompatible argument ({})", tok.ttype()),
    }
}

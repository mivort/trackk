use std::ops::Range;
use std::rc::Rc;

use logos::Logos;
use time::OffsetDateTime;

use super::eval::eval;
use super::token::Token;
use crate::entry::{Entry, FieldRef};
use crate::{app::App, prelude::*};

/// Parse date expression and produce the timestamp.
/// Convert the incoming token stream using shunting yard algorithm into RPN and eval it.
pub fn parse_date(input: &str, app: &App, issue: &Entry) -> Result<Option<i64>> {
    let local = app.local_time()?;

    let mut exp = Vec::<Token>::new();
    parse_exp(input, local, &mut exp)?;

    let mut arg_stack = Vec::<Token>::new();
    let res = eval(&exp, local, &mut arg_stack, issue)?;

    match res {
        Token::Date(date) => Ok(Some(date)),
        Token::Duration(rel) => Ok(Some(app.ts + rel as i64)),
        Token::Bool(false) | Token::Else => Ok(None),
        Token::Bool(true) => bail!("Date expression returned 'true'"),
        _ => bail!(
            "Date expression should return date, duration or 'false' (got {})",
            res.ttype()
        ),
    }
}

/// Parse and append to filter expression. If experssion wasn't empty,
/// call the merger method to add glue operations.
///
/// If only one token was found in expression, check if it's string or regex,
/// and add comparison to the title.
pub fn parse_filter(
    input: &str,
    app: &App,
    output: &mut Vec<Token>,
    merger: impl Fn(&mut Vec<Token>, bool),
) -> Result<()> {
    let before = output.len();
    let res = parse_local_exp(input, app, output);
    let delta = output.len() - before;

    if delta == 0 {
        return res;
    } else if delta == 1 {
        match output.last() {
            Some(Token::String(_)) | Some(Token::Regex(_)) => {
                output.insert(output.len() - 1, Token::Reference(FieldRef::Title));
                output.push(Token::Contains);
            }
            _ => {}
        }
    }

    merger(output, before != 0);

    res
}

/// Pass app local timestamp and parse the expresion.
pub fn parse_local_exp(input: &str, app: &App, output: &mut Vec<Token>) -> Result<()> {
    let local = app.local_time()?;
    parse_exp(input, local, output)
}

/// Produce parsed ASP tree ready for evaluation from the input.
pub fn parse_exp(mut input: &str, ts: OffsetDateTime, output: &mut Vec<Token>) -> Result<()> {
    use Token::*;

    let initial = output.len();
    let mut op_stack = Vec::<Token>::new();
    let mut mode = Mode::Arg;

    'outer: loop {
        let lexer = Token::lexer_with_extras(input, ts);
        for (tok, Range { start, end }) in lexer.spanned() {
            let tok =
                tok.with_context(|| format!("Unable to process token at position {}", start))?;

            match tok {
                Duration(_) | Date(_) | Bool(_) | Regex(_) | String(_) | Reference(_) => {
                    if !mode.expects_arg() {
                        bail!(
                            "Expected {}, got argument '{}' at position {}",
                            mode.expected(),
                            &input[start..end],
                            start
                        );
                    }
                    output.push(tok);
                    mode = Mode::Op;
                }
                Comma => {
                    if !mode.expects_comma() {
                        bail!(
                            "Expected {}, got '{}' at position {}",
                            mode.expected(),
                            &input[start..end],
                            start
                        );
                    }
                    #[allow(unused_assignments)]
                    {
                        mode = Mode::Arg;
                    }
                    todo!()
                }
                Func(_) => {
                    op_stack.push(tok);
                    mode = Mode::FnParen;
                }
                Add(_) | Sub(_) | Mul | Div | Mod | At | Eq | Contains | In | Less(_)
                | LessEq(_) | Greater(_) | GreaterEq(_) | NotEq | And | Or | Not | If | Else => {
                    let (prec, left_assoc) = tok.prec_and_assoc();
                    let (prec, tok, left_assoc) = if mode.expects_arg() {
                        if let Div = tok {
                            let (regex, remainder) = parse_regex(&input[end..])?;
                            output.push(Regex(regex));
                            mode = Mode::Op;
                            input = remainder;
                            continue 'outer;
                        }

                        let (tok, left_assoc, prec) = tok.to_unary(prec);
                        if left_assoc {
                            bail!(
                                "Expected {}, got operator '{}' at position {}",
                                mode.expected(),
                                &input[start..end],
                                start
                            );
                        }
                        (prec, tok, left_assoc)
                    } else {
                        (prec, tok, left_assoc)
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
                    if mode.expects_paren() {
                        op_stack.push(tok);
                        mode = Mode::Arg;
                        continue;
                    }
                    bail!(
                        "Expected {}, got '(' at position {}",
                        mode.expected(),
                        start
                    );
                }
                RParen => {
                    if !tilt(&mut op_stack, output) {
                        bail!("Mismatched ')' at position {}", end);
                    }
                    op_stack.pop_if(|top| {
                        if matches!(top, Func(_)) {
                            output.push(top.clone());
                            return true;
                        }
                        false
                    });
                    mode = Mode::Op;
                }
            }
        }
        break;
    }
    if tilt(&mut op_stack, output) {
        bail!("Mismatched opening bracket");
    }

    if output.len() != initial && !mode.expects_op() {
        bail!("Dangling operator at the end of expression");
    }

    Ok(())
}

/// Token parser expected token state.
enum Mode {
    Arg,
    Op,
    FnParen,
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

    #[inline]
    fn expects_paren(&self) -> bool {
        matches!(self, Self::Arg | Self::FnParen)
    }

    #[inline]
    fn expects_comma(&self) -> bool {
        matches!(self, Self::Op)
    }

    /// Show what to expect in each mode.
    fn expected(&self) -> &'static str {
        match self {
            Self::Arg => "argument, unary op or '('",
            Self::Op => "operator",
            Self::FnParen => "'('",
        }
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

/// Find the the boundaries of regex.
fn parse_regex(input: &str) -> Result<(Rc<regex::Regex>, &str)> {
    // TODO: P2: support escaped backslashes
    let end = unwrap_some_or!(input.find("/"), {
        bail!("Non-terminated regex: {}", input);
    });

    let regex = Rc::new(regex::Regex::new(&input[..end])?);
    Ok((regex, &input[end + 1..]))
}

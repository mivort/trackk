use std::rc::Rc;

use crate::args::Args;
use crate::dateexp::{eval, parse_filter};
use crate::issue::{FieldRef, Issue};
use crate::token::Token;
use crate::{App, prelude::*};

/// List of filter rules.
#[derive(Default)]
pub struct Filter {
    /// List of IDs to include in the result.
    pub ids: Vec<String>,

    /// Match expression to eval on entries.
    expression: Vec<Token>,
}

impl Filter {
    /// Compare issue properties to the filter.
    pub fn match_issue(&self, issue: &Issue, app: &App, stack: &mut Vec<Token>) -> Result<bool> {
        if self.expression.is_empty() {
            return Ok(true);
        }

        let res = eval(&self.expression, app.local_time()?, stack, issue)?;
        match res {
            Token::Bool(res) => Ok(res),
            Token::Date(_) | Token::Duration(_) => Ok(true),
            _ => bail!("Filter expression produced non-boolean result"),
        }
    }
}

/// Parse filter expressions and combine these with 'and' operator.
pub fn parse_filter_args(args: &Args, app: &App) -> Result<Filter> {
    let mut filter = Filter::default();
    let expression = &mut filter.expression;

    for expr in &args.filter_args.filter {
        let append = !expression.is_empty();
        parse_filter(expr, app, expression)?;

        if append {
            expression.push(Token::And)
        }
    }

    for title in &args.filter_args.title {
        let append = !expression.is_empty();
        expression.push(Token::Reference(FieldRef::Title));
        expression.push(Token::String(Rc::from(title.as_str())));
        expression.push(Token::FuzzyEq);

        if append {
            expression.push(Token::And);
        }
    }

    // TODO: P3: add changes to filter from each argument type

    Ok(filter)
}

/// Store provided list of IDs as index.
#[derive(Default)]
pub struct IdFilter {
    pub index: Vec<String>,

    /// Flag if ID filter shouldn't match anything.
    pub empty_set: bool,
}

impl IdFilter {
    /// Convert list of IDs with shorthands into a set of fully resolved IDs.
    pub fn from_shorthands(mut ids: Vec<String>, app: &App) -> Result<Self> {
        if ids.is_empty() {
            return Ok(Self {
                index: Default::default(),
                empty_set: false,
            });
        }

        let index = app.index()?;

        ids.retain_mut(|id| {
            let shorthand = unwrap_ok_or!(id.parse::<usize>(), _e, {
                return true;
            });

            if shorthand > 999999 {
                return true;
            }

            let pointer = unwrap_some_or!(index.active().get(shorthand - 1), {
                return false;
            });
            let (_, resolved) = unwrap_some_or!(pointer.rsplit_once("/"), {
                return false;
            });

            *id = resolved.to_owned();
            true
        });

        let empty_set = ids.is_empty();

        Ok(Self {
            index: ids,
            empty_set,
        })
    }

    /// Check if ID filter matches the ID.
    pub fn matches(&self, value: &str) -> bool {
        self.index.is_empty() || self.index.iter().any(|id| value.starts_with(id))
    }
}

#[test]
fn match_issue() {
    use std::collections::HashSet;

    let app = Default::default();
    let mut tags = HashSet::<String>::default();
    tags.extend(["a", "b", "c"].map(Into::into).into_iter());

    let issue = Issue {
        tags,
        ..Default::default()
    };

    let match_filter = |input: &str| {
        let mut exp = Vec::new();
        parse_filter(input, &app, &mut exp).unwrap();

        Filter {
            expression: exp,
            ..Default::default()
        }
        .match_issue(&issue, &app, &mut Vec::new())
        .unwrap()
    };

    assert_eq!(match_filter("true"), true);
    assert_eq!(match_filter("false"), false);
}

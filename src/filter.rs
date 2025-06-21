use std::rc::Rc;

use crate::args::Args;
use crate::dateexp::{eval, parse_local_exp};
use crate::issue::{FieldRef, Issue};
use crate::token::Token;
use crate::{app::App, prelude::*};

/// Set of ID and query based filters.
pub struct Filter<'a> {
    /// Match specific IDs.
    pub ids: &'a IdFilter,

    /// Filter by query expression.
    pub query: &'a mut QueryFilter,
}

/// List of filter rules.
#[derive(Default)]
pub struct QueryFilter {
    /// Match expression to eval on entries.
    expression: Vec<Token>,
}

impl QueryFilter {
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

    /// Replace filter query re-using the vec.
    pub fn replace(&mut self, expr: &str, app: &App) -> Result<()> {
        self.expression.clear();
        parse_local_exp(expr, app, &mut self.expression)
    }

    /// Append '&&' condition on top of the query.
    #[inline]
    fn merge(&mut self, merger: impl Fn(&mut Vec<Token>)) {
        let was_empty = self.expression.is_empty();
        merger(&mut self.expression);

        if !was_empty {
            self.expression.push(Token::And);
        }
    }
}

/// Parse filter expressions and combine these with 'and' operator.
pub fn parse_filter_args(args: &Args, app: &App) -> Result<QueryFilter> {
    let mut filter = QueryFilter::default();
    let expression = &mut filter.expression;

    for expr in &args.filter_args.filter {
        let append = !expression.is_empty();
        parse_local_exp(expr, app, expression)?;

        if append {
            expression.push(Token::And)
        }
    }

    for title in &args.filter_args.title {
        filter.merge(|e| {
            e.push(Token::Reference(FieldRef::Title));
            e.push(Token::String(Rc::from(title.as_str())));
            e.push(Token::FuzzyEq);
        });
    }

    for desc in &args.filter_args.desc {
        filter.merge(|e| {
            e.push(Token::Reference(FieldRef::Desc));
            e.push(Token::String(Rc::from(desc.as_str())));
            e.push(Token::FuzzyEq);
        });
    }

    for tag in &args.filter_args.tag {
        filter.merge(|e| {
            e.push(Token::Reference(FieldRef::Tag));
            e.push(Token::String(Rc::from(tag.as_str())));
            e.push(Token::FuzzyEq);
        });
    }

    for notag in &args.filter_args.notag {
        filter.merge(|e| {
            e.push(Token::Reference(FieldRef::Tag));
            e.push(Token::String(Rc::from(notag.as_str())));
            e.push(Token::FuzzyEq);
            e.push(Token::Not);
        });
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

    /// Flag if there was any unresolved shorthands. If query contained
    /// only shorthands, it's safe to only check the shorthands index.
    pub unresolved: bool,
}

impl IdFilter {
    /// Convert list of IDs with shorthands into a set of fully resolved IDs.
    pub fn from_shorthands(mut ids: Vec<String>, app: &App) -> Result<Self> {
        if ids.is_empty() {
            return Ok(Self {
                index: Default::default(),
                unresolved: false,
                empty_set: false,
            });
        }

        let index = app.index()?;
        let mut unresolved = false;

        ids.retain_mut(|id| {
            let shorthand = unwrap_ok_or!(id.parse::<usize>(), _e, {
                unresolved = true;
                return true;
            });

            let pointer = unwrap_some_or!(index.active().get(shorthand - 1), {
                unresolved = true;
                return true;
            });
            let (_, resolved) = unwrap_some_or!(pointer.rsplit_once("/"), {
                warn!("Index entry with missing path: {pointer}");
                return false;
            });

            *id = resolved.to_owned();
            true
        });

        let empty_set = ids.is_empty();

        Ok(Self {
            index: ids,
            unresolved,
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
        parse_local_exp(input, &app, &mut exp).unwrap();

        QueryFilter {
            expression: exp,
            ..Default::default()
        }
        .match_issue(&issue, &app, &mut Vec::new())
        .unwrap()
    };

    assert_eq!(match_filter("true"), true);
    assert_eq!(match_filter("false"), false);
}

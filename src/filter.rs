use std::rc::Rc;

use crate::args::FilterArgs;
use crate::dateexp::{eval, parse_local_exp};
use crate::entry::{Entry, FieldRef};
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
    /// Compare entry properties to the filter.
    pub fn match_issue(&self, entry: &Entry, app: &App, stack: &mut Vec<Token>) -> Result<bool> {
        if self.expression.is_empty() {
            return Ok(true);
        }

        let res = eval(&self.expression, app.local_time()?, stack, entry)?;
        match res {
            Token::Bool(res) => Ok(res),
            Token::Date(_) | Token::Duration(_) => Ok(true),
            Token::String(s) => Ok(entry.desc.contains(&*s)),
            Token::Regex(r) => Ok(r.is_match(&entry.desc)),
            _ => bail!(
                "Filter expression produced output which can't be matched against entries ({})",
                res.ttype()
            ),
        }
    }

    /// Replace filter query re-using the vec.
    pub fn replace(&mut self, expr: &str, app: &App) -> Result<()> {
        self.expression.clear();
        parse_local_exp(expr, app, &mut self.expression)
    }

    /// Append '&&' condition on top of the query.
    #[inline]
    pub fn merge(&mut self, merger: impl FnOnce(&mut Vec<Token>)) {
        let was_empty = self.expression.is_empty();
        merger(&mut self.expression);

        if !was_empty {
            self.expression.push(Token::And);
        }
    }
}

/// Parse filter expressions and combine these with 'and' operator.
pub fn merge_filter_args(filter: &mut QueryFilter, args: &FilterArgs, app: &App) -> Result<()> {
    let expression = &mut filter.expression;

    for expr in &args.filter {
        let append = !expression.is_empty();
        parse_local_exp(expr, app, expression)?;

        if append {
            expression.push(Token::And)
        }
    }

    for title in &args.title {
        let token = if title.starts_with('/') && title.ends_with('/') && title.len() > 1 {
            let slice = &title[1..(title.len() - 1)];
            Token::Regex(Rc::from(regex::Regex::new(slice)?))
        } else {
            Token::String(Rc::from(title.as_str()))
        };

        filter.merge(|e| {
            e.push(Token::Reference(FieldRef::Title));
            e.push(token);
            e.push(Token::FuzzyEq);
        });
    }

    for desc in &args.desc {
        let token = if desc.starts_with('/') && desc.ends_with('/') && desc.len() > 1 {
            let slice = &desc[1..(desc.len() - 1)];
            Token::Regex(Rc::from(regex::Regex::new(slice)?))
        } else {
            Token::String(Rc::from(desc.as_str()))
        };

        filter.merge(|e| {
            e.push(Token::Reference(FieldRef::Desc));
            e.push(token);
            e.push(Token::FuzzyEq);
        });
    }

    for status in &args.status {
        filter.merge(|e| {
            e.push(Token::Reference(FieldRef::Status));
            e.push(Token::String(Rc::from(status.as_ref())));
            e.push(Token::FuzzyEq);
        });
    }

    for tag in &args.tag {
        if let Some(tag) = tag.strip_prefix("-") {
            filter.merge(|e| {
                e.push(Token::Reference(FieldRef::Tag));
                e.push(Token::String(Rc::from(tag)));
                e.push(Token::FuzzyEq);
                e.push(Token::Not);
            });
        } else {
            filter.merge(|e| {
                e.push(Token::Reference(FieldRef::Tag));
                e.push(Token::String(Rc::from(tag.as_str())));
                e.push(Token::FuzzyEq);
            });
        }
    }

    // TODO: P3: add due, end, created and modified filters

    Ok(())
}

/// Store provided list of IDs as index.
#[derive(Default)]
pub struct IdFilter {
    pub index: Vec<Box<str>>,

    /// Flag if ID filter was provided.
    pub enabled: bool,

    /// Flag if there was any unresolved shorthands. If query contained
    /// only shorthands, it's safe to only check the shorthands index.
    pub only_active: bool,
}

impl IdFilter {
    /// Convert list of IDs with shorthands into a set of fully resolved IDs.
    pub fn from_shorthands(ids: Vec<Box<str>>, app: &App) -> Result<Self> {
        let mut out = Self::default();
        out.append_shorthands(ids, app)?;

        Ok(out)
    }

    /// Convert shorthands list to resolved UUIDs.
    pub fn append_shorthands(&mut self, ids: Vec<Box<str>>, app: &App) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        self.enabled = true;

        let index = app.index()?;
        self.only_active = self.index.is_empty() || self.only_active;

        for id in ids {
            let shorthand = unwrap_ok_or!(id.parse::<usize>(), _e, {
                self.index.push(id);
                self.only_active = false;
                continue;
            });
            let pointer = unwrap_some_or!(index.active().get(shorthand - 1), {
                self.index.push(id);
                self.only_active = false;
                continue;
            });
            let (_, resolved) = unwrap_some_or!(pointer.rsplit_once("/"), {
                warn!("Index entry with missing path: {pointer}");
                continue;
            });

            self.index.push(resolved.into());
        }

        Ok(())
    }

    /// Check if ID filter matches the ID.
    pub fn matches(&self, value: &str) -> bool {
        self.index.is_empty() || self.index.iter().any(|id| value.starts_with(id.as_ref()))
    }

    /// Check if ID set was provided, but no entries was resolved.
    pub fn empty_set(&self) -> bool {
        self.index.is_empty() && self.enabled
    }
}

#[test]
fn match_issue() {
    use std::collections::BTreeSet;

    let app = Default::default();
    let mut tags = BTreeSet::<String>::default();
    tags.extend(["a", "b", "c"].map(Into::into).into_iter());

    let issue = Entry {
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

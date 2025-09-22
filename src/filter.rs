use std::rc::Rc;

use crate::args::FilterArgs;
use crate::config::query::IndexType;
use crate::datecalc::parse::parse_local_exp;
use crate::datecalc::token::Token;
use crate::datecalc::{eval::eval, parse::parse_filter};
use crate::entry::{Entry, EntryPath, FieldRef};
use crate::{app::App, prelude::*};

/// Set of ID and query based filters.
pub struct Filter<'a> {
    /// Match specific IDs.
    pub ids: &'a IdFilter,

    /// Filter by query expression.
    pub query: &'a QueryFilter,
}

/// List of filter rules.
#[derive(Default)]
pub struct QueryFilter {
    /// Match expression to eval on entries.
    expression: Vec<Token>,

    /// Group by calculated value when grouping is enabled in the report.
    group_by: Vec<Token>,

    /// Index type expected to use with expression.
    index: IndexType,
}

impl QueryFilter {
    /// Compare entry properties to the filter.
    pub fn match_issue(&self, entry: &Entry, app: &App, stack: &mut Vec<Token>) -> Result<bool> {
        if self.expression.is_empty() {
            return Ok(true);
        }

        let res = eval(&self.expression, app.local_time()?, stack, entry, app)?;
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
        parse_filter(expr, app, &mut self.expression, |_, _| {})
    }

    /// Replace group query while re-using the vec.
    pub fn replace_group(&mut self, expr: &str, app: &App) -> Result<()> {
        self.group_by.clear();
        parse_local_exp(expr, app, &mut self.group_by)
    }

    /// Clear group when grouping is not needed.
    pub fn clear_group(&mut self) {
        self.group_by.clear();
    }

    /// Evaluate group query and produce a value.
    pub fn _eval_group(&mut self) -> Token {
        // TODO: P3: produce group value
        todo!()
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

    /// Return current query index type.
    pub fn index(&self) -> IndexType {
        self.index
    }
}

/// Parse filter expressions and combine these with 'and' operator.
pub fn merge_filter_args(filter: &mut QueryFilter, args: &FilterArgs, app: &App) -> Result<()> {
    let expression = &mut filter.expression;

    let and_merge = |o: &mut Vec<_>, merge| {
        if merge {
            o.push(Token::And)
        }
    };

    let in_merge = |o: &mut Vec<_>, merge, field| {
        o.push(Token::Reference(field));
        o.push(Token::In);
        if merge {
            o.push(Token::And);
        }
    };

    for expr in &args.filter {
        parse_filter(expr, app, expression, and_merge)
            .with_context(|| format!("Unable to parse filter: '{expr}'"))?;
    }

    if let Some(query) = &args.query {
        let query_data = app.config.query(query)?;
        filter.index = query_data.index;
        parse_filter(query_data.filter, app, expression, and_merge)
            .with_context(|| format!("Unable to parse query filter: '{}'", query_data.filter))?;
    }

    for when in &args.when {
        parse_filter(when, app, expression, |o, m| in_merge(o, m, FieldRef::When))
            .with_context(|| format!("Unable to parse planned date: '{when}'"))?;
    }

    for due in &args.due {
        parse_filter(due, app, expression, |o, m| in_merge(o, m, FieldRef::Due))
            .with_context(|| format!("Unable to parse due date: '{due}'"))?;
    }

    for end in &args.end {
        parse_filter(end, app, expression, |o, m| in_merge(o, m, FieldRef::End))
            .with_context(|| format!("Unable to parse end date: '{end}'"))?;
    }

    for created in &args.created {
        parse_filter(created, app, expression, |o, m| {
            in_merge(o, m, FieldRef::Created)
        })
        .with_context(|| format!("Unable to parse created date: '{created}'"))?;
    }

    for modified in &args.modified {
        parse_filter(modified, app, expression, |o, m| {
            in_merge(o, m, FieldRef::Modified)
        })
        .with_context(|| format!("Unable to parse modified date: '{modified}'"))?;
    }

    for title in &args.title {
        let token = if title.starts_with('/') && title.ends_with('/') && title.len() > 1 {
            let slice = &title[1..(title.len() - 1)];
            Token::Regex(Rc::from(
                regex::Regex::new(slice).context("Unable to parse regex filter")?,
            ))
        } else {
            Token::String(Rc::from(title.as_str()))
        };

        filter.merge(|e| {
            e.push(Token::Reference(FieldRef::Title));
            e.push(token);
            e.push(Token::Contains);
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
            e.push(Token::Contains);
        });
    }

    for status in &args.status {
        filter.merge(|e| {
            e.push(Token::Reference(FieldRef::Status));
            e.push(Token::String(Rc::from(status.as_ref())));
            e.push(Token::Contains);
        });
    }

    for tag in &args.tag {
        if let Some(tag) = tag.strip_prefix("-") {
            filter.merge(|e| {
                e.push(Token::Reference(FieldRef::Tag));
                e.push(Token::String(Rc::from(tag)));
                e.push(Token::Contains);
                e.push(Token::Not);
            });
        } else {
            filter.merge(|e| {
                e.push(Token::Reference(FieldRef::Tag));
                e.push(Token::String(Rc::from(tag.as_str())));
                e.push(Token::Contains);
            });
        }
    }

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
                if id.len() < 4 {
                    bail!("Entry with shorthand {shorthand} not found in index");
                }
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

    /// Iterate over entries and check if there's any which matches at least two of provided IDs.
    pub fn check_ambiguity(&self, entries: &[EntryPath]) -> bool {
        if entries.len() < 2 {
            return false;
        }
        if !self.enabled {
            return true;
        }
        if self.only_active {
            return false;
        }

        for filter in &self.index {
            let mut lookup = entries.iter().filter(|(e, _)| e.id.starts_with(&**filter));
            if lookup.nth(1).is_some() {
                return true;
            }
        }

        false
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
        parse_filter(input, &app, &mut exp, |_, _| {}).unwrap();

        QueryFilter {
            expression: exp,
            ..Default::default()
        }
        .match_issue(&issue, &app, &mut Vec::new())
        .unwrap()
    };

    assert_eq!(match_filter("true"), true);
    assert_eq!(match_filter("false"), false);

    assert_eq!(match_filter("when == false"), true);
    assert_eq!(match_filter("due == false"), true);
    assert_eq!(match_filter("created != false"), true);
}

#[test]
fn check_ambiguity() {
    let ids = |ids: &[&str]| IdFilter {
        index: ids.iter().map(|id| Into::<Box<str>>::into(*id)).collect(),
        only_active: false,
        enabled: true,
    };

    let entry = |id: &str| {
        (
            Entry {
                id: id.into(),
                ..Default::default()
            },
            Rc::from("test"),
        )
    };

    assert!(ids(&["abc", "cde"]).check_ambiguity(&[entry("abcd"), entry("abce")]));
    assert!(!ids(&["abc", "cde"]).check_ambiguity(&[entry("abcd"), entry("cdef")]));
}

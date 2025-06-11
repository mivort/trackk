use regex::Regex;

use crate::args::Args;
use crate::dateexp::parse_date;
use crate::issue::Issue;
use crate::{App, prelude::*};

/// List of filter rules.
#[derive(Default)]
pub struct Filter {
    /// List of IDs to include in the result.
    pub ids: Vec<String>,

    /// List of IDs to exclude from results.
    exclude_ids: Vec<String>,

    /// Positive filtering rules.
    positive: Vec<Vec<FilterRule>>,

    /// Negative filtering rules.
    exclude: Vec<FilterRule>,
}

/// Single filtering rule.
#[allow(unused)]
pub enum FilterRule {
    Tag(String),
    Status(String),
    DueBefore(i64),
    DueAfter(i64),
    EndBefore(i64),
    EndAfter(i64),
    Title(String),
    TitleRegex(Regex),
    Repeat,
}

impl Filter {
    /// Compare issue properties to the filter.
    pub fn match_issue(&self, issue: &Issue) -> bool {
        if !self.ids.is_empty() && !self.ids.iter().any(|id| issue.id.starts_with(id)) {
            return false;
        }

        if !self.exclude_ids.is_empty()
            && self.exclude_ids.iter().any(|id| issue.id.starts_with(id))
        {
            return false;
        }

        for group in &self.positive {
            for rule in group {
                if !rule.match_issue(issue) {
                    return false;
                }
            }
        }

        for rule in &self.exclude {
            if rule.match_issue(issue) {
                return false;
            }
        }

        true
    }

    /// Parse single argument, return 'true' on success.
    fn parse_positive_arg(&mut self, arg: &str, app: &App) -> Result<()> {
        let mut entry = Vec::new();
        for part in arg.split(',') {
            if let Some(rule) = FilterRule::from_str(part, app)? {
                entry.push(rule);
            } else {
                let id = resolve_shorthand(part, app)?;
                if !id.is_empty() {
                    self.ids.push(id);
                }
            }
        }
        self.positive.push(entry);

        Ok(())
    }

    /// Parse single exclude filter argument.
    fn parse_negative_arg(&mut self, arg: &str, app: &App) -> Result<()> {
        for part in arg.split(',') {
            if let Some(rule) = FilterRule::from_str(part, app)? {
                self.exclude.push(rule);
            } else {
                let id = resolve_shorthand(part, app)?;
                if !id.is_empty() {
                    self.exclude_ids.push(id);
                }
            }
        }

        Ok(())
    }
}

impl FilterRule {
    /// Parse rule string and produce rule enum value.
    fn from_str(rule: &str, app: &App) -> Result<Option<Self>> {
        let rule = rule.trim();

        let mut split = rule.splitn(2, ':');
        let (key, value) = (split.next(), split.next());
        if let (Some(key), Some(value)) = (key, value) {
            match key {
                "" => return Ok(Some(Self::Tag(value.to_owned()))),
                "x" | "not" => todo!("exclude tag option is nyi"),
                "due" | "d" => return Ok(None),
                "end" | "e" => return Ok(None),
                "due.before" | "d.before" => {
                    return Ok(Some(Self::DueBefore(parse_date(value, app)?)));
                }
                "due.after" | "d.after" => {
                    return Ok(Some(Self::DueAfter(parse_date(value, app)?)));
                }
                "end.before" | "e.before" => {
                    return Ok(Some(Self::EndBefore(parse_date(value, app)?)));
                }
                "end.after" | "e.after" => {
                    return Ok(Some(Self::EndAfter(parse_date(value, app)?)));
                }
                "title" | "t" => return Ok(Some(Self::Title(value.to_owned()))),
                "title.regex" | "title.re" | "t.regex" | "t.re" => {
                    return Ok(Some(Self::TitleRegex(
                        Regex::new(value).context("Unable to parse the filter regex")?,
                    )));
                }
                "status" | "s" => return Ok(Some(Self::Status(value.to_owned()))),
                "repeat" | "r" => return Ok(Some(Self::Repeat)),
                _ => {}
            }
        }

        Ok(None)
    }

    /// Check current enum value and match the issue.
    fn match_issue(&self, issue: &Issue) -> bool {
        match self {
            Self::Tag(tag) => issue.tags.contains(tag),
            Self::Status(status) => issue.status == *status,
            Self::Title(title) => issue.title.contains(title),
            Self::TitleRegex(regex) => regex.is_match(&issue.title),
            _ => false,
        }
    }
}

/// Parse filter argument and return list of IDs (if there's any) and the filter.
pub fn parse_filter_args(_args: &Args, app: &App) -> Result<Filter> {
    let (positive, negative) = (Vec::<String>::new(), &Vec::<String>::new());

    let mut filter = Filter::default();
    for arg in positive {
        // TODO: match aliases

        filter.parse_positive_arg(&arg, app)?;
    }

    for arg in negative {
        filter.parse_negative_arg(arg, app)?;
    }

    Ok(filter)
}

/// Check if value is numeric, and try to match it to the index. If input is detected
/// to be a shorthand, but doesn't match any index entry, return empty string.
pub fn resolve_shorthand(value: &str, app: &App) -> Result<String> {
    let shorthand = unwrap_ok_or!(value.parse::<usize>(), _e, { return Ok(value.to_owned()) });

    if shorthand > 999999 {
        return Ok(value.to_owned());
    }

    let index = app.index()?;
    let pointer = unwrap_some_or!(index.active().get(shorthand - 1), {
        return Ok(String::new());
    });
    let (_, resolved) = unwrap_some_or!(pointer.rsplit_once("/"), {
        return Ok(String::new());
    });

    Ok(resolved.to_owned())
}

/// Store provided list of IDs as index.
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
    pub fn matches(&self, value: &String) -> bool {
        self.index.is_empty() || self.index.iter().any(|id| value.starts_with(id))
    }

    pub fn new() -> Self {
        Self {
            index: Default::default(),
            empty_set: false,
        }
    }
}

#[test]
fn match_issue() {
    use std::collections::HashSet;

    let mut tags = HashSet::<String>::default();
    tags.extend(["a", "b", "c"].map(Into::into).into_iter());

    let issue = Issue {
        tags,
        ..Default::default()
    };

    let filter = Filter {
        positive: vec![vec![FilterRule::Tag("a".into())]],
        ..Default::default()
    };
    assert_eq!(
        filter.match_issue(&issue),
        true,
        "when filter has right tag, match the issue"
    );

    let filter = Filter {
        positive: vec![vec![FilterRule::Tag("d".into())]],
        ..Default::default()
    };
    assert_eq!(
        filter.match_issue(&issue),
        false,
        "when filter doesn't have the right tag, don't match the issue"
    );
}

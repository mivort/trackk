use std::collections::HashSet;

use crate::args::Args;
use crate::{App, prelude::*};

/// List of filter rules.
#[derive(Default)]
pub struct Filter {
    /// List of IDs to include in the result.
    _ids: HashSet<String>,

    /// List of IDs to exclude from results.
    _exclude_ids: HashSet<String>,

    /// Positive filtering rules.
    positive: Vec<Vec<FilterRule>>,

    /// Negative filtering rules.
    exclude: Vec<FilterRule>,
}

/// Single filtering rule.
#[allow(unused)]
pub enum FilterRule {
    Tag(String),
    DueBefore(i64),
    DueAfter(i64),
    EndBefore(i64),
    EndAfter(i64),
    Repeat,
}

impl Filter {
    /// Parse single argument, return 'true' on success.
    fn parse_positive_arg(&mut self, arg: &str, _app: &App) -> Result<()> {
        let mut entry = Vec::new();
        for part in arg.split(',') {
            // TODO: match id
            if let Some(rule) = FilterRule::from_str(part) {
                entry.push(rule);
            } else {
                bail!("Unable to parse narrowing rule: {}", part);
            }
        }
        self.positive.push(entry);

        Ok(())
    }

    /// Parse single exclude filter argument.
    fn parse_negative_arg(&mut self, arg: &str, _app: &App) -> Result<()> {
        for part in arg.split(',') {
            if let Some(rule) = FilterRule::from_str(part) {
                self.exclude.push(rule);
            } else {
                bail!("Unable to parse exclude rule: {}", part);
            }
        }

        Ok(())
    }
}

impl FilterRule {
    /// Parse rule string and produce rule enum value.
    fn from_str(rule: &str) -> Option<Self> {
        let rule = rule.trim();

        if rule.starts_with('@') {
            return Some(FilterRule::Tag(rule[1..rule.len()].to_owned()));
        }

        let mut split = rule.splitn(2, ':');
        let (key, value) = (split.next(), split.next());
        if let (Some(key), Some(_value)) = (key, value) {
            match key {
                "due" | "d" => return None,
                "end" | "e" => return None,
                "due.before" | "d.before" => return Some(FilterRule::DueBefore(0)),
                "due.after" | "d.after" => return Some(FilterRule::DueAfter(0)),
                "end.before" | "e.before" => return Some(FilterRule::EndBefore(0)),
                "end.after" | "e.after" => return Some(FilterRule::EndAfter(0)),
                "repeat" | "r" => return Some(FilterRule::Repeat),
                _ => {}
            }
        }

        None
    }
}

/// Parse filter argument and return list of IDs (if there's any) and the filter.
pub fn parse_filter_args(args: &Args, app: &App) -> Result<Filter> {
    let (positive, negative) = (&args.filter, &args.filter_args.exclude);

    let mut filter = Filter::default();
    for arg in positive {
        // TODO: match aliases

        filter.parse_positive_arg(arg, app)?;
    }

    for arg in negative {
        filter.parse_negative_arg(arg, app)?;
    }

    Ok(filter)
}

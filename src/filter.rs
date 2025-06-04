use std::collections::HashSet;

use crate::prelude::*;

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
pub enum FilterRule {
    #[allow(unused)]
    Tag(String),
    _DueBefore(i64),
    _DueAfter(i64),
    _EndBefore(i64),
    _EndAfter(i64),
    _Repeat,
}

impl Filter {
    /// Parse single argument, return 'true' on success.
    fn parse_positive_arg(&mut self, arg: &str) -> Result<()> {
        let mut entry = Vec::new();
        for part in arg.split(',') {
            // TODO: match id
            if let Some(rule) = FilterRule::from_str(part) {
                entry.push(rule);
            } else {
                bail!("Unable to parse narrowing rule: {}", arg);
            }
        }
        self.positive.push(entry);

        Ok(())
    }

    /// Parse single exclude filter argument.
    fn parse_negative_arg(&mut self, arg: &str) -> Result<()> {
        for part in arg.split(',') {
            if let Some(rule) = FilterRule::from_str(part) {
                self.exclude.push(rule);
            } else {
                bail!("Unable to parse exclude rule: {}", arg);
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

        None
    }
}

/// Parse filter argument and return list of IDs (if there's any) and the filter.
pub fn parse_filter_args(
    positive: &[String],
    negative: &[String],
) -> Result<(Filter, Vec<String>)> {
    let mut filter = Filter::default();
    for arg in positive {
        // TODO: match aliases

        filter.parse_positive_arg(arg)?;
    }

    for arg in negative {
        filter.parse_negative_arg(arg)?;
    }

    Ok((filter, Default::default()))
}

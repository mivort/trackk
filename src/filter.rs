use crate::prelude::*;

/// List of filter rules.
#[derive(Default)]
pub struct Filter {
    _rules: Vec<FilterRule>,
}

/// Single filter narrowing rule.
pub enum FilterRule {
    _Tag(String),
    _NoTag(String),
    _DueBefore(i64),
    _DueAfter(i64),
}

impl Filter {
    /// Parse single argument, return 'true' on success.
    pub fn parse_arg(&mut self, _arg: &str) -> Result<bool> {
        Ok(true)
    }
}

/// Parse filter argument and return list of IDs (if there's any) and the filter.
pub fn parse_filter_args(args: &[String]) -> Result<(Filter, Vec<String>)> {
    let mut filter = Filter::default();
    for arg in args {
        if filter.parse_arg(arg)? {
            continue;
        }
    }

    Ok((filter, Default::default()))
}

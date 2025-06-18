use std::rc::Rc;

use crate::issue::Issue;
use crate::prelude::*;

/// Parse sorting expression and sort entries in the provided array.
pub fn sort_entries(_entries: &mut Vec<(Issue, Rc<str>)>, rules: &str) -> Result<()> {
    let _rules = parse_rules(rules);

    // TODO: P3: implement entry sorting

    Ok(())
}

/// Iterate over rule directives and produce array of rules.
fn parse_rules(_rules: &str) -> Vec<SortingRule> {
    Vec::new()
}

/// Single sorting rule applied on comparison.
enum SortingRule {
    _UrgencyAsc,
    _UrgencyDesc,
    _TitleAsc,
    _TitleDesc,
    _CreatedAsc,
    _CreatedDesc,
    _ModifiedAsc,
    _ModifiedDesc,
    _EndAsc,
    _EndDesc,
    _DueAsc,
    _DueDesc,
    _MetaAsc(String),
    _MetaDesc(String),
}

use logos::Logos;
use std::rc::Rc;

use crate::issue::Issue;
use crate::prelude::*;

/// Parse sorting expression and sort entries in the provided array.
pub fn sort_entries(entries: &mut [(Issue, Rc<str>)], rules: &str) -> Result<()> {
    use SortToken::*;

    let _rules = parse_rules(rules);

    // TODO: P3: implement entry sorting

    for (tok, _span) in SortToken::lexer(rules).spanned() {
        match tok {
            Ok(SortAsc) => {}
            Ok(SortDesc) => {}
            Ok(Field) => {}
            _ => {}
        }
    }

    entries.sort_by(|(a, _), (b, _)| a.short.cmp(&b.short));

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

/// Sorting expression tokens.
#[derive(Clone, Copy, Debug, Logos)]
#[logos(skip r"[ \t\n\f]+")]
enum SortToken {
    #[token("+")]
    SortAsc,

    #[token("-")]
    SortDesc,

    #[regex(r"\w+")]
    Field,
}

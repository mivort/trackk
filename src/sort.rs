use logos::Logos;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::issue::Issue;
use crate::prelude::*;

/// Parse sorting expression and sort entries in the provided array.
pub fn sort_entries(entries: &mut [(Issue, Rc<str>)], rules: &[SortingRule]) -> Result<()> {
    entries.sort_by(|(a, _), (b, _)| {
        let mut cmp = Ordering::Equal;
        for r in rules {
            cmp = r.compare(a, b);
            if cmp != Ordering::Equal {
                break;
            }
        }
        cmp
    });

    Ok(())
}

/// Iterate over rule directives and produce array of rules.
pub fn parse_rules(rule: &str) -> Result<Vec<SortingRule>> {
    use SortToken::*;

    let mut res = Vec::new();
    let mut ascending: Option<bool> = None;

    for (tok, span) in SortToken::lexer(rule).spanned() {
        match tok {
            Ok(SortAsc) => {
                if ascending.is_some() {
                    bail!("Unexpected '+' in sorting rule: {rule}");
                }
                ascending = Some(true);
            }
            Ok(SortDesc) => {
                if ascending.is_some() {
                    bail!("Unexpected '-' in sorting rule: {rule}");
                }
                ascending = Some(false);
            }
            Ok(Field) => {
                let asc = unwrap_some_or!(ascending, {
                    bail!("Unexpected field '{}' in sorting rule: {rule}", &rule[span]);
                });
                res.push(SortingRule::from_str(&rule[span], asc));
                ascending = None;
            }
            _ => {
                bail!("Sorting rule parsing error: {rule}");
            }
        }
    }

    Ok(res)
}

/// Single sorting rule applied on comparison.
#[derive(Debug, PartialEq, Eq)]
pub enum SortingRule {
    UrgencyAsc,
    UrgencyDesc,
    TitleAsc,
    TitleDesc,
    CreatedAsc,
    CreatedDesc,
    ModifiedAsc,
    ModifiedDesc,
    EndAsc,
    EndDesc,
    DueAsc,
    DueDesc,
    IdAsc,
    IdDesc,
    TagsAsc,
    TagsDesc,
    _MetaAsc(String), // TODO: P2: implement meta fields sorting
    _MetaDesc(String),
}

impl SortingRule {
    /// Convert name reference to rule value.
    fn from_str(id: &str, asc: bool) -> SortingRule {
        use SortingRule::*;

        match (id, asc) {
            ("title", true) => TitleAsc,
            ("title", false) => TitleDesc,
            ("urgency", true) => UrgencyAsc,
            ("urgency", false) => UrgencyDesc,
            ("created", true) => CreatedAsc,
            ("created", false) => CreatedDesc,
            ("modified", true) => ModifiedAsc,
            ("modified", false) => ModifiedDesc,
            ("due", true) => DueAsc,
            ("due", false) => DueDesc,
            ("end", true) => EndAsc,
            ("end", false) => EndDesc,
            ("tags", true) => TagsAsc,
            ("tags", false) => TagsDesc,
            ("id", true) => IdAsc,
            ("id", false) => IdDesc,

            _ => todo!(), // TODO: P2: support custom field sorting
        }
    }

    /// Compare two fields.
    fn compare(&self, a: &Issue, b: &Issue) -> Ordering {
        match self {
            Self::TitleAsc => a.title.cmp(&b.title),
            Self::TitleDesc => b.title.cmp(&a.title),

            Self::CreatedAsc => a.created.cmp(&b.created),
            Self::CreatedDesc => b.created.cmp(&a.created),

            Self::ModifiedAsc => a.modified.cmp(&b.modified),
            Self::ModifiedDesc => b.modified.cmp(&a.modified),

            Self::DueAsc => a.due.cmp(&b.due),
            Self::DueDesc => b.due.cmp(&a.due),

            Self::EndAsc => a.end.cmp(&b.end),
            Self::EndDesc => b.end.cmp(&a.end),

            Self::UrgencyAsc => a.created.cmp(&b.created), // TODO: P3: compare urgency, not created
            Self::UrgencyDesc => b.created.cmp(&a.created),

            Self::IdAsc => a.id.cmp(&b.id),
            Self::IdDesc => b.id.cmp(&a.id),

            Self::TagsAsc => a.tags.len().cmp(&b.tags.len()),
            Self::TagsDesc => b.tags.len().cmp(&a.tags.len()),

            Self::_MetaAsc(_) | Self::_MetaDesc(_) => todo!(),
        }
    }
}

/// Sorting expression tokens.
#[derive(Clone, Copy, Debug, Logos)]
#[logos(skip r"[ \t\n\f,]+")]
enum SortToken {
    #[token("+")]
    SortAsc,

    #[token("-")]
    SortDesc,

    #[regex(r"\w+")]
    Field,
}

#[test]
fn parse_sorter() {
    use SortingRule::*;

    let rule = "+created -urgency +due";
    let rules = parse_rules(rule).unwrap();
    assert_eq!(rules, vec![CreatedAsc, UrgencyDesc, DueAsc]);
}

#[test]
fn parse_fail() {
    assert!(parse_rules("++created").is_err());
    assert!(parse_rules("created").is_err());
    assert!(parse_rules("@created").is_err());
}

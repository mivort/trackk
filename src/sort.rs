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
    let mut field: Option<(usize, usize)> = None;

    for (tok, span) in SortToken::lexer(rule).spanned() {
        match tok {
            Ok(SortAsc) => {
                let f = unwrap_some_or!(field, {
                    bail!("Unexpected '+' in sorting rule: '{rule}'");
                });
                res.push(SortingRule::from_str(&rule[f.0..f.1], true));
                field = None;
            }
            Ok(SortDesc) => {
                let f = unwrap_some_or!(field, {
                    bail!("Unexpected '-' in sorting rule: {rule}");
                });
                res.push(SortingRule::from_str(&rule[f.0..f.1], false));
                field = None;
            }
            Ok(Field) => {
                if field.is_some() {
                    bail!("Unexpected field '{}' in sorting rule: {rule}", &rule[span]);
                }
                field = Some((span.start, span.end));
            }
            _ => {
                bail!("Sorting rule parsing error: '{rule}'");
            }
        }
    }

    if let Some(field) = field {
        bail!(
            "Sorting direction (+/-) is not specified for '{}'",
            &rule[field.0..field.1]
        )
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

            Self::DueAsc => a.due.unwrap_or(i64::MAX).cmp(&b.due.unwrap_or(i64::MAX)),
            Self::DueDesc => b.due.unwrap_or(i64::MAX).cmp(&a.due.unwrap_or(i64::MAX)),

            Self::EndAsc => a.end.unwrap_or(i64::MAX).cmp(&b.end.unwrap_or(i64::MAX)),
            Self::EndDesc => b.end.unwrap_or(i64::MAX).cmp(&a.end.unwrap_or(i64::MAX)),

            Self::UrgencyAsc => a.urgency.partial_cmp(&b.urgency).unwrap_or(Ordering::Equal),
            Self::UrgencyDesc => b.urgency.partial_cmp(&a.urgency).unwrap_or(Ordering::Equal),

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

    let rule = "created+ urgency- due+";
    let rules = parse_rules(rule).unwrap();
    assert_eq!(rules, vec![CreatedAsc, UrgencyDesc, DueAsc]);
}

#[test]
fn parse_fail() {
    assert!(parse_rules("+created").is_err());
    assert!(parse_rules("created++").is_err());
    assert!(parse_rules("created").is_err());
    assert!(parse_rules("@created").is_err());
}

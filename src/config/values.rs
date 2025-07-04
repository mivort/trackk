use std::borrow::Cow;
use std::collections::HashSet;

use serde_derive::Deserialize;

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct ValuesConfig {
    /// Default status to assign upon creation.
    #[serde(default)]
    initial_status: Box<str>,

    /// List of statuses which are considered as 'active'.
    #[serde(default)]
    pub active_status: HashSet<String>,

    /// When task is marked this status, check if it should be repeated.
    #[serde(default)]
    pub repeat_status: Vec<Box<str>>,

    /// Only allow to assign tags from this list. Allow any tag if empty.
    #[serde(default)]
    pub _permit_tags: HashSet<String>, // TODO: P1: support list of permitted tags

    /// Only allow one of the provided statuses. Don't check status if empty.
    #[serde(default)]
    pub permit_status: Vec<Box<str>>,

    /// Urgency formula to use on entries.
    #[serde(default)]
    pub urgency_formula: Box<str>,

    /// Perform validation query when entry is created or modified.
    #[serde(default)]
    _validation_query: Box<str>, // TODO: P1: support validaton queries.

    /// Default time string to assign as 'when'.
    #[serde(default)]
    _assign_when: Box<str>, // TODO: P2: support default when value

    /// Default time string to assign as 'due'.
    #[serde(default)]
    _assign_due: Box<str>, // TODO: P2: support default due value
}

impl ValuesConfig {
    /// Default urgency formula string.
    pub fn urgency_formula(&self) -> &str {
        if self.urgency_formula.is_empty() {
            return concat!(
                concat!(
                    "(",
                    stringify!(sig((now - (due or when or someday)) / 10mil) * (10 if due else 5)),
                    concat!(" + ", stringify!(sig((now - created) / 10mil) * 0.5)),
                    ")",
                    " * ",
                    stringify!((0 if end else 1)), // Only apply due/created if end is not set
                ),
                concat!(
                    " - ",
                    // Reduce urgency for older complete tasks
                    stringify!((sig((now - (end or now)) / 10mil) * 2 - 0.5 if end else 0))
                ),
                " + (1 if status == started else 0)",
                " + (-1 if status == blocked else 0)",
                " + (-20 if status == deleted else 0)",
            );
        }
        &self.urgency_formula
    }

    /// Produce list of statuses which will trigger the repeat property to produce a copy.
    pub fn repeat_status(&self) -> Cow<[Box<str>]> {
        if self.repeat_status.is_empty() {
            Cow::Owned(vec!["completed".into()])
        } else {
            Cow::Borrowed(&self.repeat_status)
        }
    }

    /// Status which is assigned by default when entry is created.
    pub fn initial_status(&self) -> &str {
        if self.initial_status.is_empty() { "pending" } else { &self.initial_status }
    }
}

use std::borrow::Cow;
use std::collections::HashSet;

use serde_derive::Deserialize;

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct ValuesConfig {
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
}

impl ValuesConfig {
    /// Default urgency formula string.
    pub fn urgency_formula(&self) -> &str {
        if self.urgency_formula.is_empty() {
            return concat!(
                concat!(
                    "(",
                    stringify!(sig((now - (due or when or someday)) / 10mil) * (has(due) and 10 or 5)),
                    concat!(" + ", stringify!(sig((now - created) / 10mil) * 0.5)),
                    ")",
                    " * ",
                    stringify!((end:false and 1 or 0)), // Only apply due/created if end is not set
                ),
                concat!(
                    " - ",
                    stringify!((end:false and 0 or (sig((now - (end or now)) / 10mil) - 0.25) * 2))
                ),
                " + (status == started and 1 or 0)",
                " + (status == blocked and -1 or 0)",
                " + (status == deleted and -20 or 0)",
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
}

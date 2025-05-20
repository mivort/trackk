use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::args::{EntryArgs, FilterArgs};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Issue {
    /// Entry unique ID used for merging.
    pub id: String,

    /// Issue main title.
    pub title: String,

    /// List of issue's tags.
    #[serde(default)]
    pub tags: HashSet<String>,

    /// Entry status string.
    #[serde(default)]
    pub status: String,

    /// Repeat string which is applied to task copy upon completion.
    #[serde(default)]
    pub repeat: String,

    /// Creation date/time.
    #[serde(default)]
    pub created: i64,

    /// Last modify timestamp.
    #[serde(default)]
    pub modified: i64,

    /// Due date/time.
    #[serde(default)]
    pub due: Option<i64>,

    /// Last status change timestamp.
    #[serde(default)]
    pub complete: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Bucket {
    /// Storage bucket schema version.
    pub version: i64,

    /// Entry IDs.
    pub entries: Vec<Issue>,
}

impl Bucket {
    const VERSION: i64 = 1;

    pub fn new() -> Self {
        Self {
            version: Self::VERSION,
            entries: Default::default(),
        }
    }
}

impl Issue {
    /// Take values from provided arguments and apply to the issue.
    pub fn apply_args(&mut self, args: &EntryArgs) {
        if let Some(title) = &args.title {
            self.title = title.clone();
        }
        if let Some(status) = &args.status {
            self.status = status.clone();
        }
        if let Some(repeat) = &args.repeat {
            self.repeat = repeat.clone();
        }
    }

    /// Compare issue properties to provided filter.
    pub fn match_filter(&self, filter: &FilterArgs) -> bool {
        if !filter.has_status.is_empty() && !filter.has_status.contains(&self.status) {
            return false;
        }

        for tag in &filter.has_tag {
            if !self.tags.contains(tag) {
                return false;
            }
        }

        true
    }
}

impl Bucket {
    /// Fetch the reference to a bucket entry.
    pub fn find_by_id(&self, id: &str) -> Option<&Issue> {
        // TODO: bucket is sorted by id in most cases - attempt to find the issue
        // with a binary search.

        self.entries.iter().find(|&issue| issue.id.starts_with(id))
    }

    /// Consume bucket to move out a single entry.
    pub fn take_by_id(self, id: &str) -> Option<Issue> {
        self.entries.into_iter().find(|i| i.id.starts_with(id))
    }
}

#[test]
fn issue_matching() {
    let issue = Issue {
        status: "test".into(),
        ..Default::default()
    };

    let filter = FilterArgs {
        has_status: vec!["test".into()],
        ..Default::default()
    };

    assert!(issue.match_filter(&filter), "matches test status");

    let filter = FilterArgs {
        has_status: vec!["test1".into()],
        ..Default::default()
    };

    assert!(
        !issue.match_filter(&filter),
        "doesn't match non-existing status"
    );
}

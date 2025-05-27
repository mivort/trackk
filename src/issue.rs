use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use time::UtcDateTime;
use uuid::Uuid;

use crate::args::{EntryArgs, FilterArgs};
use crate::config::Config;
use crate::prelude::*;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Issue {
    /// Entry unique ID used for merging.
    pub id: String,

    /// Numeric shorthand.
    #[serde(skip)]
    pub short: Option<usize>,

    /// Issue main title.
    #[serde(default)]
    pub title: String,

    /// List of issue's tags.
    #[serde(default)]
    pub tags: HashSet<String>,

    /// Entry status string.
    #[serde(default)]
    pub status: String,

    /// Repeat string which is applied to task copy upon completion.
    #[serde(default)]
    pub repeat: Option<String>,

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
    pub end: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Bucket {
    /// Storage bucket schema version.
    pub version: i64,

    /// List of bucket entries.
    #[serde(default)]
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
    /// Create new entry using provided arguments.
    pub fn new(entry: &EntryArgs, config: &Config) -> Self {
        let new_uuid = Uuid::new_v4().to_string();

        let ts = UtcDateTime::now().unix_timestamp();

        Self {
            id: new_uuid,
            title: entry.title.clone().unwrap_or_default(),
            status: unwrap_some_or!(&entry.status, { &config.defaults.status_initial }).clone(),
            created: ts,
            modified: ts,
            ..Default::default()
        }
    }

    /// Take values from provided arguments and apply to the issue. Also,
    /// update the modified timestamp.
    pub fn apply_args(&mut self, args: &EntryArgs) {
        if let Some(title) = &args.title {
            self.title = title.clone();
        }
        if let Some(status) = &args.status {
            self.apply_status(status);
        }
        if let Some(repeat) = &args.repeat {
            self.repeat = if repeat.is_empty() {
                None
            } else {
                Some(repeat.clone())
            };
        }

        self.update_ts();
    }

    /// Update entry status (and timestamp in case of change).
    pub fn apply_status(&mut self, status: &str) {
        if self.status == status {
            return;
        }
        self.status = status.to_owned();
        self.update_end_ts();
    }

    /// Update timestamp to the current time.
    pub fn update_ts(&mut self) {
        self.modified = UtcDateTime::now().unix_timestamp();
    }

    /// Update status timestamp to the current time.
    pub fn update_end_ts(&mut self) {
        self.end = Some(UtcDateTime::now().unix_timestamp());
    }

    /// Compare issue properties to provided filter.
    pub fn match_filter(&self, filter: &FilterArgs) -> bool {
        if let Some(id) = &filter.id {
            if !self.id.contains(id) {
                return false;
            }
        }

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

    /// Provide cloned entry with shorthand.
    pub fn with_shorthand(&self, short: usize) -> Self {
        let mut new = self.clone();
        new.short = Some(short);
        new
    }
}

impl Bucket {
    /// Fetch the reference to a bucket entry.
    pub fn find_by_id(&self, id: &str) -> Option<&Issue> {
        // TODO: bucket is sorted by id in most cases - attempt to find the issue
        // with a binary search.

        self.entries.iter().find(|&issue| issue.id.starts_with(id))
    }

    /// Fetch the mutable reference to a bucket entry.
    pub fn find_by_id_mut(&mut self, id: &str) -> Option<&mut Issue> {
        self.entries
            .iter_mut()
            .find(|issue| issue.id.starts_with(id))
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

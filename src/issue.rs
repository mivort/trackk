use serde_derive::{Deserialize, Serialize};

use crate::args::EntryArgs;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Issue {
    /// Entry unique ID used for merging.
    pub id: String,

    /// Issue main title.
    pub title: String,

    /// List of issue's tags.
    #[serde(default)]
    pub tags: Vec<String>,

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
}

impl Bucket {
    /// Fetch the reference to a bucket entry.
    pub fn find_by_id(&self, id: &str) -> Option<&Issue> {
        // TODO: bucket is sorted by id in most cases - attempt to find the issue
        // with a binary search.

        for issue in &self.entries {
            if issue.id.starts_with(id) {
                return Some(&issue);
            }
        }
        None
    }

    /// Consume bucket to move a single entry.
    pub fn take_by_id(self, id: &str) -> Option<Issue> {
        for issue in self.entries {
            if issue.id.starts_with(id) {
                return Some(issue);
            }
        }
        None
    }
}

use serde_derive::{Deserialize, Serialize};

use crate::issue::Issue;

/// Storage bucket which groups several entries in a single file.
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

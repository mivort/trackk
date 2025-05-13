use std::collections::HashMap;
use uuid::Uuid;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Issue {
    /// Issue main title.
    pub title: String,

    /// List of issue's tags.
    pub tags: Vec<String>,

    /// Entry status string.
    pub status: String,

    /// Last modify timestamp.
    pub modified: i64,

    /// Creation date/time.
    pub created: i64,

    /// Due date/time.
    pub due: i64,

    /// Last modify timestamp.
    pub status_changed: i64,
}

#[derive(Serialize, Deserialize)]
pub struct Bucket {
    /// Storage bucket schema version.
    pub version: i64,

    /// Entry IDs.
    pub entries: HashMap<Uuid, Issue>,
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

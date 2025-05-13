use std::collections::HashMap;
use uuid::Uuid;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Issue {
    /// Issue main title.
    pub title: String,

    /// Issue description text.
    pub description: String,

    /// Entry status string.
    pub status: String,

    /// Last modify timestamp.
    pub modified: u64,
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

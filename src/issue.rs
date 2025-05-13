use std::collections::HashMap;
use uuid::Uuid;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Issue {
    /// Issue main title.
    title: String,

    /// Issue description text.
    description: String,

    /// Entry status string.
    status: String,

    /// Last modify timestamp.
    modified: u64,
}

#[derive(Serialize, Deserialize)]
pub struct Bucket {
    /// Storage bucket schema version.
    version: i64,

    /// Entry IDs.
    entries: HashMap<Uuid, Issue>,
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

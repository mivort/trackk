use std::collections::HashMap;
use uuid::Uuid;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Issue {
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

    /// Last modify timestamp.
    #[serde(default)]
    pub modified: i64,

    /// Creation date/time.
    #[serde(default)]
    pub created: i64,

    /// Due date/time.
    #[serde(default)]
    pub due: i64,

    /// Last modify timestamp.
    #[serde(default)]
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

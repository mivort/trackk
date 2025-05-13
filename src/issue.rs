use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Issue {
    /// Issue main title.
    title: String,

    /// Issue description text.
    description: String,
}

#[derive(Serialize, Deserialize)]
pub struct Storage {
    /// Storage bucket schema version.
    version: i64,

    /// Entry IDs.
    entries: HashMap<i64, Issue>,
}

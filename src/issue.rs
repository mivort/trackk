use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Issue {
    /// Issue main title.
    title: String,

    /// Issue description text.
    description: String,
}

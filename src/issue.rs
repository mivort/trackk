use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Issue {
    /// Issue main title.
    title: String,

    /// Issue description text.
    description: String,
}

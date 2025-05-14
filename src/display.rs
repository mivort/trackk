use std::rc::Rc;
use uuid::Uuid;

use crate::issue::Issue;

/// Render the list of filtered entries.
pub fn show_entries(entries: &[(Uuid, Issue, Rc<str>)]) {
    for (id, issue, path) in entries {
        println!("{id}: {path}: {}", issue.title);
    }
}

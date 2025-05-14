use std::rc::Rc;

use crate::issue::Issue;

/// Render the list of filtered entries.
pub fn show_entries(entries: &[(Issue, Rc<str>)]) {
    for (issue, path) in entries {
        println!("{id}: {path}: {title}", id = issue.id, title = issue.title);
    }
}

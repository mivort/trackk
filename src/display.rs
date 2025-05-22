use std::rc::Rc;

use crate::issue::Issue;

/// Render the list of filtered entries.
pub fn show_entries(entries: &[(Issue, Rc<str>)]) {
    for (issue, path) in entries {
        let title = issue.title.lines().next().unwrap_or_default();
        println!(
            "[{short}] {id}: {path}: {title}",
            id = issue.id,
            short = issue.short.unwrap_or(0)
        );
    }
}

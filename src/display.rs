use std::rc::Rc;

use crate::issue::Issue;

/// Render the list of filtered entries.
pub fn show_entries(entries: &[(Issue, Rc<str>)]) {
    for (issue, _path) in entries {
        let title = issue.title.lines().next().unwrap_or_default();
        let status = &issue.status[0..1];
        println!(
            " {short}. [{status}] {id}: {title} ({tags})",
            id = issue.id,
            short = issue.short.unwrap_or(0),
            tags = issue
                .tags
                .clone()
                .into_iter()
                .collect::<Vec<String>>()
                .join(" "),
        );
    }
}

/// Render single entry.
pub fn show_entry((issue, path): &(Issue, Rc<str>)) {
    let tags = issue.tags.iter().map(|t| format!("@{}", t));
    let tags = tags.collect::<Vec<_>>().join(" ");

    println!(
        concat!(
            "\n",
            "ID:     {id}\n",
            "--------------------------------------------\n",
            "{title}\n\n",
            "--------------------------------------------\n",
            "Status: {status}\n",
            "Tags:   {tags}\n",
            "Path:   {path}\n"
        ),
        id = &issue.id,
        title = &issue.title,
        status = &issue.status,
        path = path,
        tags = tags,
    )
}

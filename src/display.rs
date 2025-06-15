use std::rc::Rc;

use crate::config::{ReportConfig, SectionConfig};
use crate::issue::Issue;
use crate::{App, prelude::*};

/// Render the list of filtered entries.
pub fn show_entries(entries: &[(Issue, Rc<str>)], report: &ReportConfig, app: &App) {
    // TODO: P3: read report settings and use template

    for section in &report.sections {
        // TODO: P3: move entry fetching
        // TODO: P3: add entry sorting
        show_section(section, app);
    }

    for (issue, _path) in entries {
        let tags = issue.tags.iter().map(|t| format!(":{}", t));
        let tags = tags.collect::<Vec<_>>().join(" ");

        let title = issue.title.lines().next().unwrap_or_default();
        let status = &issue.status.chars().next().unwrap_or('?');
        println!(
            "{short:3}. [{status}] {id}: {title}{tags_space}{tags}",
            id = &issue.id.as_str()[0..8],
            short = issue.short.unwrap_or(0),
            tags_space = if tags.is_empty() { "" } else { " " }
        );
    }
}

/// Apply template and render single output section.
fn show_section(_config: &SectionConfig, _app: &App) {}

/// Render single entry.
pub fn show_entry((issue, path): &(Issue, Rc<str>)) {
    let tags = issue.tags.iter().map(|t| format!("@{}", t));
    let tags = tags.collect::<Vec<_>>().join(" ");

    // TODO: P2: use template for single entry report

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

/// Export entries as JSON.
pub fn show_json(entries: &[(Issue, Rc<str>)]) -> Result<()> {
    print!("[");
    for (i, (e, _)) in entries.iter().enumerate() {
        print!("{}", serde_json::to_string_pretty(e)?);
        if i < entries.len() - 1 {
            print!(",");
        }
    }
    println!("]");

    Ok(())
}

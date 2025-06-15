use std::rc::Rc;

use crate::config::{ReportConfig, SectionConfig};
use crate::filter::IdFilter;
use crate::issue::Issue;
use crate::{App, prelude::*, storage};

/// Render the list of filtered entries.
pub fn show_entries(ids: &IdFilter, report: &ReportConfig, app: &App) -> Result<()> {
    // TODO: P3: read report settings and use template

    for section in &report.sections {
        show_section(ids, section, app)?;
    }

    Ok(())
}

/// Apply template and render single output section.
fn show_section(ids: &IdFilter, section: &SectionConfig, app: &App) -> Result<()> {
    let entries = storage::fetch_entries(ids, section.index, app)?;

    // TODO: P3: add entry sorting
    // TODO: P3: perform lazy parsing of the template

    let _ = app.templates.template()?;

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

    Ok(())
}

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
    // TODO: P2: support JSON in regular reports
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

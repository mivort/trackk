use std::borrow::Cow;
use std::rc::Rc;

use serde_derive::Serialize;

use crate::config::{ReportConfig, SectionConfig};
use crate::filter::IdFilter;
use crate::issue::Issue;
use crate::{app::App, prelude::*, sort, storage};

#[derive(Serialize)]
struct RowContext<'a> {
    /// Shorthand issue reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    sid: Option<usize>,

    /// Calculated issue urgency.
    urgency: f64,

    /// Flag if current row is odd or even.
    lineno: usize,

    /// Reference to the issue data.
    issue: Cow<'a, Issue>,

    /// Path to storage file which contains the entry.
    path: Cow<'a, str>,
}

/// Render the list of filtered entries.
pub fn show_entries<'a>(ids: &IdFilter, report: &'a ReportConfig, app: &App<'a>) -> Result<()> {
    app.templates.init(app)?;

    for section in &report.sections {
        println!("--- {} ---", section.template); // TODO: P3: add header template

        show_section(ids, section, app)?;
    }

    Ok(())
}

/// Apply template and render single output section.
fn show_section<'a>(ids: &IdFilter, section: &'a SectionConfig, app: &App<'a>) -> Result<()> {
    let SectionConfig {
        template,
        index,
        sorting,
        ..
    } = section;

    // TODO: P3: apply report-local filter
    // TODO: P2: propagate sorting override from args

    let mut entries = storage::fetch_entries(ids, *index, app)?;

    let sort = if app.sort.is_empty() { &sort::parse_rules(sorting)? } else { &app.sort };
    sort::sort_entries(&mut entries, sort)?;

    app.templates
        .load_template(template)
        .with_context(|| format!("Unable to load template: {template}"))?;

    let j2 = app.templates.j2.borrow();
    let template = j2.get_template(&section.template)?;
    let out = std::io::stdout();

    for (lineno, (issue, path)) in entries.iter().enumerate() {
        let context = RowContext {
            sid: issue.sid,
            urgency: issue.urgency,
            issue: Cow::Borrowed(issue),
            path: Cow::Borrowed(path),
            lineno,
        };
        template
            .render_to_write(context, &out)
            .with_context(|| format!("Unable to render report template: {}", section.template))?;
    }

    Ok(())
}

/// Render single entry.
pub fn show_entry<'a>((issue, path): &(Issue, Rc<str>), app: &'a App<'a>) -> Result<()> {
    app.templates.init(app)?;

    let template_id = app.config.issue_view();
    app.templates.load_template(template_id)?;

    let j2 = app.templates.j2.borrow();
    let template = j2.get_template(template_id)?;
    let out = std::io::stdout();

    let context = RowContext {
        sid: issue.sid,
        urgency: issue.urgency,
        issue: Cow::Borrowed(issue),
        path: Cow::Borrowed(path),
        lineno: 0,
    };
    template
        .render_to_write(context, &out)
        .with_context(|| format!("Unable to render issue template: {}", template_id))?;

    Ok(())
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

/// Print changes to the entry as the series of info messages.
pub fn show_diff(before: &Issue, after: &Issue) {
    let id = &before.id[..8];
    info!("Issue {id} updated");

    if before.status != after.status {
        info!("Status:   {} -> {}", before.status, after.status);
    }
    if before.title != after.title {
        let before = before.title.len();
        let after = after.title.len();
        info!("Title:    {before} -> {after} bytes");
    }

    if before.tags != after.tags {
        let before = before.tags.len();
        let after = after.tags.len();
        let s = if after > 1 { "s" } else { "" };
        info!("Tags:     {before} -> {after} tag{s}");
    }

    if before.repeat != after.repeat {
        let before = before.repeat.as_deref().unwrap_or("no repeat");
        let after = after.repeat.as_deref().unwrap_or("no repeat");
        info!("Repeat:   {before} -> {after}");
    }

    // TODO: P3: nicer due/end dates format

    if before.due != after.due {
        info!("Due:      {:?} -> {:?}", before.due, after.due);
    }
    if before.end != after.end {
        info!("End:      {:?} -> {:?}", before.end, after.end);
    }
}

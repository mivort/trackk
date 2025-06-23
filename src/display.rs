use std::borrow::Cow;
use std::rc::Rc;

use serde_derive::Serialize;

use crate::config::{ReportConfig, SectionConfig};
use crate::filter::{Filter, IdFilter, QueryFilter};
use crate::issue::Issue;
use crate::templates::dates;
use crate::{app::App, prelude::*, sort, storage};

#[derive(Serialize)]
struct RowContext<'a> {
    /// Shorthand entry reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    sid: Option<usize>,

    /// Calculated entry urgency.
    urgency: f64,

    /// Flag if current row is odd or even.
    lineno: usize,

    /// Number of items in the section.
    count: usize,

    /// Reference to the issue data.
    entry: Cow<'a, Issue>,

    /// Path to storage file which contains the entry.
    path: Cow<'a, str>,
}

#[derive(Serialize)]
struct HeaderContext<'a> {
    /// Section title.
    title: &'a str,

    /// Number of items in the section.
    count: usize,
}

/// Render the list of filtered entries.
pub fn show_entries<'a>(ids: &IdFilter, report: &'a ReportConfig, app: &App<'a>) -> Result<()> {
    app.templates.init(app)?;

    let query = &mut QueryFilter::default();
    let mut filter = Filter { ids, query };
    let mut shown = 0;

    for section in &report.sections {
        shown += show_section(&mut filter, section, app)?;
    }

    if shown == 0 {
        info!("No results");
    }

    Ok(())
}

/// Apply template and render single output section.
/// Return the number of shown entries.
fn show_section<'a>(
    filters: &mut Filter,
    section: &'a SectionConfig,
    app: &App<'a>,
) -> Result<usize> {
    let SectionConfig {
        header,
        template,
        index,
        sorting,
        filter,
        title,
        ..
    } = section;

    // TODO: P2: propagate sorting override from args

    filters
        .query
        .replace(filter, app)
        .with_context(|| format!("Unable to parse filter predicate: '{filter}'"))?;
    let mut entries = storage::fetch_entries(filters, *index, app)?;

    if entries.is_empty() {
        return Ok(0);
    }

    if !header.is_empty() {
        app.templates
            .load_template(header)
            .with_context(|| format!("Unable to load header template: {header}"))?;
    }

    if !template.is_empty() {
        app.templates
            .load_template(template)
            .with_context(|| format!("Unable to load template: {template}"))?;
    }

    let sort = if app.sort.is_empty() { &sort::parse_rules(sorting)? } else { &app.sort };
    sort::sort_entries(&mut entries, sort)?;

    let j2 = app.templates.j2.borrow();
    let out = std::io::stdout();

    if !header.is_empty() {
        let header = j2.get_template(&section.header)?;
        let context = HeaderContext {
            title,
            count: entries.len(),
        };
        header.render_to_write(context, &out).with_context(|| {
            format!(
                "Unable to render report header template: {}",
                section.header
            )
        })?;
    }

    if template.is_empty() {
        return Ok(0);
    }

    let template = j2.get_template(&section.template)?;
    for (lineno, (entry, path)) in entries.iter().enumerate() {
        let context = RowContext {
            sid: entry.sid,
            urgency: entry.urgency,
            entry: Cow::Borrowed(entry),
            path: Cow::Borrowed(path),
            lineno,
            count: entries.len(),
        };
        template
            .render_to_write(context, &out)
            .with_context(|| format!("Unable to render report template: {}", section.template))?;
    }

    Ok(entries.len())
}

/// Render single entry.
pub fn show_entry<'a>((entry, path): &(Issue, Rc<str>), app: &'a App<'a>) -> Result<()> {
    app.templates.init(app)?;

    let template_id = app.config.issue_view();
    app.templates.load_template(template_id)?;

    let j2 = app.templates.j2.borrow();
    let template = j2.get_template(template_id)?;
    let out = std::io::stdout();

    let context = RowContext {
        sid: entry.sid,
        urgency: entry.urgency,
        entry: Cow::Borrowed(entry),
        path: Cow::Borrowed(path),
        lineno: 0,
        count: 1,
    };
    template
        .render_to_write(context, &out)
        .with_context(|| format!("Unable to render entry template: {}", template_id))?;

    Ok(())
}

/// Export entries as JSON.
pub fn _show_json(entries: &[(Issue, Rc<str>)]) -> Result<()> {
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
pub fn show_diff(before: &Issue, after: &Issue, app: &App) {
    let id = &before.id[..8];
    info!("Issue {id} updated");

    // TODO: P2: use diff template to show the diff

    if before.status != after.status {
        info!(" status: {} -> {}", before.status, after.status);
    }
    if before.desc != after.desc {
        let before = before.desc.len();
        let after = after.desc.len();
        info!("  title: {before} -> {after} bytes");
    }

    if before.tags != after.tags {
        let before = before.tags.len();
        let after = after.tags.len();
        let s = if after > 1 { "s" } else { "" };
        info!("   tags: {before} -> {after} tag{s}");
    }

    if before.repeat != after.repeat {
        let before = before.repeat.as_deref().unwrap_or("no repeat");
        let after = after.repeat.as_deref().unwrap_or("no repeat");
        info!(" repeat: {before} -> {after}");
    }

    if before.due != after.due {
        let before = before.due.map(|d| dates::reldate(d, app.ts, Some(1)));
        let before = before.as_deref().unwrap_or("..");

        let after = after.due.map(|d| dates::reldate(d, app.ts, Some(1)));
        let after = after.as_deref().unwrap_or("..");

        info!("    due: {before} -> {after}");
    }
    if before.end != after.end {
        let before = before.end.map(|d| dates::reldate(d, app.ts, Some(1)));
        let before = before.as_deref().unwrap_or("..");

        let after = after.end.map(|d| dates::reldate(d, app.ts, Some(1)));
        let after = after.as_deref().unwrap_or("..");

        info!("    end: {before} -> {after}");
    }
}

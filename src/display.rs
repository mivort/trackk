use std::rc::Rc;

use serde_derive::Serialize;

use crate::config::{
    query::IndexType,
    reports::{ReportConfig, SectionConfig},
};
use crate::entry::Entry;
use crate::filter::{Filter, IdFilter, QueryFilter};
use crate::repo;
use crate::templates::dates;
use crate::templating::Templates;
use crate::{app::App, prelude::*, sort, storage};

#[derive(Serialize)]
pub struct RowContext<'a> {
    /// Flag if current row is odd or even.
    pub lineno: usize,

    /// Number of items in the section.
    pub count: usize,

    /// Limit number of shown entries.
    pub limit: usize,

    /// Entry reference.
    pub entry: &'a EntryContext<'a>,
}

#[derive(Serialize)]
pub struct EntryContext<'a> {
    /// Shorthand entry reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid: Option<usize>,

    /// Calculated entry urgency.
    pub urgency: f64,

    /// Path to storage file which contains the entry.
    pub path: &'a str,

    /// Reference to the issue data.
    #[serde(flatten)]
    pub entry: &'a Entry,
}

#[derive(Serialize)]
struct HeaderContext<'a> {
    /// Section title.
    title: &'a str,

    /// Number of items in the section.
    count: usize,
}

/// Render the list of filtered entries.
pub fn show_entries<'a>(ids: &IdFilter, report: &'a ReportConfig, app: &'a App<'a>) -> Result<()> {
    let mut templates = app.templates.borrow_mut();
    templates.init(app.ts, &app.config)?;

    let query = &mut QueryFilter::default();
    let mut shown = 0;

    for section in &report.sections {
        shown += show_section(ids, query, section, app, &mut templates)?;
    }

    let sync = repo::check_status(app)?;
    let sync_msg = if sync { "" } else { ". There are local changes." };

    if shown > 0 {
        info!(
            "{} entr{}{}",
            shown,
            if shown > 1 { "ies" } else { "y" },
            sync_msg
        );
    } else {
        info!("No results{}", sync_msg);
    }

    Ok(())
}

/// Apply template and render single output section.
/// Return the number of shown entries.
fn show_section(
    ids: &IdFilter,
    query: &mut QueryFilter,
    section: &SectionConfig,
    app: &App,
    templates: &mut Templates,
) -> Result<usize> {
    let SectionConfig {
        query: query_id,
        header,
        template,
        title,
        ..
    } = section;

    let query_data = app.config.query(query_id);
    let query_data = query_data.with_context(|| format!("Query '{query_id}' not defined"))?;
    let filter = query_data.filter;
    let sorting = query_data.sorting;
    let index = query_data.index;

    query
        .replace(filter, app)
        .with_context(|| format!("Unable to parse filter predicate: '{filter}'"))?;
    let mut entries = storage::fetch_entries(&Filter { ids, query }, index, app)?;

    if entries.is_empty() {
        return Ok(0);
    }

    if !header.is_empty() {
        templates
            .load_template(header)
            .with_context(|| format!("Unable to load header template: {header}"))?;
    }

    if !template.is_empty() {
        templates
            .load_template(template)
            .with_context(|| format!("Unable to load template: {template}"))?;
    }

    let sort = if app.sort.is_empty() { &sort::parse_rules(sorting)? } else { &app.sort };
    sort::sort_entries(&mut entries, sort)?;

    let j2 = &mut templates.j2;
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

    let count = entries.len();
    let limit = app.limit.min(entries.len()); // TODO: P2: support report-defined limit

    let template = j2.get_template(&section.template)?;
    for (lineno, (entry, path)) in entries[(count - limit)..].iter().enumerate() {
        let context = RowContext {
            entry: &EntryContext {
                sid: entry.sid,
                urgency: entry.urgency,
                entry,
                path,
            },
            lineno,
            count,
            limit,
        };
        template
            .render_to_write(context, &out)
            .with_context(|| format!("Unable to render report template: {}", section.template))?;
    }

    Ok(entries.len())
}

/// Render single entry.
pub fn show_entry<'a>((entry, path): &(Entry, Rc<str>), app: &'a App<'a>) -> Result<()> {
    let mut templates = app.templates.borrow_mut();
    templates.init(app.ts, &app.config)?;

    let template_id = app.config.templates.entry();
    templates.load_template(template_id)?;

    let j2 = &mut templates.j2;
    let template = j2.get_template(template_id)?;
    let out = std::io::stdout();

    let context = RowContext {
        entry: &EntryContext {
            sid: entry.sid,
            urgency: entry.urgency,
            entry,
            path,
        },
        lineno: 0,
        count: 1,
        limit: 1,
    };
    template
        .render_to_write(context, &out)
        .with_context(|| format!("Unable to render entry template: {}", template_id))?;

    Ok(())
}

/// Use one-shot format override.
pub fn show_format_override<'a>(fmt: &str, ids: &IdFilter, app: &'a App<'a>) -> Result<()> {
    let filter = Filter {
        ids,
        query: &app.filter,
    };
    let entries = storage::fetch_entries(&filter, IndexType::All, app)?;

    let mut templates = app.templates.borrow_mut();
    templates.init(app.ts, &app.config)?;

    for (lineno, (entry, path)) in entries.iter().enumerate() {
        let out = templates.j2.render_str(
            fmt,
            RowContext {
                entry: &EntryContext {
                    sid: entry.sid,
                    urgency: entry.urgency,
                    entry,
                    path,
                },
                count: entries.len(),
                limit: entries.len(),
                lineno,
            },
        )?;
        println!("{}", out);
    }

    Ok(())
}

/// Export entries as JSON.
pub fn _show_json(entries: &[(Entry, Rc<str>)]) -> Result<()> {
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
pub fn show_diff(before: &Entry, after: &Entry, app: &App) {
    let id = &before.id[..8];
    info!("Entry {id} updated");

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

    if before.when != after.when {
        let before = before.when.map(|d| dates::reldate(d, app.ts, None));
        let before = before.as_deref().unwrap_or("..");

        let after = after.when.map(|d| dates::reldate(d, app.ts, None));
        let after = after.as_deref().unwrap_or("..");

        info!("   when: {before} -> {after}");
    }
    if before.due != after.due {
        let before = before.due.map(|d| dates::reldate(d, app.ts, None));
        let before = before.as_deref().unwrap_or("..");

        let after = after.due.map(|d| dates::reldate(d, app.ts, None));
        let after = after.as_deref().unwrap_or("..");

        info!("    due: {before} -> {after}");
    }
    if before.end != after.end {
        let before = before.end.map(|d| dates::reldate(d, app.ts, None));
        let before = before.as_deref().unwrap_or("..");

        let after = after.end.map(|d| dates::reldate(d, app.ts, None));
        let after = after.as_deref().unwrap_or("..");

        info!("    end: {before} -> {after}");
    }
}

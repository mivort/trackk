use std::rc::Rc;

use serde_derive::Serialize;

use crate::entry::Entry;
use crate::filter::{Filter, IdFilter, QueryFilter};
use crate::repo;
use crate::templates::Templates;
use crate::templates::dates;
use crate::{app::App, prelude::*, sort, storage};
use crate::{
    config::{
        query::IndexType,
        reports::{ReportConfig, SectionConfig},
    },
    datecalc::token::Token,
};

use serde_json::Value;

#[derive(Serialize)]
pub struct RowContext<'a> {
    /// Number of the entry in query output. Can be used to alternate odd and
    /// even rows rendering.
    pub lineno: usize,

    /// Number of items in the section.
    pub count: usize,

    /// Limit number of shown entries.
    pub limit: usize,

    /// Entry reference.
    pub entry: &'a EntryContext<'a>,
}

#[derive(Serialize)]
pub struct GroupContext {
    /// Expression result as JSON value.
    pub group: Value,

    /// Number of the entry in query output.
    pub lineno: usize,

    /// Number of items in the section.
    pub count: usize,
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

    for preload in &report.preload {
        templates
            .load_template(preload, app)
            .with_context(|| format!("Unable to preload report base template: {preload}"))?;
    }

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
        group,
        header,
        template,
        title,
        ..
    } = section;

    let query_data = app.config.query(query_id)?;
    let filter = query_data.filter;
    let sorting = query_data.sorting;
    let index = query_data.index;
    let group_by = query_data.group_by;
    let mut group_stack = Vec::new();

    query
        .replace(filter, app)
        .with_context(|| format!("Unable to parse filter predicate: '{filter}'"))?;
    let mut entries = storage::fetch_entries(&Filter { ids, query }, index, app)?;

    let count = entries.len();
    let limit = app.limit.min(count); // TODO: P2: support report-defined limit

    let sort = if app.sort.is_empty() { &sort::parse_rules(sorting)? } else { &app.sort };
    sort::sort_entries(&mut entries, sort);

    if app.has_range() {
        app.apply_range(&mut entries);
    }

    if entries.is_empty() {
        return Ok(0);
    }

    if !header.is_empty() {
        templates
            .load_template(header, app)
            .with_context(|| format!("Unable to load header template: {header}"))?;
    }

    if !group.is_empty() && !group_by.is_empty() {
        templates
            .load_template(group, app)
            .with_context(|| format!("Unable to load group header template: {group}"))?;

        query.replace_group(group_by, app)?;
    } else {
        query.clear_group();
    };

    if !template.is_empty() {
        templates
            .load_template(template, app)
            .with_context(|| format!("Unable to load template: {template}"))?;
    }

    let j2 = &mut templates.j2;
    let out = std::io::stdout();

    if !header.is_empty() {
        let header = j2.get_template(&section.header)?;
        let context = HeaderContext { title, count };
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
    let group_template = j2.get_template(group).ok();
    let ts = app.local_time()?;
    let mut group_token = Token::Bool(false);

    for (lineno, (entry, path)) in entries.iter().enumerate() {
        'group_header: {
            let row_token = query.eval_group(entry, &mut group_stack, ts, app)?;
            if group_token == row_token {
                break 'group_header;
            }
            let template = unwrap_some_or!(&group_template, { break 'group_header });
            group_token = row_token;

            let context = GroupContext {
                lineno,
                count,
                group: group_token.as_value(),
            };

            template
                .render_to_write(context, &out)
                .with_context(|| format!("Unable to render group header: {}", section.group))?;
        }

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

    app.config.templates.preload(|id| {
        templates
            .load_template(id, app)
            .with_context(|| format!("Unable to preload template: {id}"))
    })?;

    let template_id = app.config.templates.entry();
    templates.load_template(template_id, app)?;

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

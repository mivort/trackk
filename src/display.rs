use std::borrow::Cow;
use std::rc::Rc;

use serde_derive::Serialize;

use crate::config::{ReportConfig, SectionConfig};
use crate::filter::IdFilter;
use crate::issue::Issue;
use crate::{App, prelude::*, sort, storage};

#[derive(Serialize)]
struct RowContext<'a> {
    /// Shorthand issue reference.
    sid: Option<usize>,

    /// Flag if current row is odd or even.
    lineno: usize,

    /// Reference to the issue data.
    #[serde(borrow)]
    issue: Cow<'a, Issue>,
}

/// Render the list of filtered entries.
pub fn show_entries<'a>(ids: &IdFilter, report: &'a ReportConfig, app: &App<'a>) -> Result<()> {
    app.templates.init(app);

    for section in &report.sections {
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
    sort::sort_entries(&mut entries, &sorting)?;

    app.templates
        .load_template(template)
        .with_context(|| format!("Unable to load template: {template}"))?;

    let j2 = app.templates.j2.borrow();
    let template = j2.get_template(&section.template)?;
    let out = std::io::stdout();

    for (lineno, (issue, _path)) in entries.iter().enumerate() {
        let context = RowContext {
            issue: Cow::Borrowed(issue),
            sid: issue.short,
            lineno,
        };
        template
            .render_to_write(context, &out)
            .with_context(|| format!("Unable to render report template: {}", section.template))?;
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

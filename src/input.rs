use std::io::{IsTerminal, Write, stdin, stdout};

use minijinja::Template;

use crate::app::App;
use crate::display::{EntryContext, RowContext};
use crate::entry::EntryPath;
use crate::prelude::*;

/// Read user input from stdin.
pub fn prompt(prompt: &str) -> Result<String> {
    let mut input = String::new();

    print!("{}", prompt);
    stdout().flush()?;

    stdin().read_line(&mut input)?;
    input = input.trim().to_string();

    Ok(input)
}

/// When several tasks match criteria, show the task picker.
pub fn pick_prompt<'a>(
    action: &str,
    mut entries: Vec<EntryPath>,
    app: &'a App<'a>,
) -> Result<Vec<EntryPath>> {
    if entries.len() < 2 || app.select_all {
        return Ok(entries);
    }

    // TODO: P2: support custom entry pickers

    if !stdout().is_terminal() {
        bail!(
            "Matching entries: {}. Not a terminal: abort.",
            entries.len()
        );
    }

    let template_id = app.config.templates.picker();

    let mut templates = app.templates.borrow_mut();

    templates.init(app.ts, &app.config)?;

    app.config.templates.preload(|id| {
        templates
            .load_template(id, app)
            .with_context(|| format!("Unable to preload template: {id}"))
    })?;

    templates
        .load_template(app.config.templates.picker(), app)
        .with_context(|| format!("Unable to load picker template: {template_id}"))?;

    let j2 = &templates.j2;
    let template = j2.get_template(template_id)?;

    'prompt: loop {
        let count = entries.len();
        let limit = count.min(9);
        let subset = &entries[(count - limit)..];

        let input = render_entries(action, &template, template_id, subset, count, limit)?;

        let mut selected = vec![];
        let mut filter = vec![];

        for tok in input.split(" ").filter(|s| !s.is_empty()) {
            match tok {
                "A" | "a" => break 'prompt,
                "Q" | "q" => return Ok(vec![]),
                _ => {}
            }

            if let Ok(numeric) = tok.parse::<usize>() {
                if numeric > limit {
                    bail!("Enter number from 1 to {}", limit);
                }
                selected.push(numeric);
                continue;
            }

            filter.push(tok);
        }

        if selected.is_empty() && filter.is_empty() {
            entries.drain(..entries.len() - 1);
            break;
        }

        retain_ids(&mut entries, count, limit, &selected, &filter);

        if entries.len() <= 1 || !selected.is_empty() {
            break;
        }
    }

    Ok(entries)
}

/// Render list of entries to pick from and prompt line.
fn render_entries(
    action: &str,
    template: &Template,
    template_id: &str,
    subset: &[EntryPath],
    count: usize,
    limit: usize,
) -> Result<String> {
    let out = std::io::stdout();
    for (lineno, (entry, path)) in subset.iter().enumerate() {
        let context = RowContext {
            entry: &EntryContext {
                sid: Some(limit - lineno),
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
            .with_context(|| format!("Unable to render picker template: {}", template_id))?;
    }

    prompt(&format!(
        "[{count}] {action}: a: all / 1..{limit}: select / text: filter / q: cancel [1]: "
    ))
}

/// Keep only entries with matching indices.
fn retain_ids(
    entries: &mut Vec<EntryPath>,
    count: usize,
    limit: usize,
    selected: &[usize],
    filter: &[&str],
) {
    let mut retain_idx: usize = 0;
    entries.retain(|(e, _)| {
        let rev_idx = count - retain_idx;
        let selected = selected.is_empty() || rev_idx <= limit && selected.contains(&rev_idx);
        let matches = filter.is_empty() || filter.iter().any(|f| e.title().contains(*f));

        retain_idx += 1;
        matches && selected
    });
}

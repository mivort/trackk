use std::borrow::Cow;
use std::io::{Write, stdin, stdout};
use std::rc::Rc;

use crate::app::App;
use crate::display::RowContext;
use crate::issue::Issue;
use crate::{prelude::*, sort};

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
pub fn pick_prompt(action: &str, mut entries: Vec<(Issue, Rc<str>)>, app: &App) -> Result<Vec<(Issue, Rc<str>)>> {
    // TODO: P2: check terminal state/config/args to suppress the prompt
    // TODO: P2: check the limit of entries to show in prompt

    if entries.len() < 2 {
        return Ok(entries);
    }

    let template_id = app.config.templates.picker();

    app.templates.init(app)?;
    app.templates
        .load_template(app.config.templates.picker())
        .with_context(|| format!("Unable to load picker template: {template_id}"))?;

    // TODO: P2: apply configurable sorting to picker results
    let sort = &sort::parse_rules("urgency+")?;
    sort::sort_entries(&mut entries, sort)?;

    let count = entries.len();
    let limit = count.min(9);

    let j2 = app.templates.j2.borrow();
    let template = j2.get_template(template_id)?;
    let out = std::io::stdout();

    let subset = &entries[(count - limit)..];

    for (lineno, (entry, path)) in subset.iter().enumerate() {
        let context = RowContext {
            sid: Some(limit - lineno),
            urgency: entry.urgency,
            entry: Cow::Borrowed(entry),
            path: Cow::Borrowed(path),
            lineno,
            count,
            limit,
        };
        template
            .render_to_write(context, &out)
            .with_context(|| format!("Unable to render picker template: {}", template_id))?;
    }

    let input = prompt(&format!(
        "{action}: a: all ({count}) / 1..{limit}: select / q: cancel: [1] "
    ))?;

    let mut selected = vec![];

    for tok in input.split(" ").filter(|s| !s.is_empty()) {
        match tok {
            "A" | "a" => return Ok(entries),
            "Q" | "q" => return Ok(vec![]),
            _ => {}
        }

        let pick = tok
            .parse::<usize>()
            .context("Non-numeric input in picker")?;
        if pick > limit {
            bail!("Enter number from 1 to {}", limit);
        }
        selected.push(pick);
    }

    let mut retain_idx: usize = 0;
    entries.retain(|_| {
        let rev_idx = count - retain_idx;
        if rev_idx <= limit && selected.contains(&rev_idx) {
            retain_idx += 1;
            true
        } else {
            retain_idx += 1;
            false
        }
    });

    Ok(entries)
}

use std::process::{Command, Stdio};

use crate::args::FilterArgs;
use crate::config::Config;
use crate::issue::Issue;
use crate::{prelude::*, storage};

/// Iterate over matching entries and run editor for each.
pub fn edit_entries(filter: &FilterArgs, config: &Config) -> Result<()> {
    let entries = storage::fetch_entries(filter, config)?;
    for (issue, _path) in &entries {
        let _format = format_markdown(&issue);
    }

    if false {
        Command::new(&config.editor)
            .stdin(Stdio::inherit())
            .spawn()?;
    }

    Ok(())
}

/// Output entry in editor-friendly format.
///
/// Format should look like this:
/// ``` markdown
/// Issue title and description
///
/// ----
///
/// * Field 1: value
/// * Field 2: value
/// ```
fn format_markdown(issue: &Issue) {
    let tags = issue.tags.iter().map(|t| &**t).collect::<Vec<_>>();

    println!(
        concat!(
            "{title}\n\n----\n\n",
            "* Status: {status}\n",
            "* Due: {due}\n",
            "* Tags: {tags}\n",
        ),
        title = issue.title,
        status = issue.status,
        due = issue.due.unwrap_or_default(),
        tags = tags.join(" "),
    );
}

/// Read edited entry back to the issue struct.
fn _parse_markdown() {}

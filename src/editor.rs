use std::process::Command;

use crate::args::FilterArgs;
use crate::config::Config;
use crate::prelude::*;

/// Iterate over matching entries and run editor for each.
pub fn edit_entries(_filter: &FilterArgs, config: &Config) -> Result<()> {
    Command::new(&config.editor).output()?;

    // TODO: output issue in editor-friendly format

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
fn _format_markdown() {}

/// Read edited entry back to the issue struct.
fn _parse_markdown() {}

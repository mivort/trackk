use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

use anyhow::Context;
use regex::RegexBuilder;

use crate::args::FilterArgs;
use crate::config::Config;
use crate::issue::Issue;
use crate::{prelude::*, storage};

/// Iterate over matching entries and run editor for each.
pub fn edit_entries(filter: &FilterArgs, config: &Config) -> Result<()> {
    let entries = storage::fetch_entries(filter, config)?;
    let mut changes = 0;
    for (mut issue, path) in entries {
        let mut tempfile = tempfile::NamedTempFile::with_suffix(".trackit.md")?;
        format_markdown(&issue, tempfile.as_file_mut())?;

        let status = Command::new(&config.editor)
            .arg(tempfile.path())
            .spawn()?
            .wait()?;

        if !status.success() {
            println!("Editing cancelled.");
            break;
        }

        let mut edited = File::open(tempfile.path())?;
        parse_markdown(&mut issue, &mut edited)?;
        issue.update_ts();

        let mut bucket = storage::fetch_bucket(&*path)?;
        let prev_issue = bucket.find_by_id_mut(&issue.id).unwrap();

        if prev_issue.status != issue.status {
            issue.update_status_ts();
        }

        *prev_issue = issue;

        storage::write_bucket(&bucket, &*path)?;
        changes += 1;
    }

    if changes > 0 {
        println!("Edited {changes} entry(es)");
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
fn format_markdown(issue: &Issue, file: &mut File) -> Result<()> {
    let tags = issue.tags.iter().map(|t| &**t).collect::<Vec<_>>();

    file.write_fmt(format_args!(
        concat!(
            "# {title}\n\n----\n\n",
            "* Status: {status}\n",
            "* Due:    {due}\n",
            "* Tags:   {tags}\n",
            "* Repeat: {repeat}\n",
            "\n",
            "----\n\n",
            "- Created:        {created}\n",
            "- Last modified:  {modified}\n",
            "- Status changed: {status_modified}\n",
        ),
        title = issue.title,
        status = issue.status,
        due = issue.due.map(|d| d.to_string()).unwrap_or_default(),
        tags = tags.join(" "),
        repeat = unwrap_some_or!(&issue.repeat, { "" }),
        created = issue.created,
        modified = issue.modified,
        status_modified = issue.status_modified.unwrap_or_default(),
    ))?;

    Ok(())
}

/// Read edited entry back to the issue struct.
fn parse_markdown(issue: &mut Issue, file: &mut File) -> Result<()> {
    let mut entry = String::new();
    file.read_to_string(&mut entry)?;

    let re = RegexBuilder::new("\\s*#?(.*?)^-{4,}$(.*)")
        .multi_line(true)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    let caps = re
        .captures(&entry)
        .context("Unable to find the metadata delimiter ('----')")?;

    let (_, [title, meta]) = caps.extract();
    issue.title = title.trim().to_owned();

    let meta_re = RegexBuilder::new("^*\\s+(\\w+):(.*)$")
        .multi_line(true)
        .build()
        .unwrap();

    for (_, [key, val]) in meta_re.captures_iter(meta).map(|c| c.extract()) {
        let key = key.to_lowercase();
        match key.as_str() {
            "status" => {
                issue.status = val.trim().to_owned();
            }
            "repeat" => {
                let val = val.trim();
                if val.is_empty() {
                    issue.repeat = None;
                } else {
                    issue.repeat = Some(val.to_owned());
                }
            }
            _ => {}
        }
    }

    Ok(())
}

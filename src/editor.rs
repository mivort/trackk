use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, ExitStatus};

use regex::RegexBuilder;

use crate::bucket::Bucket;
use crate::issue::Issue;
use crate::{App, prelude::*, storage};

/// Run editor, apply changes and return the exit status.
pub fn edit_entry(issue: &mut Issue, app: &App) -> Result<ExitStatus> {
    let mut tempfile = tempfile::NamedTempFile::with_suffix(".trackit.md")?;
    format_markdown(issue, tempfile.as_file_mut())?;

    let status = Command::new(&app.config.editor)
        .arg(tempfile.path())
        .spawn()?
        .wait()?;

    if !status.success() {
        println!("Editing cancelled.");
        return Ok(status);
    }

    let mut edited = File::open(tempfile.path())?;
    parse_markdown(issue, &mut edited)?;
    issue.update_ts();

    Ok(status)
}

/// Iterate over matching entries and run editor for each.
pub fn edit_entries(app: &App) -> Result<()> {
    let mut index = app.index_owned()?;
    let entries = storage::filter_all_entries(app)?;

    let mut changes = 0;
    for (mut issue, path) in entries {
        if !edit_entry(&mut issue, &app)?.success() {
            break;
        }

        let mut bucket = Bucket::from_path(&*path)?;
        let prev_issue = bucket.find_by_id_mut(&issue.id).unwrap();

        if !prev_issue.differs(&issue) {
            continue;
        }

        if prev_issue.status != issue.status {
            issue.apply_status(&prev_issue.status, issue.end.is_none(), &app.config);
            index.update_status(&app.config, &path, &issue);
        }

        *prev_issue = issue;

        storage::write_bucket(&bucket, &*path)?;
        changes += 1;
    }

    if changes > 0 {
        index.write()?;
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
            "* Tags:   {tags}\n",
            "* Due:    {due}\n",
            "* End:    {end}\n",
            "* Repeat: {repeat}\n",
            "\n",
            "----\n\n",
            "- Created:  {created}\n",
            "- Modified: {modified}\n",
        ),
        title = issue.title,
        status = issue.status,
        due = issue.due.map(|d| d.to_string()).unwrap_or_default(),
        end = issue.end.map(|d| d.to_string()).unwrap_or_default(),
        tags = tags.join(" "),
        repeat = unwrap_some_or!(&issue.repeat, { "" }),
        created = issue.created,
        modified = issue.modified,
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
            "tags" => {
                let tags = val.split_whitespace().filter(|s| !s.is_empty());
                issue.tags = tags.map(|s| s.to_string()).collect();
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

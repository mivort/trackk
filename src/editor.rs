use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, ExitStatus};

use regex::RegexBuilder;
use time::macros::format_description;
use time::{UtcDateTime, UtcOffset};

use crate::bucket::Bucket;
use crate::config::IndexType;
use crate::dateexp::parse_date;
use crate::filter::{Filter, IdFilter};
use crate::issue::Issue;
use crate::templates::dates;
use crate::{app::App, display, prelude::*, storage};

/// Run editor, apply changes and return the exit status.
pub fn edit_entry(issue: &mut Issue, app: &App) -> Result<ExitStatus> {
    let mut tempfile =
        tempfile::NamedTempFile::with_suffix(concat!(".", env!("CARGO_PKG_NAME"), ".md"))?;
    format_markdown(issue, tempfile.as_file_mut())?;

    let editor = &*app.config.editor();
    let status = Command::new(editor)
        .arg(tempfile.path())
        .spawn()
        .with_context(|| format!("Unable to start editor '{editor}'"))?
        .wait()?;

    if !status.success() {
        warn!(
            "Editor exited with code {}. Editing cancelled.",
            status.code().unwrap_or(-1)
        );
        return Ok(status);
    }

    let mut edited = File::open(tempfile.path())?;
    parse_markdown(issue, &mut edited, app)?;
    issue.update_ts();

    Ok(status)
}

/// Iterate over matching entries and run editor for each.
pub fn edit_entries(ids: &IdFilter, app: &App) -> Result<()> {
    let filters = Filter {
        ids,
        query: &mut Default::default(),
    };
    let entries = storage::fetch_entries(&filters, IndexType::All, app)?;
    let mut index = app.index_mut()?;

    let mut changes = 0;
    for (mut issue, path) in entries {
        if !edit_entry(&mut issue, app)?.success() {
            break;
        }

        issue.validate()?;

        let mut bucket = Bucket::from_path(&*path, app)?;
        let prev_issue = bucket.find_by_id_mut(&issue.id).unwrap();

        if !prev_issue.differs(&issue) {
            continue;
        }

        display::show_diff(prev_issue, &issue, app);

        if prev_issue.status != issue.status {
            issue.update_end(&app.config);
            index.update_status(&app.config, &path, &issue);
        }

        *prev_issue = issue;

        storage::write_bucket(&bucket, &*path, app)?;
        changes += 1;
    }

    if changes > 0 {
        index.write()?;
        info!("Edited {changes} entry(es)");
    } else {
        info!("No changes");
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
/// * __Field 1__: value
/// * __Field 2__: value
/// ```
fn format_markdown(issue: &Issue, file: &mut File) -> Result<()> {
    let tags = issue.tags.iter().map(|t| &**t).collect::<Vec<_>>();

    let offset = UtcOffset::current_local_offset()?;

    let due = match issue.due {
        Some(due) => format_date(due, offset)?,
        None => String::new(),
    };
    let end = match issue.end {
        Some(end) => format_date(end, offset)?,
        None => String::new(),
    };

    let now = time::UtcDateTime::now().unix_timestamp();

    let created = format!(
        "{} *({})*",
        format_date(issue.created, offset)?,
        dates::longreldate(issue.created, now, None)
    );
    let modified = format!(
        "{} *({})*",
        format_date(issue.modified, offset)?,
        dates::longreldate(issue.modified, now, None)
    );

    file.write_fmt(format_args!(
        concat!(
            "# {title}\n\n",
            "--------------------------------------------------------------------------------\n",
            "* __Status__  : {status}\n",
            "* __Tags__    : {tags}\n",
            "* __Due__     : {due}\n",
            "* __End__     : {end}\n",
            "* __Repeat__  : {repeat}\n",
            "--------------------------------------------------------------------------------\n",
            "- __ID__      : {id}\n",
            "- __Created__ : {created}\n",
            "- __Modified__: {modified}\n",
        ),
        title = issue.title,
        status = issue.status,
        due = due,
        end = end,
        tags = tags.join(" "),
        repeat = unwrap_some_or!(&issue.repeat, { "" }),
        created = created,
        modified = modified,
        id = issue.id,
    ))?;

    Ok(())
}

/// Apply ISO8601 format to UNIX timestamps.
fn format_date(date: i64, offset: UtcOffset) -> Result<String> {
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    let time = UtcDateTime::from_unix_timestamp(date)?.to_offset(offset);
    Ok(time.format(&format)?)
}

/// Read edited entry back to the issue struct.
fn parse_markdown(issue: &mut Issue, file: &mut File, app: &App) -> Result<()> {
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

    let meta_re = RegexBuilder::new(r"^\s*\*\s+__(\w+)__\s*:(.*)$")
        .multi_line(true)
        .build()
        .unwrap();

    for (_, [key, val]) in meta_re.captures_iter(meta).map(|c| c.extract()) {
        let key = key.to_lowercase();

        let mut due = issue.due;
        let mut end = issue.end;

        match key.as_str() {
            "status" => {
                issue.update_status(val.trim(), app)?;
            }
            "tags" => {
                let tags = val.split_whitespace().filter(|s| !s.is_empty());
                issue.tags = tags.map(|s| s.to_string()).collect();
            }
            "due" => {
                let val = val.trim();
                due = if val.is_empty() { None } else { Some(parse_date(val, app, issue)?) };
            }
            "end" => {
                let val = val.trim();
                end = if val.is_empty() { None } else { Some(parse_date(val, app, issue)?) };
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

        issue.due = due;
        issue.end = end;
    }

    Ok(())
}

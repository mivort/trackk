use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, ExitStatus};

use regex::RegexBuilder;
use time::macros::format_description;
use time::{UtcDateTime, UtcOffset};

use crate::datecalc::parse::parse_date;
use crate::entry::Entry;
use crate::input;
use crate::templates::dates;
use crate::{app::App, prelude::*};

/// Run editor in loop until it's not fully valid.
///
/// Return either true of false depending on if editing should continue or not.
pub fn edit_entry(entry: &mut Entry, app: &App) -> Result<bool> {
    loop {
        match run_edit_entry(entry, app) {
            Ok(status) => return Ok(status.success()),
            Err(err) => {
                error!("Editing error: {err:?}");
                let reply = input::prompt("Keep editing? [Y]/n ")?;
                if reply == "n" {
                    return Ok(false);
                }
            }
        }
    }
}

/// Run editor once, apply changes and return the exit status.
fn run_edit_entry(entry: &mut Entry, app: &App) -> Result<ExitStatus> {
    let mut tempfile =
        tempfile::NamedTempFile::with_suffix(concat!(".", env!("CARGO_PKG_NAME"), ".md"))?;
    format_markdown(entry, tempfile.as_file_mut())?;

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
    parse_markdown(entry, &mut edited, app)?;
    entry.update_ts();
    entry.validate(app)?;

    Ok(status)
}

/// Output entry in editor-friendly format.
///
/// Format should look like this:
/// ``` markdown
/// Entry title and description
///
/// ----
///
/// * __Field 1__: value
/// * __Field 2__: value
/// ```
fn format_markdown(issue: &Entry, file: &mut File) -> Result<()> {
    let tags = issue.tags.iter().map(|t| &**t).collect::<Vec<_>>();

    let offset = UtcOffset::current_local_offset()?;

    let when = match issue.when {
        Some(when) => format_date(when, offset)?,
        None => String::new(),
    };

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
            "* __status__  : {status}\n",
            "* __tags__    : {tags}\n",
            "* __when__    : {when}\n",
            "* __due__     : {due}\n",
            "* __end__     : {end}\n",
            "* __repeat__  : {repeat}\n",
            "--------------------------------------------------------------------------------\n",
            "- __id__      : {id}\n",
            "- __created__ : {created}\n",
            "- __modified__: {modified}\n",
        ),
        title = issue.desc,
        status = issue.status,
        when = when,
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
fn parse_markdown(issue: &mut Entry, file: &mut File, app: &App) -> Result<()> {
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
    issue.desc = title.trim().to_owned();

    let meta_re = RegexBuilder::new(r"^\s*\*\s+__(\w+)__\s*:(.*)$")
        .multi_line(true)
        .build()
        .unwrap();

    for (_, [key, val]) in meta_re.captures_iter(meta).map(|c| c.extract()) {
        let key = key.to_lowercase();

        let mut when = issue.when;
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
            "when" => {
                let val = val.trim();
                when = if val.is_empty() { None } else { parse_date(val, app, issue)? };
            }
            "due" => {
                let val = val.trim();
                due = if val.is_empty() { None } else { parse_date(val, app, issue)? };
            }
            "end" => {
                let val = val.trim();
                end = if val.is_empty() { None } else { parse_date(val, app, issue)? };
            }
            "repeat" => {
                let val = val.trim();
                if val.is_empty() {
                    issue.repeat = None;
                } else {
                    issue.repeat = Some(val.to_owned());
                }
            }
            _ => {
                // TODO: P2: set custom field value
            }
        }

        issue.when = when;
        issue.due = due;
        issue.end = end;
    }

    Ok(())
}

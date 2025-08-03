use std::fmt::Write as _;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
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
    format_markdown(entry, tempfile.as_file_mut(), app)?;

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

    parse_markdown_file(entry, tempfile.path(), app)?;
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
fn format_markdown(entry: &Entry, file: &mut File, app: &App) -> Result<()> {
    let tags = entry.tags.iter().map(|t| &**t).collect::<Vec<_>>();

    let offset = UtcOffset::current_local_offset()?;

    let when = match entry.when {
        Some(when) => format_date(when, offset)?,
        None => String::new(),
    };

    let due = match entry.due {
        Some(due) => format_date(due, offset)?,
        None => String::new(),
    };
    let end = match entry.end {
        Some(end) => format_date(end, offset)?,
        None => String::new(),
    };

    let now = time::UtcDateTime::now().unix_timestamp();

    let created = format!(
        "{} *({})*",
        format_date(entry.created, offset)?,
        dates::longreldate(entry.created, now, None)
    );
    let modified = format!(
        "{} *({})*",
        format_date(entry.modified, offset)?,
        dates::longreldate(entry.modified, now, None)
    );

    use terminal_size::{Height, Width, terminal_size};
    let (Width(cols), _) = terminal_size().unwrap_or((Width(0), Height(0)));
    let sepw = (cols - 8).clamp(4, 80) as usize;

    file.write_fmt(format_args!(
        concat!(
            "# {title}\n\n",
            "{empty:-^sepw$}\n",
            "* __status__      : {status}\n",
            "* __tags__        : {tags}\n",
            "* __when__        : {when}\n",
            "* __due__         : {due}\n",
            "* __end__         : {end}\n",
            "* __repeat__      : {repeat}\n",
            "{empty:-^sepw$}\n",
            "{custom}",
            "{empty:-^sepw$}\n",
            "- __id__          : {id}\n",
            "- __created__     : {created}\n",
            "- __modified__    : {modified}\n",
        ),
        empty = "",
        sepw = sepw,
        title = entry.desc,
        status = entry.status,
        when = when,
        due = due,
        end = end,
        tags = tags.join(" "),
        repeat = unwrap_some_or!(&entry.repeat, { "" }),
        created = created,
        modified = modified,
        id = entry.id,
        custom = format_custom_fields(entry, app),
    ))?;

    Ok(())
}

/// Produce formatted list of custom fields. In case if value mismatches
/// declared field type, prefix it with '-' to avoid accidential overwrite.
fn format_custom_fields(entry: &Entry, app: &App) -> String {
    let mut out = String::new();
    let fields = app.config.fields_map();

    for (field, field_type) in fields.iter() {
        let value = entry.meta(field);
        let (valid, value) = value.map_or((true, String::new()), |v| {
            field_type.format_value(v).map_or_else(
                || (false, format!("incompatible value: '{}'", v)),
                |v| (true, v),
            )
        });

        let prefix = if valid { "*" } else { "-" };
        let fill = 12 - field.len();
        let _ = writeln!(out, "{prefix} __{field}__{:fill$}: {value}", "",);
    }

    out
}

/// Apply ISO8601 format to UNIX timestamps.
fn format_date(date: i64, offset: UtcOffset) -> Result<String> {
    let date = crate::templates::dates::safe_clamp(date);
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    let time = UtcDateTime::from_unix_timestamp(date)?.to_offset(offset);
    Ok(time.format(&format)?)
}

/// Read edited entry from file back to the entry struct.
fn parse_markdown_file(entry: &mut Entry, file: &Path, app: &App) -> Result<()> {
    let mut file = File::open(file)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    parse_markdown(entry, &data, app)
}

/// Read edited entry from string back to the entry struct.
fn parse_markdown(entry: &mut Entry, data: &str, app: &App) -> Result<()> {
    let re = RegexBuilder::new("\\s*#?(.*?)^-{4,}$(.*)")
        .multi_line(true)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    let caps = re
        .captures(data)
        .context("Unable to find the metadata delimiter ('----')")?;

    let (_, [title, meta]) = caps.extract();
    entry.desc = title.trim().to_owned();

    let meta_re = RegexBuilder::new(r"^\s*\*\s+__(\w+)__\s*:(.*)$")
        .multi_line(true)
        .build()
        .unwrap();

    let mut when = entry.when;
    let mut due = entry.due;
    let mut end = entry.end;

    for (_, [key, val]) in meta_re.captures_iter(meta).map(|c| c.extract()) {
        let key = key.to_lowercase();
        let val = val.trim();

        match key.as_str() {
            "status" => {
                entry.update_status(val, app)?;
                end = entry.end; // TODO: P1: consider field set order
            }
            "tags" => {
                let tags = val.split_whitespace().filter(|s| !s.is_empty());
                entry.tags = tags.map(|s| s.to_string()).collect();
            }
            "when" => {
                when = if val.is_empty() {
                    None
                } else {
                    parse_date(val, app, entry).context("Unable to parse 'when' field")?
                };
            }
            "due" => {
                due = if val.is_empty() {
                    None
                } else {
                    parse_date(val, app, entry).context("Unable to parse 'due' field")?
                };
            }
            "end" => {
                end = if val.is_empty() {
                    None
                } else {
                    parse_date(val, app, entry).context("Unable to parse 'end' field")?
                };
            }
            "repeat" => {
                if val.is_empty() {
                    entry.repeat = None;
                } else {
                    entry.repeat = Some(val.to_owned());
                }
            }
            key => {
                if val.is_empty() {
                    entry.meta.remove(key);
                    continue;
                }
                let field_type = app
                    .config
                    .field_type(key)
                    .with_context(|| format!("Unknown field: {}", key))?;
                let value = field_type.parse_value(val)?;
                entry.meta.insert(key.into(), value);
            }
        }
    }

    entry.when = when;
    entry.due = due;
    entry.end = end;

    Ok(())
}

#[cfg(test)]
const MD_TEXT: &str = r#"
# test title

----
* __tags__   : a b c
* __when__   : 10min
* __due__    : 
* __priority__ : 3
* __project__ : abc
"#;

#[test]
fn parse_text() {
    let app = App::default();
    let mut entry = Entry::default();

    parse_markdown(&mut entry, MD_TEXT, &app).unwrap();
    assert_eq!(entry.title(), "test title");
    assert_eq!(entry.tags.len(), 3);
    assert_eq!(entry.meta["priority"].as_f64(), Some(3.));
    assert_eq!(entry.meta["project"].as_str(), Some("abc"));
}

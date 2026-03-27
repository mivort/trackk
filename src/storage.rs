use crate::args::EntryArgs;
use crate::bucket::Bucket;
use crate::config::{Config, query::IndexType};
use crate::entry::Entry;
use crate::filter::{Filter, IdFilter};
use crate::{app::App, display, prelude::*};
use crate::{editor, input, sort};

use std::fs::{self, File};
use std::io::{BufWriter, ErrorKind, Write};
use std::path::Path;
use std::rc::Rc;
use time::{Date, UtcDateTime};
use walkdir::WalkDir;

/// Access storage bucket if it exists and add new entry to it.
pub fn add_entry(new_entry: Entry, app: &App) -> Result<()> {
    let date = UtcDateTime::from_unix_timestamp(app.ts)?.date();

    let (mut bucket, path) = fetch_new_bucket(&date, &app.config)?;

    if bucket.entries.iter().any(|e| e.id == new_entry.id) {
        bail!("collision has occured: task uuid exists");
    }

    let mut index = app.index_mut()?;
    let row = index.update_status(&app.config, &path, &new_entry);
    index.write()?;

    let id = &new_entry.id[..(new_entry.id.len().min(8))];
    if let Some(row) = row {
        info!("New active entry ({}) added, ID: {id}", row + 1);
    } else {
        info!("New inactive entry added, ID: {id}");
    }

    if let Some(idx) = bucket.insert(new_entry) {
        bail!("UUID collisiion detected: {}", bucket.entries[idx].id);
    }
    write_bucket(&bucket, &path, app)
}

/// Find entry using the filter and update its properties.
pub fn modify_entries<'a>(ids: &IdFilter, args: &EntryArgs, app: &'a App<'a>) -> Result<()> {
    let mut changes = 0;

    let filters = Filter {
        ids,
        query: &mut Default::default(),
    };
    let mut entries = fetch_entries(&filters, app.filter.index(), app)?;

    if entries.is_empty() {
        bail!("No entries match the criteria");
    }

    let entries = 'entries: {
        let show_picker = ids.check_ambiguity(&entries);
        if show_picker || app.has_range() {
            sort::sort_entries(&mut entries, app.sort_or_default());
            app.apply_range(&mut entries);
        }

        if show_picker {
            break 'entries input::pick_prompt("Modify", entries, app)?;
        }
        entries
    };

    let mut index = app.index_mut()?;
    let mut repeats = Vec::new();

    // TODO: P1: use cache to reduce amount of re-parsing/writes?

    for (entry, path) in &entries {
        let mut bucket = Bucket::from_path(&**path, app)?;
        let bucket_entry = bucket.find_by_id_mut(&entry.id).unwrap();
        bucket_entry.apply_args(args, app)?;

        if !args.edit {
            bucket_entry.validate(app)?;
        } else if !editor::edit_entry(bucket_entry, app)? {
            break;
        }

        if !entry.differs(bucket_entry) {
            continue;
        }

        display::show_diff(entry, bucket_entry, app);

        if entry.status != bucket_entry.status {
            bucket_entry.update_end(&app.config);
            index.update_status(&app.config, path, bucket_entry);

            if let Some(repeat) = bucket_entry.check_repeat(app)? {
                repeats.push(repeat);
            }
        };

        write_bucket(&bucket, &**path, app)?;
        changes += 1;
    }

    if changes > 0 {
        index.write()?;
        info!("Updated {changes} entry(es)");
    } else {
        info!("No changes");
    }

    drop(index);
    for repeat in repeats {
        add_entry(repeat, app)?;
    }

    Ok(())
}

/// Produce the list of entries to display or modify.
///
/// If any specific ID was provided, index type (usually provided by query)
/// is overriden based on how shorthand resolution went.
pub fn fetch_entries(
    filters: &Filter,
    index: IndexType,
    app: &App,
) -> Result<Vec<(Entry, Rc<str>)>> {
    let ids = filters.ids;

    if ids.empty_set() {
        return Ok(Vec::new());
    }

    if ids.enabled {
        return if ids.only_active {
            filter_active_entries(filters, app)
        } else {
            filter_all_entries(filters, app)
        };
    }

    match index {
        IndexType::All => filter_all_entries(filters, app),
        IndexType::Active => filter_active_entries(filters, app),
        IndexType::Recent => todo!(), // TODO: P1: introduce recent entries index
    }
}

/// Create or get the storage bucket using the current date.
pub fn fetch_new_bucket(date: &Date, config: &Config) -> Result<(Bucket, String)> {
    let rel_path = rel_path_by_date(date);
    let mut full_path = config.entries_path()?;
    full_path.push(&rel_path);

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).context("Unable to create storage directory")?;
    }

    let bucket = Bucket::from_full_path_or_default(&full_path)?;

    Ok((bucket, rel_path))
}

/// Serialize bucket data and store in provided relative path.
pub fn write_bucket(data: &Bucket, rel_path: impl AsRef<Path>, app: &App) -> Result<()> {
    let mut full_path = app.config.entries_path()?;
    full_path.push(&rel_path);

    let file = File::create(full_path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, data)?;
    writer.write_all(b"\n")?;

    Ok(())
}

/// Iterate over buckets and produce the list of entries which qualify.
fn filter_all_entries(filters: &Filter, app: &App) -> Result<Vec<(Entry, Rc<str>)>> {
    trace!("Traversing all buckets");

    let mut output = Vec::new();
    let path = app.config.entries_path()?;

    let index = app.index()?;
    let local = app.local_time()?;
    let urgency = app.urgency()?;
    let mut op_stack = Vec::new();

    if !path.exists() {
        bail!(
            "Path {} doesn't exist. Run '{} init' to initiate task repository. Add '--clone [url]' to fetch existing repository.",
            path.to_string_lossy(),
            env!("CARGO_BIN_NAME")
        );
    }

    let mut traverse_count = 0;

    for entry in WalkDir::new(&path).min_depth(2) {
        let entry = entry?;

        if entry.file_type().is_dir() {
            continue;
        }

        let bucket = Bucket::from_full_path(entry.path())?;
        let relpath = entry.path().strip_prefix(&path)?;
        let path = Rc::<str>::from(relpath.to_string_lossy());

        traverse_count += 1;

        for mut issue in bucket.entries {
            if !filters.ids.matches(&issue.id) {
                continue;
            }

            op_stack.clear();
            if !filters.query.match_issue(&issue, app, &mut op_stack)? {
                continue;
            }

            op_stack.clear();
            if !app.filter.match_issue(&issue, app, &mut op_stack)? {
                continue;
            }

            op_stack.clear();
            issue.calculate_urgency(&mut op_stack, local, urgency, app)?;

            if app.config.values.active_status.contains(&issue.status) {
                issue.sid = index.find_id(&issue.id);
            }
            output.push((issue, path.clone()));
        }
    }

    trace!("Traversed {} buckets", traverse_count);
    Ok(output)
}

/// Iterate over entries from the active index.
fn filter_active_entries(filters: &Filter, app: &App) -> Result<Vec<(Entry, Rc<str>)>> {
    let mut result = Vec::new();

    let cache = &mut *app.cache.borrow_mut();
    let index = app.index()?;
    let local = app.local_time()?;
    let urgency = app.urgency()?;
    let mut op_stack = Vec::new();

    for (idx, e) in index.active().iter().enumerate() {
        let (bucket_path, id) = unwrap_some_or!(e.rsplit_once("/"), {
            warn!("Active index entry has broken reference: {e}");
            continue;
        });

        let bucket = Bucket::from_cache(bucket_path, cache, app).with_context(|| {
            format!(
                concat!(
                    "Unable to open index reference: {}. ",
                    "Run '",
                    env!("CARGO_BIN_NAME"),
                    " refresh --force' to rebuild the index."
                ),
                bucket_path
            )
        })?;

        let issue = bucket.find_by_id(id);
        if let Some(issue) = issue {
            if !filters.ids.matches(&issue.id) {
                continue;
            }

            op_stack.clear();
            if !filters.query.match_issue(issue, app, &mut op_stack)? {
                continue;
            }

            op_stack.clear();
            if !app.filter.match_issue(issue, app, &mut op_stack)? {
                continue;
            }

            op_stack.clear();
            let mut issue_owned = issue.with_shorthand(idx + 1);
            issue_owned.calculate_urgency(&mut op_stack, local, urgency, app)?;

            result.push((issue_owned, Rc::from(bucket_path)));
        } else {
            warn!(
                "Index ID is missing: {id}. Run '{} refresh --force' to rebuild the index.",
                env!("CARGO_BIN_NAME")
            );
        }
    }

    Ok(result)
}

/// Iterate over files in storage directory and update the index. If 'force' is
/// not set and mtime is lower or equal to index, skip the entry.
pub fn refresh_index(app: &App, force: bool) -> Result<()> {
    let path = app.config.entries_path()?;
    let mut index = if force { app.index_empty_mut() } else { app.index_mut() }?;

    for entry in WalkDir::new(&path).min_depth(2) {
        let entry = unwrap_ok_or!(entry, err, {
            match err.io_error() {
                Some(ioerr) if ioerr.kind() == ErrorKind::NotFound => {
                    return Err(anyhow!(err).context(concat!(
                        "Entry directory not found. You may need to run '",
                        env!("CARGO_BIN_NAME"),
                        " init'."
                    )));
                }
                _ => return Err(anyhow!(err).context("Unable to refresh the index")),
            }
        });

        if entry.file_type().is_dir() {
            continue;
        }

        let mtime = entry.metadata()?.modified()?;
        if mtime <= index.mtime() {
            continue;
        }

        let bucket = Bucket::from_full_path(entry.path())?;
        let relpath = entry.path().strip_prefix(&path)?;

        for issue in &bucket.entries {
            index.update_status(&app.config, &relpath.to_string_lossy(), issue);
        }
    }

    if force {
        index.sort();
    }

    index.write()?;
    trace!("Active entry index rewritten");

    Ok(())
}

/// Produce bucket path from the provided date.
pub fn rel_path_by_date(date: &Date) -> String {
    let year = date.year();
    let month = date.month() as i32;

    format!("{year}/{month:02}.json")
}

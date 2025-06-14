use crate::args::EntryArgs;
use crate::bucket::Bucket;
use crate::config::Config;
use crate::filter::IdFilter;
use crate::issue::Issue;
use crate::{App, prelude::*};

use std::fs;
use std::path::Path;
use std::rc::Rc;
use time::{Date, UtcDateTime};
use walkdir::WalkDir;

/// Access storage bucket if it exists and add new entry to it.
pub fn add_entry(new_entry: Issue, app: &App) -> Result<()> {
    let date = UtcDateTime::now().date();

    let (mut bucket, path) = fetch_new_bucket(&date, &app.config)?;

    if bucket.entries.iter().any(|e| e.id == new_entry.id) {
        bail!("collision has occured: task uuid exists");
    }

    let mut index = app.index_mut()?;
    index.update_status(&app.config, &path, &new_entry);
    index.write()?;

    bucket.insert(new_entry);
    write_bucket(&bucket, &path, app)
}

/// Find entry using the filter and update its properties.
pub fn modify_entries(ids: &IdFilter, args: &EntryArgs, app: &App) -> Result<()> {
    let mut changes = 0;

    let entries = filter_all_entries(ids, app)?;
    let mut index = app.index_mut()?;

    // TODO: ask if multiple entries are expected
    // TODO: use cache to reduce amount of re-parsing/writes?

    for (issue, path) in &entries {
        let mut bucket = Bucket::from_path(&**path, app)?;
        let bucket_issue = bucket.find_by_id_mut(&issue.id).unwrap();
        bucket_issue.apply_args(args, app)?;

        if issue.status != bucket_issue.status {
            bucket_issue.update_end(&app.config);
            index.update_status(&app.config, path, bucket_issue);
        }

        write_bucket(&bucket, &**path, app)?;
        changes += 1;
    }

    if changes > 0 {
        index.write()?;
    }

    info!("Updated {changes} entry(es)");

    Ok(())
}

/// Produce the list of entries to display or modify.
pub fn fetch_entries(ids: &IdFilter, app: &App, all: bool) -> Result<Vec<(Issue, Rc<str>)>> {
    if all {
        filter_all_entries(ids, app)
    } else {
        filter_active_entries(ids, app)
    }
}

/// Create or get the storage bucket using the current date.
fn fetch_new_bucket(date: &Date, config: &Config) -> Result<(Bucket, String)> {
    let year = date.year();
    let month = date.month() as i32;
    let data = &config.data_path;
    let issues = &config.issues_path;
    let directory = format!("{data}/{issues}/{year}");

    fs::create_dir_all(&directory).context("Unable to create storage directory")?;

    let full_path = format!("{directory}/{month:02}.json");
    let path = format!("{year}/{month:02}.json");
    let bucket = Bucket::from_full_path_or_default(&full_path)?;

    Ok((bucket, path))
}

/// Serialize bucket data and store in provided path.
pub fn write_bucket(data: &Bucket, path: impl AsRef<Path>, app: &App) -> Result<()> {
    let output = serde_json::to_string_pretty(data)?;
    let path = Path::new(&*app.config.data_path)
        .join(&*app.config.issues_path)
        .join(&path);
    fs::write(path, output)?;

    Ok(())
}

/// Iterate over buckets and produce the list of entries which qualify.
pub fn filter_all_entries(ids: &IdFilter, app: &App) -> Result<Vec<(Issue, Rc<str>)>> {
    let mut output = Vec::new();
    if ids.empty_set {
        return Ok(output);
    }

    let path = Path::new(&*app.config.data_path).join(&*app.config.issues_path);

    let index = app.index()?;
    let mut op_stack = Vec::new();

    for entry in WalkDir::new(&path) {
        let entry = entry?;

        if entry.file_type().is_dir() {
            continue;
        }

        let bucket = Bucket::from_full_path(entry.path())?;
        let relpath = entry.path().strip_prefix(&path)?;
        let path = Rc::<str>::from(relpath.to_string_lossy());

        trace!("Reading bucket: {}", path);

        for mut issue in bucket.entries {
            if !ids.matches(&issue.id) {
                continue;
            }

            op_stack.clear();
            if !app.filter.match_issue(&issue, app, &mut op_stack)? {
                continue;
            }

            if app.config.values.active_status.contains(&issue.status) {
                issue.short = index.find_id(&issue.id);
            }
            output.push((issue, path.clone()));
        }
    }

    Ok(output)
}

/// Iterate over entries from the active index.
pub fn filter_active_entries(ids: &IdFilter, app: &App) -> Result<Vec<(Issue, Rc<str>)>> {
    let mut result = Vec::new();
    if ids.empty_set {
        return Ok(result);
    }

    let cache = &mut *app.cache.borrow_mut();
    let index = app.index()?;
    let mut op_stack = Vec::new();

    for (idx, e) in index.active().iter().enumerate() {
        let (bucket_path, id) = unwrap_some_or!(e.rsplit_once("/"), {
            warn!("Active index entry has broken reference: {e}");
            continue;
        });

        let bucket = Bucket::from_cache(bucket_path, cache, app)
            .with_context(|| format!("Unable to open bucket referenced in index: {bucket_path}"))?;
        let issue = bucket.find_by_id(id);
        if let Some(issue) = issue {
            if !ids.matches(&issue.id) {
                continue;
            }

            op_stack.clear();
            if app.filter.match_issue(issue, app, &mut op_stack)? {
                result.push((issue.with_shorthand(idx + 1), Rc::from(bucket_path)));
            }
        }
    }

    Ok(result)
}

/// Iterate over files in storage directory and update the index. If 'force' is
/// not set and mtime is lower or equal to index, skip the entry.
pub fn refresh_index(app: &App, force: bool) -> Result<()> {
    let path = Path::new(&*app.config.data_path).join(&*app.config.issues_path);
    let mut index = app.index_mut()?;
    let mut changes = false;

    for entry in WalkDir::new(&path) {
        let entry = entry?;

        if entry.file_type().is_dir() {
            continue;
        }

        if !force {
            let mtime = entry.metadata()?.modified()?;
            if mtime <= index.mtime() {
                continue;
            }
        }

        let bucket = Bucket::from_full_path(entry.path())?;
        let relpath = entry.path().strip_prefix(&path)?;

        for issue in &bucket.entries {
            index.update_status(&app.config, &relpath.to_string_lossy(), issue);
        }
        changes = true;
    }

    if changes {
        index.write()?;
    }

    Ok(())
}

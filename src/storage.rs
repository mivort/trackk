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

    let mut index = app.index_mut()?;
    let entries = filter_all_entries(ids, app)?;

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

    println!("Updated {changes} entry(es)");

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
    let data = &config.data_dir;
    let issues = &config.issues_dir;
    let directory = format!("{data}/{issues}/{year}");

    fs::create_dir_all(&directory)?;

    let path = format!("{directory}/{month:02}.json");
    let bucket = Bucket::from_path_or_default(&path)?;

    Ok((bucket, path))
}

/// Serialize bucket data and store in provided path.
pub fn write_bucket(data: &Bucket, path: impl AsRef<Path>, app: &App) -> Result<()> {
    let output = serde_json::to_string_pretty(data)?;
    let path = Path::new(&app.config.data_dir)
        .join(&app.config.issues_dir)
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

    let path = Path::new(&app.config.data_dir).join(&app.config.issues_dir);

    let index = app.index()?;
    let mut op_stack = Vec::new();

    for entry in WalkDir::new(&path) {
        let entry = entry?;

        if entry.file_type().is_dir() {
            continue;
        }

        let _m = entry.metadata()?.modified()?;

        let relpath = entry.path().strip_prefix(&path)?;
        let bucket = Bucket::from_path(relpath, app)?;
        let path = Rc::<str>::from(relpath.to_string_lossy());

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
            bail!("Active index entry has broken reference: {e}");
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

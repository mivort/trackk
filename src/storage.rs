use crate::args::{FilterArgs, ModArgs};
use crate::bucket::Bucket;
use crate::config::Config;
use crate::index::Index;
use crate::issue::Issue;
use crate::prelude::*;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use time::{Date, UtcDateTime};
use walkdir::WalkDir;

/// Access storage bucket if it exists and add new entry to it.
pub fn add_entry(new_entry: Issue, config: &Config) -> Result<()> {
    let date = UtcDateTime::now().date();

    let (mut bucket, path) = fetch_new_bucket(&date, config)?;

    if bucket.entries.iter().any(|e| e.id == new_entry.id) {
        bail!("collision has occured: task uuid exists");
    }

    let mut index = Index::load(config)?;
    index.update_status(&path, &new_entry);
    index.write()?;

    bucket.insert(new_entry);
    write_bucket(&bucket, &path)
}

/// Find entry using the filter and update its properties.
pub fn modify_entries(args: &ModArgs, config: &Config, index: &mut Index) -> Result<()> {
    let mut changes = 0;

    let entries = fetch_entries(&args.filter, config, index)?;

    // TODO: ask if multiple entries are expected
    // TODO: use cache to reduce amount of re-parsing/writes?

    for (issue, path) in &entries {
        let mut bucket = Bucket::from_path(&**path)?;
        let bucket_issue = bucket.find_by_id_mut(&issue.id).unwrap();
        bucket_issue.apply_args(&args.entry, config);

        if bucket_issue.status != issue.status {
            bucket_issue.update_end_ts();
            index.update_status(path, issue);
        }

        write_bucket(&bucket, &**path)?;
        changes += 1;
    }

    if changes > 0 {
        index.write()?;
    }

    println!("Updated {changes} entry(es)");

    Ok(())
}

/// Produce the list of entries to display or modify.
pub fn fetch_entries(
    filter: &FilterArgs,
    config: &Config,
    index: &Index,
) -> Result<Vec<(Issue, Rc<str>)>> {
    if filter.all {
        return filter_all_entries(filter, config);
    }

    filter_active_entries(filter, index)
}

/// Create or get the storage bucket using the current date.
fn fetch_new_bucket(date: &Date, config: &Config) -> Result<(Bucket, String)> {
    let year = date.year();
    let month = date.month() as i32;
    let data = &config.data;
    let directory = format!("{data}/{year}");

    fs::create_dir_all(&directory)?;

    let path = format!("{directory}/{month:02}.json");
    let bucket = Bucket::from_path_or_default(&path)?;

    Ok((bucket, path))
}

/// Serialize bucket data and store in provided path.
pub fn write_bucket(data: &Bucket, path: impl AsRef<Path>) -> Result<()> {
    let output = serde_json::to_string_pretty(data)?;
    fs::write(path, output)?;

    Ok(())
}

/// Iterate over buckets and produce the list of entries which qualify.
fn filter_all_entries(filter: &FilterArgs, config: &Config) -> Result<Vec<(Issue, Rc<str>)>> {
    let mut output = Vec::new();

    let walkdir = WalkDir::new(&config.data).into_iter().filter_entry(|e| {
        !e.file_name()
            .to_str()
            .map(|n| n.starts_with("."))
            .unwrap_or(false)
    });

    for entry in walkdir {
        let entry = entry?;

        if entry.depth() < 2 || entry.file_type().is_dir() {
            continue;
        }

        let bucket = Bucket::from_path(entry.path())?;
        let path = Rc::<str>::from(entry.path().to_string_lossy());

        for issue in bucket.entries {
            if issue.match_filter(filter) {
                output.push((issue, path.clone()));
            }
        }
    }

    Ok(output)
}

/// Iterate over entries from the active index.
fn filter_active_entries(filter: &FilterArgs, index: &Index) -> Result<Vec<(Issue, Rc<str>)>> {
    let mut cache = HashMap::<String, Rc<Bucket>>::new();
    let mut result = Vec::new();

    for (idx, e) in index.active().iter().enumerate() {
        let (bucket_path, id) = unwrap_some_or!(e.rsplit_once("/"), {
            bail!("Active index entry has missing path");
        });

        let bucket = unwrap_some_or!(cache.get(bucket_path), {
            &(|| -> Result<_> {
                let bucket = Rc::new(Bucket::from_path(bucket_path)?);
                cache.insert(bucket_path.to_owned(), bucket.clone());
                Ok(bucket)
            })()?
        });

        let issue = bucket.find_by_id(id);
        if let Some(issue) = issue {
            if issue.match_filter(filter) {
                result.push((issue.with_shorthand(idx + 1), Rc::from(bucket_path)));
            }
        }
    }

    Ok(result)
}

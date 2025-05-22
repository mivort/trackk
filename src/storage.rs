use crate::args::{EntryArgs, FilterArgs, ModArgs};
use crate::config::Config;
use crate::index::Index;
use crate::issue::{Bucket, Issue};
use crate::prelude::*;

use anyhow::Context;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::rc::Rc;
use time::{Date, UtcDateTime};
use uuid::Uuid;
use walkdir::WalkDir;

/// Access storage bucket if it exists and add new entry to it.
pub fn add_entry(entry: &EntryArgs, config: &Config) -> Result<()> {
    let now = UtcDateTime::now();
    let date = now.date();

    let (mut bucket, path) = fetch_new_bucket(&date, config)?;

    let new_uuid = Uuid::new_v4().to_string();

    if bucket.entries.iter().any(|e| e.id == new_uuid) {
        bail!("collision has occured: task uuid exists");
    }

    let ts = now.unix_timestamp();
    let new_entry = Issue {
        id: new_uuid,
        title: entry.title.clone().unwrap_or_default(),
        status: unwrap_some_or!(&entry.status, { &config.defaults.status_initial }).clone(),
        created: ts,
        modified: ts,
        ..Default::default()
    };

    let mut index = Index::load(config)?;
    index.update_status(&path, &new_entry);
    index.write()?;

    let insert = bucket.entries.iter().position(|e| new_entry.id < e.id);
    if let Some(insert) = insert {
        bucket.entries.insert(insert, new_entry);
    } else {
        bucket.entries.push(new_entry);
    };

    write_bucket(&bucket, &path)
}

/// Find entry using the filter and update its properties.
pub fn modify_entries(args: &ModArgs, config: &Config) -> Result<()> {
    // TODO: ask if multiple entries are expected
    let mut changes = 0;

    for entry in WalkDir::new(&config.data) {
        let entry = entry?;

        if entry.depth() < 2 || entry.file_type().is_dir() {
            continue;
        }

        let data = File::open(entry.path())?;
        let reader = BufReader::new(data);
        let mut bucket: Bucket = serde_json::from_reader(reader).with_context(|| {
            format!(
                "Unable to read the bucket: {}",
                entry.path().to_string_lossy()
            )
        })?;

        let mut has_changes = false;
        for issue in &mut bucket.entries {
            if issue.match_filter(&args.filter) {
                issue.apply_args(&args.entry);
                has_changes = true;
                changes += 1;
            }
        }

        if has_changes {
            write_bucket(&bucket, entry.path())?;
        }
    }

    println!("Updated {changes} entry(es)");

    Ok(())
}

/// Produce the list of entries to display or modify.
pub fn fetch_entries(filter: &FilterArgs, config: &Config) -> Result<Vec<(Issue, Rc<str>)>> {
    if filter.all {
        return filter_all_entries(filter, config);
    }

    filter_active_entries(filter, config)
}

/// Create the storage bucket using the current date.
fn fetch_new_bucket(date: &Date, config: &Config) -> Result<(Bucket, String)> {
    let year = date.year();
    let month = date.month() as i32;
    let data = &config.data;
    let directory = format!("{data}/{year}");

    fs::create_dir_all(&directory)?;

    let path = format!("{directory}/{month:02}.json");
    let bucket = fetch_bucket(&path)?;

    Ok((bucket, path))
}

/// Fetch bucket data if it exists, create empty bucket data otherwise.
fn fetch_bucket(path: impl AsRef<Path>) -> Result<Bucket> {
    let data = File::open(&path);
    let data = match data {
        Ok(d) => d,
        Err(_e) => return Ok(Bucket::new()),
    };

    let reader = BufReader::new(data);
    serde_json::from_reader(reader)
        .with_context(|| format!("Unable to read bucket: {}", path.as_ref().to_string_lossy()))
}

/// Serialize bucket data and store in provided path.
fn write_bucket(data: &Bucket, path: impl AsRef<Path>) -> Result<()> {
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

        let data = File::open(entry.path())?;
        let reader = BufReader::new(data);
        let bucket: Bucket = serde_json::from_reader(reader).with_context(|| {
            format!(
                "Unable to display the bucket: {}",
                entry.path().to_string_lossy()
            )
        })?;
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
fn filter_active_entries(filter: &FilterArgs, config: &Config) -> Result<Vec<(Issue, Rc<str>)>> {
    let mut cache = HashMap::<String, Rc<Bucket>>::new();
    let mut result = Vec::new();

    let index = Index::load(config)?;
    for (idx, e) in index.active().iter().enumerate() {
        let (bucket_path, id) = unwrap_some_or!(e.rsplit_once("/"), {
            bail!("Active index entry has missing path");
        });

        let bucket = unwrap_some_or!(cache.get(bucket_path), {
            &(|| -> Result<_> {
                let bucket = Rc::new(fetch_bucket(bucket_path)?);
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

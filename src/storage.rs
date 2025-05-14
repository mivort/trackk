use crate::args::{EntryArgs, FilterArgs, ModArgs};
use crate::config::Config;
use crate::issue::{Bucket, Issue};
use crate::prelude::*;

use anyhow::Context;
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

    if bucket.entries.iter().find(|e| e.id == new_uuid).is_some() {
        bail!("collision has occured: task uuid exists");
    }

    let ts = now.unix_timestamp();
    let new_entry = Issue {
        id: new_uuid,
        title: match &entry.title {
            Some(t) => t.clone(),
            None => String::new(),
        },
        status: match &entry.status {
            Some(s) => s.clone(),
            None => config.defaults.status.clone(),
        },
        created: ts,
        modified: ts,
        ..Default::default()
    };

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

    for entry in WalkDir::new(&config.data) {
        let entry = entry?;

        if entry.file_type().is_dir() {
            continue;
        }

        let data = File::open(entry.path())?;
        let reader = BufReader::new(data);
        let mut bucket: Bucket = serde_json::from_reader(reader)?;

        if let Some(id) = &args.filter.id {
            for issue in &mut bucket.entries {
                if issue.id.starts_with(id) {
                    issue.apply_args(&args.entry);
                    write_bucket(&bucket, entry.path())?;

                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

/// Produce the list of entries to display or modify.
pub fn fetch_entries(filter: &FilterArgs, config: &Config) -> Result<Vec<(Issue, Rc<str>)>> {
    filter_entries(filter, config)
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
fn fetch_bucket(path: &String) -> Result<Bucket> {
    let data = File::open(path);
    let data = match data {
        Ok(d) => d,
        Err(_e) => return Ok(Bucket::new()),
    };

    let reader = BufReader::new(data);
    Ok(
        serde_json::from_reader(reader)
            .with_context(|| format!("Unable to read bucket: {path}"))?,
    )
}

/// Serialize bucket data and store in provided path.
fn write_bucket(data: &Bucket, path: impl AsRef<Path>) -> Result<()> {
    let output = serde_json::to_string_pretty(data)?;
    fs::write(path, output)?;

    Ok(())
}

/// Iterate over buckets and produce the list of entries which qualify.
fn filter_entries(filter: &FilterArgs, config: &Config) -> Result<Vec<(Issue, Rc<str>)>> {
    let mut output = Vec::new();

    for entry in WalkDir::new(&config.data) {
        let entry = entry?;

        if entry.file_type().is_dir() {
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

        if let Some(id) = &filter.id {
            for issue in bucket.entries {
                if issue.id.starts_with(id) {
                    output.push((issue, path.clone()));
                    return Ok(output);
                }
            }
            continue;
        }

        for issue in bucket.entries {
            output.push((issue, path.clone()));
        }
    }

    Ok(output)
}

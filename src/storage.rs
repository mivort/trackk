use crate::args::{EntryArgs, FilterArgs, ModArgs};
use crate::config::Config;
use crate::issue::{Bucket, Issue};
use crate::prelude::*;

use std::collections::hash_map::Entry;
use std::fs::{self, File};
use std::io::BufReader;
use std::rc::Rc;
use time::{Date, UtcDateTime};
use uuid::Uuid;
use walkdir::WalkDir;

/// Access storage bucket if it exists and add new entry to it.
pub fn add_entry(entry: &EntryArgs, config: &Config) -> Result<()> {
    let now = UtcDateTime::now();
    let date = now.date();

    let (mut bucket, path) = fetch_new_bucket(&date, config)?;

    let new_uuid = Uuid::new_v4();

    let mut new_entry = match bucket.entries.entry(new_uuid) {
        Entry::Occupied(_) => {
            bail!("collision has occured: task uuid exists");
        }
        Entry::Vacant(entry) => entry.insert_entry(Issue::default()),
    };
    let new_entry = new_entry.get_mut();
    new_entry.title = match &entry.title {
        Some(t) => t.clone(),
        None => String::new(),
    };
    new_entry.status = match &entry.status {
        Some(s) => s.clone(),
        None => config.defaults.status.clone(),
    };

    let ts = now.unix_timestamp();
    new_entry.created = ts;
    new_entry.modified = ts;

    write_bucket(&bucket, &path)
}

/// Find entry using the filter and update its properties.
pub fn modify_entry(entry: &ModArgs, config: &Config) -> Result<()> {
    let _entries = filter_entries(&entry.filter, config)?;

    // TODO: ask if multiple entries are expected

    Ok(())
}

/// Produce the list of entries to display or modify.
pub fn fetch_entries(filter: &FilterArgs, config: &Config) -> Result<Vec<(Uuid, Issue, Rc<str>)>> {
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
    Ok(serde_json::from_reader(reader)?)
}

/// Serialize bucket data and store in provided path.
fn write_bucket(data: &Bucket, path: &String) -> Result<()> {
    let output = serde_json::to_string_pretty(data)?;
    fs::write(path, output)?;

    Ok(())
}

/// Iterate over buckets and produce the list of entries which qualify.
fn filter_entries(_filter: &FilterArgs, config: &Config) -> Result<Vec<(Uuid, Issue, Rc<str>)>> {
    let mut output = Vec::new();

    for entry in WalkDir::new(&config.data) {
        let entry = entry?;

        if entry.file_type().is_dir() {
            continue;
        }

        let data = File::open(entry.path())?;
        let reader = BufReader::new(data);
        let bucket: Bucket = serde_json::from_reader(reader)?;
        let path = Rc::<str>::from(entry.path().to_string_lossy());

        for (uuid, issue) in bucket.entries {
            output.push((uuid, issue, path.clone()));
        }
    }

    Ok(output)
}

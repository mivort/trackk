use crate::args::{EntryArgs, ModArgs};
use crate::config::Config;
use crate::issue::Bucket;
use crate::prelude::*;

use std::fs;
use time::UtcDateTime;

/// Access storage bucket if it exists and add new entry to it.
pub fn add_entry(_entry: &EntryArgs, config: &Config) -> Result<()> {
    let (_bucket, _path) = fetch_new_bucket(config)?;

    Ok(())
}

/// Find entry using the filter and update its properties.
pub fn modify_entry(_entry: &ModArgs, _config: &Config) -> Result<()> {
    Ok(())
}

/// Create the storage bucket using the current date.
fn fetch_new_bucket(config: &Config) -> Result<(Bucket, String)> {
    let date = UtcDateTime::now().date();
    let year = date.year();
    let month = date.month() as i32;
    let data = &config.data;
    let directory = format!("{data}/{year}");

    fs::create_dir_all(&directory)?;

    let bucket = format!("{directory}/{month:02}.json");

    Ok((Bucket::new(), bucket))
}

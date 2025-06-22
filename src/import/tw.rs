use std::path::Path;

use serde_derive::Deserialize;

use crate::prelude::*;

/// Taskwarrior export data format schema.
#[derive(Deserialize)]
#[allow(unused)]
struct TWData {
    uuid: Box<str>,
    entry: Box<str>,
    description: Box<str>,

    #[serde(default)]
    modified: Box<str>,

    #[serde(default)]
    status: Box<str>,

    #[serde(default)]
    tags: Vec<Box<str>>,

    #[serde(default)]
    annotations: Vec<TWAnnotation>,
    // TODO: P1: add 'depends' handling
}

#[derive(Deserialize)]
#[allow(unused)]
struct TWAnnotation {
    #[serde(default)]
    entry: Box<str>,

    #[serde(default)]
    description: Box<str>,
}

// Importer for Taskwarrior v2 JSON export format.
pub fn import(_file: impl AsRef<Path>) -> Result<()> {
    // TODO: P3: implement import from taskwarrior

    Ok(())
}

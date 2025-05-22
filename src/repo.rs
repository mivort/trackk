use anyhow::Context;
use std::fs;
use std::process::Command;

use crate::config::Config;
use crate::prelude::*;

/// Check repository validity: merge tool, etc.
pub fn check_repo() {}

/// Run git to create repo, set the main settings.
pub fn init_repo(config: &Config) -> Result<()> {
    fs::create_dir_all(&config.data)
        .with_context(|| format!("Unable to create storage directory at '{}'", config.data))?;

    Command::new("git")
        .current_dir(&config.data)
        .arg("init")
        .output()
        .with_context(|| format!("Unable to create repo at '{}'", config.data))?;

    Ok(())
}

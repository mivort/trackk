use std::fs;
use std::process::Command;

use crate::config::Config;
use crate::prelude::*;

/// Check repository validity: merge tool, etc.
pub fn check_repo(config: &Config) -> Result<()> {
    // TODO: P2: implement method to produce check report

    info!("Data directory: {}", config.data_path()?.to_string_lossy());

    Ok(())
}

/// Run VCS to create repo, set the main settings.
pub fn init_repo(config: &Config) -> Result<()> {
    let data_path = config.data_path()?;
    let issues_path = config.issues_path()?;

    fs::create_dir_all(&issues_path).with_context(|| {
        format!(
            "Unable to create storage directory at '{}'",
            data_path.to_string_lossy()
        )
    })?;

    Command::new("git")
        .current_dir(&data_path)
        .arg("init")
        .output()
        .with_context(|| format!("Unable to create repo at '{}'", data_path.to_string_lossy()))?;

    Ok(())
}

/// Pull and push local changes.
pub fn sync_repo(_config: &Config) -> Result<()> {
    Ok(())
}

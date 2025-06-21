use std::fs;
use std::path::Path;

use crate::config::{Config, SyncDriverMode};
use crate::prelude::*;
use crate::sync::driver::SyncDriver;
use crate::sync::git::Git;

/// Check repository validity: merge tool, etc.
pub fn check_repo(config: &Config) -> Result<()> {
    // TODO: P2: implement method to produce check report

    info!("Data directory: {}", config.data_path()?.to_string_lossy());

    Ok(())
}

/// Run VCS to create repo, set the main settings.
pub fn init_repo(config: &Config, clone: Option<&str>) -> Result<()> {
    let data_path = config.data_path()?;
    let entries_path = config.issues_path()?;

    info!("Data directory: {}", data_path.to_string_lossy());
    info!("Entries directory: {}", entries_path.to_string_lossy());

    fs::create_dir_all(&entries_path).with_context(|| {
        format!(
            "Unable to create storage directory at '{}'",
            data_path.to_string_lossy()
        )
    })?;

    match config.sync.driver {
        SyncDriverMode::Git => init_driver::<Git>(data_path, clone)?,
        SyncDriverMode::Custom => todo!(),
    };

    Ok(())
}

/// Call specific init driver.
pub fn init_driver<D>(path: impl AsRef<Path>, url: Option<&str>) -> Result<()>
where
    D: SyncDriver,
{
    if let Some(url) = url { D::clone_repo(url, path) } else { D::init_repo(path) }
}

/// Pull and push local changes.
pub fn sync_repo(config: &Config) -> Result<()> {
    info!("Repo sync started");

    match config.sync.driver {
        SyncDriverMode::Git => todo!(),
        SyncDriverMode::Custom => todo!(),
    }
}

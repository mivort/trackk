use std::fs;
use std::path::Path;

use crate::args::InitArgs;
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
pub fn init_repo(config: &Config, args: &InitArgs) -> Result<()> {
    let data_path = config.data_path()?;
    let entries_path = config.entries_path()?;

    info!("Data directory: {}", data_path.to_string_lossy());
    info!("Entries directory: {}", entries_path.to_string_lossy());

    fs::create_dir_all(&entries_path).with_context(|| {
        format!(
            "Unable to create storage directory at '{}'",
            data_path.to_string_lossy()
        )
    })?;

    if args.no_sync {
        info!("Sync setup disabled: skip");
        return Ok(());
    }

    match config.sync.driver {
        SyncDriverMode::Git => init_driver::<Git>(data_path, args),
        SyncDriverMode::Custom => todo!(),
    }
    .context("Repo init failed")
}

/// Call specific init driver.
pub fn init_driver<D>(path: impl AsRef<Path>, args: &InitArgs) -> Result<()>
where
    D: SyncDriver,
{
    if let Some(url) = &args.clone {
        D::clone_repo(url, args, path)
    } else {
        D::init_repo(args, path)
    }
}

/// Create commit in the repo.
pub fn commit_repo(config: &Config) -> Result<()> {
    match config.sync.driver {
        SyncDriverMode::Git => Git::commit_repo(config.data_path()?),
        SyncDriverMode::Custom => todo!(),
    }
}

/// Pull and push local changes.
pub fn sync_repo(config: &Config) -> Result<()> {
    info!("Repo sync started");

    match config.sync.driver {
        SyncDriverMode::Git => Git::sync_repo(config.data_path()?),
        SyncDriverMode::Custom => todo!(),
    }
    .context("Repo sync failed")
}

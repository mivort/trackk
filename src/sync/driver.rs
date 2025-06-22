use std::path::Path;

use crate::args::InitArgs;
use crate::config::Config;
use crate::prelude::*;

/// Shared trait which provides sync driver interface.
pub trait SyncDriver {
    /// Download the remote repo into the specified directory.
    fn clone_repo(url: &str, args: &InitArgs, config: &Config) -> Result<()>;

    /// Initialize new sync repo.
    fn init_repo(args: &InitArgs, config: &Config) -> Result<()>;

    /// Perform commit in the repository.
    fn commit_repo(target: impl AsRef<Path>) -> Result<()>;

    /// Download changes from the remote and upload local changes.
    fn sync_repo(target: impl AsRef<Path>) -> Result<()>;
}

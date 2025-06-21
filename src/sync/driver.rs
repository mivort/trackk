use std::path::Path;

use crate::prelude::*;

/// Shared trait which provides sync driver interface.
pub trait SyncDriver {
    /// Download the remote repo into the specified directory.
    fn clone_repo(url: &str, target: impl AsRef<Path>) -> Result<()>;

    /// Initialize new sync repo.
    fn init_repo(target: impl AsRef<Path>) -> Result<()>;
}

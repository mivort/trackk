use std::path::Path;

use crate::prelude::*;
use crate::sync::driver::SyncDriver;

use std::process::Command;

pub struct Git;

impl SyncDriver for Git {
    fn init_repo(path: impl AsRef<Path>) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.current_dir(&path).arg("init");
        cmd.spawn().with_context(|| {
            format!(
                "Unable to create repo at '{}'",
                path.as_ref().to_string_lossy()
            )
        })?;

        Ok(())
    }

    fn clone_repo(url: &str, target: impl AsRef<Path>) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.args(["clone", url]);
        cmd.arg(target.as_ref());

        cmd.spawn()?;

        Ok(())
    }
}

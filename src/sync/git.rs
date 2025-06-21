use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::index::ACTIVE_INDEX;
use crate::prelude::*;
use crate::sync::driver::SyncDriver;

use std::process::Command;

pub struct Git;

impl SyncDriver for Git {
    fn init_repo(path: impl AsRef<Path>) -> Result<()> {
        info!("Running 'git init' in repo directory");

        let mut cmd = Command::new("git");
        cmd.current_dir(&path).arg("init");
        cmd.spawn()
            .with_context(|| {
                format!(
                    "Unable to create repo at '{}'",
                    path.as_ref().to_string_lossy()
                )
            })?
            .wait()?;

        let mut path = PathBuf::from(path.as_ref());
        path.push(".gitignore");

        let gitignore = match File::open(&path) {
            Err(e) if matches!(e.kind(), ErrorKind::NotFound) => File::create(&path)?,
            Err(e) => return Err(anyhow!(e)),
            Ok(f) => f,
        };

        let reader = BufReader::new(gitignore);
        for line in reader.lines() {
            if line?.trim_end() == ACTIVE_INDEX {
                info!("Local index is already in .gitignore: skip");
                return Ok(());
            }
        }

        info!("Adding local index to .gitignore");
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)
            .context("Unable to open .gitignore for writing")?;
        file.write(ACTIVE_INDEX.as_bytes())?;

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

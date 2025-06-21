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

        let mut cmd = git_command(&path);
        let spawn = cmd.arg("init").spawn();

        match spawn {
            Err(e) if matches!(e.kind(), ErrorKind::NotFound) => {
                return Err(anyhow!(e)).context("'git' command is not found");
            }
            Err(e) => {
                return Err(anyhow!(e)).with_context(|| {
                    format!(
                        "Unable to create repo at '{}'",
                        path.as_ref().to_string_lossy()
                    )
                });
            }
            Ok(mut spawn) => spawn.wait()?,
        };

        let mut path = PathBuf::from(path.as_ref());
        path.push(".gitignore");

        let gitignore = match File::open(&path) {
            Err(e) if matches!(e.kind(), ErrorKind::NotFound) => File::create(&path)?,
            Err(e) => return Err(anyhow!(e)).context("Unable to access .gitignore"),
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

        // TODO: P3: create .gitattributes
        // TODO: P3: setup git merge driver
        // TODO: P3: ask git user name
        // TODO: P3: ask git user email

        Ok(())
    }

    fn clone_repo(url: &str, target: impl AsRef<Path>) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.args(["clone", url]);
        cmd.arg(target.as_ref());

        cmd.spawn().context("Unable to run 'git clone'")?.wait()?;

        Ok(())
    }

    fn sync_repo(target: impl AsRef<Path>) -> Result<()> {
        info!("Creating new commit");

        let mut cmd = git_command(&target);
        cmd.args(["add", "--all"]);
        if !cmd.spawn()?.wait()?.success() {
            bail!("Unable to stage repo");
        };

        let mut cmd = git_command(&target);
        cmd.args(["diff", "--name-only", "--cached"]);

        let output = cmd.output()?;
        let file = String::from_utf8_lossy(&output.stdout);
        let file = file.lines().next().unwrap_or("n/a");

        let mut cmd = git_command(&target);
        cmd.args(["commit", "-m"]);
        cmd.arg(file);
        if !cmd.spawn()?.wait()?.success() {
            bail!("Unable to commit changes");
        }

        info!("Running 'git pull --rebase'");
        let mut cmd = git_command(&target);
        cmd.args(["pull", "--rebase"]);
        if !cmd.spawn()?.wait()?.success() {
            bail!("Unable to pull remote changes");
        }

        info!("Running 'git push'");
        let mut cmd = git_command(&target);
        cmd.arg("push");
        if !cmd.spawn()?.wait()?.success() {
            bail!("Unable to push local changes to remote");
        }

        Ok(())
    }
}

/// Create git command instance.
fn git_command(path: impl AsRef<Path>) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(&path);
    cmd
}

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::args::InitArgs;
use crate::index::ACTIVE_INDEX;
use crate::sync::driver::SyncDriver;
use crate::{input, prelude::*};

use std::process::Command;

pub struct Git;

impl SyncDriver for Git {
    fn init_repo(args: &InitArgs, path: impl AsRef<Path>) -> Result<()> {
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

        'index_ignore: {
            let mut ignorepath = PathBuf::from(path.as_ref());
            ignorepath.push(".gitignore");

            let gitignore = match File::open(&ignorepath) {
                Err(e) if matches!(e.kind(), ErrorKind::NotFound) => File::create(&ignorepath)?,
                Err(e) => return Err(anyhow!(e)).context("Unable to access .gitignore"),
                Ok(f) => f,
            };

            let reader = BufReader::new(gitignore);
            for line in reader.lines() {
                if line?.trim_end() == ACTIVE_INDEX {
                    info!("Local index is already in .gitignore: skip");
                    break 'index_ignore;
                }
            }

            info!("Adding local index to .gitignore");
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&ignorepath)
                .context("Unable to open .gitignore for writing")?;
            file.write_all(ACTIVE_INDEX.as_bytes())?;
        }

        // TODO: P3: create .gitattributes
        // TODO: P3: setup git merge driver

        git_config_setup(&path, args)
    }

    fn clone_repo(url: &str, args: &InitArgs, target: impl AsRef<Path>) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.args(["clone", url]);
        cmd.arg(target.as_ref());

        cmd.spawn().context("Unable to run 'git clone'")?.wait()?;

        git_config_setup(&target, args)
    }

    fn sync_repo(target: impl AsRef<Path>) -> Result<()> {
        Self::commit_repo(&target)?;

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

    fn commit_repo(target: impl AsRef<Path>) -> Result<()> {
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
        let file = format!("sync: {}", file);

        let mut cmd = git_command(&target);
        cmd.args(["commit", "-m"]);
        cmd.arg(file);
        if !cmd.spawn()?.wait()?.success() {
            bail!("Unable to commit changes");
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

/// Set git config value.
fn git_config(path: impl AsRef<Path>, key: &str, value: impl FnOnce() -> String) -> Result<()> {
    let mut cmd = git_command(&path);
    cmd.arg("config");
    cmd.arg(key);
    if cmd.output()?.status.success() {
        return Ok(());
    }
    let mut cmd = git_command(&path);
    cmd.arg("config");
    cmd.arg(key);
    cmd.arg(value());
    if !cmd.spawn()?.wait()?.success() {
        bail!("Unable to set git config {}", key);
    }
    Ok(())
}

/// Perform setup common for new repos and clones.
fn git_config_setup(path: impl AsRef<Path>, args: &InitArgs) -> Result<()> {
    info!("Setting up 'git config'");

    git_config(&path, "user.name", || {
        if let Some(user) = &args.user {
            return user.to_string();
        }
        let name = input::prompt(concat!(
            "Enter git config user.name: [",
            env!("CARGO_PKG_NAME"),
            "] "
        ))
        .unwrap_or_default();
        if name.is_empty() { String::from(env!("CARGO_PKG_NAME")) } else { name }
    })?;

    git_config(&path, "user.email", || {
        if let Some(email) = &args.email {
            return email.to_string();
        }
        let email = input::prompt(concat!(
            "Enter git config user.email: [@",
            env!("CARGO_PKG_NAME"),
            "] "
        ))
        .unwrap_or_default();
        if email.is_empty() { String::from(concat!("@", env!("CARGO_PKG_NAME"))) } else { email }
    })?;

    git_config(
        &path,
        concat!("merge.", env!("CARGO_PKG_NAME"), "-bucket.name"),
        || concat!("'", env!("CARGO_PKG_NAME"), " json driver'").into(),
    )?;
    git_config(
        &path,
        concat!("merge.", env!("CARGO_PKG_NAME"), "-driver.driver"),
        || concat!("'", env!("CARGO_PKG_NAME"), " merge %O %A %B'").into(),
    )?;

    Ok(())
}

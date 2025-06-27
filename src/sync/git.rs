use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::app::App;
use crate::args::InitArgs;
use crate::index::ACTIVE_INDEX;
use crate::sync::driver::SyncDriver;
use crate::{input, prelude::*};

use std::process::Command;

pub struct Git;

/// Gitattributes file content.
const GIT_ATTR: &str = "*.json merge=trackk-bucket";

impl SyncDriver for Git {
    fn init_repo(args: &InitArgs, app: &App) -> Result<()> {
        info!("Running 'git init' in repo directory");

        let path = app.config.data_path()?;
        let mut cmd = git_command(&path);
        let spawn = cmd.arg("init").spawn();

        match spawn {
            Err(e) if matches!(e.kind(), ErrorKind::NotFound) => {
                return Err(anyhow!(e)).context("'git' command is not found");
            }
            Err(e) => {
                return Err(anyhow!(e)).with_context(|| {
                    format!("Unable to create repo at '{}'", path.to_string_lossy())
                });
            }
            Ok(mut spawn) => spawn.wait()?,
        };

        let entries_path = app.config.entries_path()?;
        fs::create_dir_all(&entries_path).with_context(|| {
            format!(
                "Unable to create entries directory at '{}'",
                entries_path.to_string_lossy()
            )
        })?;

        let mut ignorepath = PathBuf::from(&path);
        ignorepath.push(".gitignore");
        append_line(&ignorepath, ACTIVE_INDEX)?;

        let mut attrpath = PathBuf::from(entries_path);
        attrpath.push(".gitattributes");
        append_line(&attrpath, GIT_ATTR)?;

        git_config_setup(&path, args)
    }

    fn clone_repo(url: &str, args: &InitArgs, app: &App) -> Result<()> {
        let target = app.config.data_path()?;

        let mut cmd = Command::new("git");
        cmd.args(["clone", url]);
        cmd.arg(&target);
        cmd.spawn().context("Unable to run 'git clone'")?.wait()?;

        Self::init_repo(args, app)
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
        cmd.spawn()?.wait()?;

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
fn git_config(
    path: impl AsRef<Path>,
    key: &str,
    overwrite: bool,
    value: impl FnOnce() -> String,
) -> Result<()> {
    if !overwrite {
        let mut cmd = git_command(&path);
        cmd.arg("config");
        cmd.arg(key);
        if cmd.output()?.status.success() {
            return Ok(());
        }
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

    git_config(&path, "user.name", false, || {
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

    git_config(&path, "user.email", false, || {
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

    let name = concat!("merge.", env!("CARGO_PKG_NAME"), "-bucket.name");
    let driver = concat!("merge.", env!("CARGO_PKG_NAME"), "-bucket.driver");

    let exe = std::env::current_exe()
        .context("Unable to locate own executable to set as merge driver")?;
    let exe = exe
        .file_name()
        .unwrap_or_else(|| OsStr::new(env!("CARGO_BIN_NAME")))
        .to_string_lossy();
    info!("Setting current executable name ({}) as merge driver", exe);

    let command = format!("{} merge %O %A %B", exe);

    git_config(&path, name, true, || {
        concat!("'", env!("CARGO_PKG_NAME"), " json bucket merge driver'").into()
    })?;

    git_config(&path, driver, true, || command)?;

    Ok(())
}

/// Check if file contains the line. If not, append at the end.
fn append_line(path: impl AsRef<Path>, value: &str) -> Result<()> {
    let name = path
        .as_ref()
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    match File::open(&path) {
        Err(e) if matches!(e.kind(), ErrorKind::NotFound) => {}
        Err(e) => {
            return Err(anyhow!(e)).with_context(|| format!("Unable to access {name}"));
        }
        Ok(f) => {
            let reader = BufReader::new(f);
            for line in reader.lines() {
                if line?.trim_end() == value {
                    info!("Repo's {name} already set: skip");
                    return Ok(());
                }
            }
        }
    };

    info!("Adding merge driver to {name}");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("Unable to open {name} for writing"))?;
    file.write_all(value.as_bytes())?;
    file.write_all("\n".as_bytes())?;

    Ok(())
}

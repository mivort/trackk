use std::fs;

use assert_cmd::Command;
use testdir::testdir;

const BIN_NAME: &str = "trk";

#[cfg(test)]
fn cmd_base() -> Command {
    let dir = testdir!();
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.env("TRACKK_DATA", dir)
        .env("TRACKK_CONFIG", "")
        .env("RUST_BACKTRACE", "1");
    cmd
}

#[cfg(test)]
fn prepare_storage() {
    let mut cmd = cmd_base();
    cmd.args(&["init", "--user", "test", "--email", "test"]);
    cmd.assert().success();
}

/// Check if storage is initialized, issue can be created and removed.
#[test]
fn init_storage() {
    prepare_storage();

    assert!(fs::exists(testdir!().join(".git")).unwrap());
    assert!(fs::exists(testdir!().join("entries")).unwrap());

    let mut cmd = cmd_base();
    cmd.args(&["all"]);
    cmd.assert().success();

    let mut cmd = cmd_base();
    cmd.args(&["add", "test entry"]);
    cmd.assert().success();

    assert!(fs::exists(testdir!().join("active")).unwrap());

    let mut cmd = cmd_base();
    cmd.args(&["1", "mod", "--status=deleted"]);
    cmd.assert().success();
}

/// Test if task gets copied on completion.
#[test]
fn repeat_task() {
    prepare_storage();

    // Create repeatable task
    let mut cmd = cmd_base();
    cmd.args(&["add", "test entry", "--repeat=2d"]);
    cmd.assert().success();

    // Mark task as complete
    let mut cmd = cmd_base();
    cmd.args(&["--id=1", "mod", "--status=completed"]);
    cmd.assert().success();

    // Mark repeated task as complete and stop repetition
    let mut cmd = cmd_base();
    cmd.args(&["--id=1", "mod", "--status=completed", "--repeat="]);
    cmd.assert().success();

    // Ensure no more copies were created
    let mut cmd = cmd_base();
    cmd.args(&["--id=1", "mod", "--status=completed"]);
    cmd.assert().failure();
}

/// Try to run different report types to check if templates are valid.
#[test]
fn show_reports() {
    prepare_storage();

    // Create test task
    let mut cmd = cmd_base();
    cmd.args(&["add", "test entry", "--due=3d"]);
    cmd.assert().success();

    // Check regular report.
    let mut cmd = cmd_base();
    cmd.assert().success();

    // Check calendar report.
    let mut cmd = cmd_base();
    cmd.args(&["list", "calendar"]);
    cmd.assert().success();
}

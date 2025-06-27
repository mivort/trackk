use std::fs;

use assert_cmd::Command;
use testdir::testdir;

const BIN_NAME: &str = "trk";

#[cfg(test)]
fn cmd_base() -> Command {
    let dir = testdir!();
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.env("TRACKK_DATA", dir).env("TRACKK_CONFIG", "");
    cmd
}

#[test]
fn init_storage() {
    let mut cmd = cmd_base();
    cmd.args(&["init", "--user", "test", "--email", "test"]);
    cmd.assert().success();

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

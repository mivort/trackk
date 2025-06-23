use std::env;

use assert_cmd::Command;

const BIN_NAME: &str = "trk";

#[test]
fn init_storage() {
    let dir = env!("CARGO_TARGET_TMPDIR");

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    let cmd = cmd.args(&["--data", dir, "init", "--user", "test", "--email", "test"]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    let cmd = cmd.args(&["--data", dir, "all"]);
    cmd.assert().success();
}

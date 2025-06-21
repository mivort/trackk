use std::env;

use assert_cmd::Command;

#[test]
fn init_storage() {
    let dir = env!("CARGO_TARGET_TMPDIR");

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let cmd = cmd.args(&["--data", dir, "init", "--user", "test", "--email", "test"]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let cmd = cmd.args(&["--data", dir, "all"]);
    cmd.assert().success();
}

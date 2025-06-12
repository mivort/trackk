use std::env;

use assert_cmd::Command;

#[test]
fn init_storage() {
    let dir = env!("CARGO_TARGET_TMPDIR");

    let mut cmd = Command::cargo_bin("trackit").unwrap();
    let cmd = cmd.args(&["--data", dir]);
    cmd.assert().success();
}

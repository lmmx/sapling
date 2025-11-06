#![allow(missing_docs)]
#[cfg(feature = "cli")]
use assert_cmd::cargo_bin_cmd;
#[cfg(feature = "cli")]
use predicates::prelude::*;

#[cfg(feature = "cli")]
#[test]
fn cli_help_succeeds() {
    cargo_bin_cmd!("sapling")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

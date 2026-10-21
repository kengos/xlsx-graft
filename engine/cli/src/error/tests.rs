//! Unit tests for `CliError`: the message `main.rs` prints for each variant.

use super::CliError;
use serde::ser::Error as _;
use std::io;

#[test]
fn each_variant_says_which_step_failed_and_why() {
    let write = CliError::Write(io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed"));
    assert_eq!(write.to_string(), "failed to write the output: pipe closed");
    let serialize = CliError::from(serde_json::Error::custom("refused"));
    assert_eq!(
        serialize.to_string(),
        "failed to serialize the output: refused"
    );
}

//! Unit tests for `CliError`: the message `exit_status` prints for each
//! variant.

use super::CliError;
use serde::ser::Error as _;
use std::io;
use std::path::PathBuf;
use xlsx_graft_package::PackageError;
use xlsx_graft_read::ReadError;

#[test]
fn each_variant_says_which_step_failed_and_why() {
    let open = CliError::Open {
        path: PathBuf::from("book.xlsx"),
        source: io::Error::new(io::ErrorKind::NotFound, "no such file"),
    };
    assert_eq!(open.to_string(), "failed to open book.xlsx: no such file");
    let package = PackageError::Io(io::Error::other("the disk went away"));
    let read = CliError::from(ReadError::from(package));
    assert_eq!(
        read.to_string(),
        "failed to read the workbook: reading the package failed: the disk went away"
    );
    let write = CliError::Write(io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed"));
    assert_eq!(write.to_string(), "failed to write the output: pipe closed");
    let serialize = CliError::from(serde_json::Error::custom("refused"));
    assert_eq!(
        serialize.to_string(),
        "failed to serialize the output: refused"
    );
}

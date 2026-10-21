//! Unit tests for `McpError`: the message `main.rs` prints.

use super::McpError;
use std::io;

#[test]
fn an_io_failure_says_the_transport_failed_and_why() {
    let err = McpError::from(io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed"));
    assert_eq!(err.to_string(), "stdio transport failed: pipe closed");
}

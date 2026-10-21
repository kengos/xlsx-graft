//! Unit tests for `serve`: the shared conversation driver and the transport
//! failures here, the handshake and the protocol errors in the submodules.

mod handshake;
mod protocol;

use super::serve;
use crate::McpError;
use serde_json::Value;
use std::io::{self, BufReader, Cursor, Read};

const PING: &str = r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#;

/// Runs one whole conversation over in-memory stdio; returns the parsed
/// response lines.
fn talk(input: &str) -> Vec<Value> {
    let mut out = Vec::new();
    serve(&mut Cursor::new(input.as_bytes()), &mut out).unwrap();
    String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn an_empty_stdin_ends_cleanly_with_nothing_written() {
    assert!(talk("").is_empty());
}

#[test]
fn a_stdout_that_refuses_a_response_ends_the_loop_with_an_io_error() {
    // A zero-length buffer takes no bytes, like a full disk.
    let mut full: &mut [u8] = &mut [];
    let err = serve(&mut Cursor::new(PING.as_bytes()), &mut full).unwrap_err();
    assert!(matches!(err, McpError::Io(e) if e.kind() == io::ErrorKind::WriteZero));
}

#[test]
fn a_stdin_that_fails_ends_the_loop_with_an_io_error() {
    /// Fails every read.
    struct Broken;

    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::ConnectionReset.into())
        }
    }

    let mut out = Vec::new();
    let err = serve(&mut BufReader::new(Broken), &mut out).unwrap_err();
    assert!(matches!(err, McpError::Io(e) if e.kind() == io::ErrorKind::ConnectionReset));
    assert!(out.is_empty());
}

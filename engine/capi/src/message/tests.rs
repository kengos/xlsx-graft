//! Unit tests for the per-thread message.

use super::{failed, lent};
use crate::status::XLSX_GRAFT_ERR_IO;

/// This thread's message, copied out.
fn message() -> String {
    let (ptr, len) = lent();
    // SAFETY: `lent` points at `len` bytes of this thread's message, which
    // nothing replaces before they are copied.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn a_failure_keeps_its_message_and_is_the_io_status() {
    assert_eq!(failed("disk full"), XLSX_GRAFT_ERR_IO);
    assert_eq!(message(), "disk full");
    failed("gone");
    assert_eq!(message(), "gone");
}

#[test]
fn each_thread_has_a_message_of_its_own() {
    failed("here");
    let there = std::thread::spawn(message).join().unwrap();
    assert_eq!((message().as_str(), there.as_str()), ("here", ""));
}

//! Unit tests for what the entry points share: the guard around a body,
//! the UTF-8 check, and `xlsx_graft_error_message`.

use super::{run, text, xlsx_graft_error_message};
use crate::message::failed;
use crate::status::*;
use std::ptr;

#[test]
fn a_body_is_its_status_or_ok() {
    assert_eq!(run(|| Ok(())), XLSX_GRAFT_OK);
    assert_eq!(
        run(|| Err(XLSX_GRAFT_ERR_SHEET_NOT_FOUND)),
        XLSX_GRAFT_ERR_SHEET_NOT_FOUND
    );
    assert_eq!(run(|| panic!("boom")), XLSX_GRAFT_ERR_PANIC);
}

#[test]
fn text_that_is_not_utf8_is_an_invalid_argument() {
    let bytes = [b'a', 0xff];
    // SAFETY: `bytes` holds two bytes for the call.
    let both = unsafe { text(bytes.as_ptr(), 2) };
    assert_eq!(both, Err(XLSX_GRAFT_ERR_INVALID_ARG));
    // SAFETY: as above, one of them.
    assert_eq!(unsafe { text(bytes.as_ptr(), 1) }, Ok("a"));
}

#[test]
fn the_message_of_the_last_failure_is_lent() {
    failed("no such file");
    let (mut ptr, mut len) = (ptr::null(), 0);
    // SAFETY: both arguments are writable slots of the declared types.
    let status = unsafe { xlsx_graft_error_message(&raw mut ptr, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the message is `len` bytes, alive until the next failure.
    let message = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert_eq!(message, b"no such file");
}

#[test]
fn a_null_out_pointer_is_refused_for_the_message() {
    let mut len = usize::MAX;
    // SAFETY: `len` is a writable slot; the null `out_ptr` is what is tested.
    let status = unsafe { xlsx_graft_error_message(ptr::null_mut(), &raw mut len) };
    assert_eq!((status, len), (XLSX_GRAFT_ERR_NULL_ARG, usize::MAX));
    let mut out = ptr::dangling();
    // SAFETY: `out` is a writable slot; the null `out_len` is what is tested.
    let status = unsafe { xlsx_graft_error_message(&raw mut out, ptr::null_mut()) };
    assert_eq!((status, out), (XLSX_GRAFT_ERR_NULL_ARG, ptr::dangling()));
}

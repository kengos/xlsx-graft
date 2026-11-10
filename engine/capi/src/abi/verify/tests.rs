//! Unit tests for the verify entry points: the null-argument refusals, an
//! empty input, a path that is not UTF-8, and a result read twice.

use super::{
    xlsx_graft_policy_check, xlsx_graft_verify_bytes, xlsx_graft_verify_close,
    xlsx_graft_verify_path, xlsx_graft_verify_result,
};
use crate::status::*;
use crate::verify::XlsxGraftVerify;
use std::ptr;

/// The result of a handle, as text.
fn result(verify: *mut XlsxGraftVerify) -> (*const u8, String) {
    let (mut ptr, mut len) = (ptr::null(), 0);
    // SAFETY: `verify` is a live handle; the out pointers are writable slots.
    let status = unsafe { xlsx_graft_verify_result(verify, &raw mut ptr, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the library lent `len` UTF-8 bytes, alive until the close.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    (ptr, String::from_utf8(bytes.to_vec()).unwrap())
}

#[test]
fn a_null_pointer_that_must_not_be_null_is_refused_and_nothing_is_written() {
    let mut out: *mut XlsxGraftVerify = ptr::null_mut();
    let some = b"x".as_ptr();
    // SAFETY: each call's out pointer is a writable slot or the null one
    // under test, and each other pointer is null with a length, or alive.
    let statuses = unsafe {
        [
            xlsx_graft_verify_path(ptr::null(), 1, ptr::null(), 0, &raw mut out),
            xlsx_graft_verify_path(some, 1, ptr::null(), 1, &raw mut out),
            xlsx_graft_verify_path(some, 1, ptr::null(), 0, ptr::null_mut()),
            xlsx_graft_verify_bytes(ptr::null(), 1, ptr::null(), 0, &raw mut out),
            xlsx_graft_verify_bytes(some, 1, ptr::null(), 1, &raw mut out),
            xlsx_graft_verify_bytes(some, 1, ptr::null(), 0, ptr::null_mut()),
            xlsx_graft_policy_check(ptr::null(), 1, &raw mut out),
            xlsx_graft_policy_check(some, 1, ptr::null_mut()),
        ]
    };
    assert_eq!(statuses, [XLSX_GRAFT_ERR_NULL_ARG; 8]);
    assert!(out.is_null());
    let (mut p, mut len) = (ptr::null(), 0);
    // SAFETY: the null handle and null out pointers are what is tested.
    let statuses = unsafe {
        [
            xlsx_graft_verify_result(ptr::null(), &raw mut p, &raw mut len),
            xlsx_graft_verify_result(
                ptr::NonNull::dangling().as_ptr(),
                ptr::null_mut(),
                &raw mut len,
            ),
            xlsx_graft_verify_result(
                ptr::NonNull::dangling().as_ptr(),
                &raw mut p,
                ptr::null_mut(),
            ),
        ]
    };
    assert_eq!(statuses, [XLSX_GRAFT_ERR_NULL_ARG; 3]);
    assert_eq!((p, len), (ptr::null(), 0));
}

#[test]
fn an_input_of_no_bytes_may_be_null() {
    let mut out: *mut XlsxGraftVerify = ptr::null_mut();
    // SAFETY: null with a length of 0 is no bytes; `out` a writable slot.
    let status = unsafe { xlsx_graft_verify_bytes(ptr::null(), 0, ptr::null(), 0, &raw mut out) };
    assert_eq!(status, XLSX_GRAFT_OK);
    assert!(result(out).1.starts_with(r#"{"ok":false,"failOn":"error""#));
    // SAFETY: `out` is a live handle, closed once.
    assert_eq!(unsafe { xlsx_graft_verify_close(out) }, XLSX_GRAFT_OK);

    // SAFETY: as above, for the policy.
    let status = unsafe { xlsx_graft_policy_check(ptr::null(), 0, &raw mut out) };
    assert_eq!(status, XLSX_GRAFT_OK);
    assert!(result(out).1.starts_with(r#"{"ok":true"#));
    // SAFETY: `out` is a live handle, closed once.
    unsafe { xlsx_graft_verify_close(out) };
}

#[test]
fn a_path_that_is_not_utf8_is_refused_and_nothing_is_written() {
    let mut out: *mut XlsxGraftVerify = ptr::null_mut();
    let path = b"\xff\xfe";
    // SAFETY: `path` is alive for the call, `out` a writable slot.
    let status =
        unsafe { xlsx_graft_verify_path(path.as_ptr(), path.len(), ptr::null(), 0, &raw mut out) };
    assert_eq!(status, XLSX_GRAFT_ERR_INVALID_ARG);
    assert!(out.is_null());
}

#[test]
fn the_result_is_the_same_bytes_every_time_and_a_null_close_does_nothing() {
    let mut out: *mut XlsxGraftVerify = ptr::null_mut();
    let path = b"/no/such/book.xlsx";
    // SAFETY: `path` is alive for the call, `out` a writable slot.
    let status =
        unsafe { xlsx_graft_verify_path(path.as_ptr(), path.len(), ptr::null(), 0, &raw mut out) };
    assert_eq!(status, XLSX_GRAFT_OK);
    let (first, text) = result(out);
    assert!(text.contains(r#""error":{"code":"io""#), "{text}");
    assert_eq!(result(out).0, first);
    // SAFETY: `out` is a live handle, closed once.
    let closed = unsafe { xlsx_graft_verify_close(out) };
    assert_eq!(closed, XLSX_GRAFT_OK);
    // SAFETY: a null handle is accepted.
    let status = unsafe { xlsx_graft_verify_close(ptr::null_mut()) };
    assert_eq!(status, XLSX_GRAFT_OK);
}

#[test]
fn options_that_are_too_large_or_not_utf8_are_refused_and_nothing_is_written() {
    let mut out: *mut XlsxGraftVerify = ptr::null_mut();
    let large = vec![b' '; xlsx_graft_host::MAX_REQUEST_BYTES + 1];
    for options in [&large[..], b"\xff"] {
        // SAFETY: `options` is alive for the call, `out` a writable slot.
        let status = unsafe {
            xlsx_graft_verify_bytes(
                ptr::null(),
                0,
                options.as_ptr(),
                options.len(),
                &raw mut out,
            )
        };
        assert_eq!(status, XLSX_GRAFT_ERR_INVALID_ARG);
        assert!(out.is_null());
    }
}

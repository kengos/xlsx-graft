//! Unit tests for the view entry points: the null-argument refusals, an
//! empty input, a path that is not UTF-8, options that are refused, and a
//! result read twice.

use super::{
    xlsx_graft_describe_sheet_bytes, xlsx_graft_describe_sheet_path, xlsx_graft_tree_bytes,
    xlsx_graft_tree_path, xlsx_graft_view_close, xlsx_graft_view_result,
};
use crate::status::*;
use crate::view::XlsxGraftView;
use std::ptr;

/// The result of a handle, as text.
fn result(view: *mut XlsxGraftView) -> (*const u8, String) {
    let (mut ptr, mut len) = (ptr::null(), 0);
    // SAFETY: `view` is a live handle; the out pointers are writable slots.
    let status = unsafe { xlsx_graft_view_result(view, &raw mut ptr, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the library lent `len` UTF-8 bytes, alive until the close.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    (ptr, String::from_utf8(bytes.to_vec()).unwrap())
}

/// A slot value that is not null and not a handle, to tell a call that wrote
/// `*out_view` from one that did not. Never read or closed.
fn sentinel() -> *mut XlsxGraftView {
    ptr::NonNull::<XlsxGraftView>::dangling().as_ptr()
}

#[test]
fn a_null_pointer_that_must_not_be_null_is_refused_and_nothing_is_written() {
    let mut out = sentinel();
    let some = b"x".as_ptr();
    // SAFETY: each call's out pointer is a writable slot or the null one
    // under test, and each other pointer is null with a length, or alive.
    let statuses = unsafe {
        [
            xlsx_graft_tree_path(ptr::null(), 1, ptr::null(), 0, &raw mut out),
            xlsx_graft_tree_path(some, 1, ptr::null(), 1, &raw mut out),
            xlsx_graft_tree_path(some, 1, ptr::null(), 0, ptr::null_mut()),
            xlsx_graft_tree_bytes(ptr::null(), 1, ptr::null(), 0, &raw mut out),
            xlsx_graft_tree_bytes(some, 1, ptr::null(), 1, &raw mut out),
            xlsx_graft_tree_bytes(some, 1, ptr::null(), 0, ptr::null_mut()),
            xlsx_graft_describe_sheet_path(ptr::null(), 1, ptr::null(), 0, &raw mut out),
            xlsx_graft_describe_sheet_path(some, 1, ptr::null(), 1, &raw mut out),
            xlsx_graft_describe_sheet_path(some, 1, ptr::null(), 0, ptr::null_mut()),
            xlsx_graft_describe_sheet_bytes(ptr::null(), 1, ptr::null(), 0, &raw mut out),
            xlsx_graft_describe_sheet_bytes(some, 1, ptr::null(), 1, &raw mut out),
            xlsx_graft_describe_sheet_bytes(some, 1, ptr::null(), 0, ptr::null_mut()),
        ]
    };
    assert_eq!(statuses, [XLSX_GRAFT_ERR_NULL_ARG; 12]);
    assert_eq!(out, sentinel());
    let (mut p, mut len) = (ptr::null(), 0);
    // SAFETY: the null handle and null out pointers are what is tested.
    let statuses = unsafe {
        [
            xlsx_graft_view_result(ptr::null(), &raw mut p, &raw mut len),
            xlsx_graft_view_result(
                ptr::NonNull::dangling().as_ptr(),
                ptr::null_mut(),
                &raw mut len,
            ),
            xlsx_graft_view_result(
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
fn a_null_pointer_of_length_0_is_read_as_no_bytes() {
    let mut out: *mut XlsxGraftView = ptr::null_mut();
    // SAFETY: null with a length of 0 is no bytes; `out` a writable slot.
    let status = unsafe { xlsx_graft_tree_bytes(ptr::null(), 0, ptr::null(), 0, &raw mut out) };
    assert_eq!(status, XLSX_GRAFT_OK);
    assert!(result(out).1.starts_with(r#"{"ok":false,"#));
    // SAFETY: `out` is a live handle, closed once.
    assert_eq!(unsafe { xlsx_graft_view_close(out) }, XLSX_GRAFT_OK);

    // SAFETY: as above, for the cells of a sheet.
    let status =
        unsafe { xlsx_graft_describe_sheet_bytes(ptr::null(), 0, ptr::null(), 0, &raw mut out) };
    assert_eq!(status, XLSX_GRAFT_OK);
    assert!(result(out).1.starts_with(r#"{"ok":false,"#));
    // SAFETY: `out` is a live handle, closed once.
    assert_eq!(unsafe { xlsx_graft_view_close(out) }, XLSX_GRAFT_OK);
}

#[test]
fn a_path_that_is_not_utf8_is_refused_and_nothing_is_written() {
    let mut out = sentinel();
    let path = b"\xff\xfe";
    // SAFETY: `path` is alive for the call, `out` a writable slot.
    let statuses = unsafe {
        [
            xlsx_graft_tree_path(path.as_ptr(), path.len(), ptr::null(), 0, &raw mut out),
            xlsx_graft_describe_sheet_path(path.as_ptr(), path.len(), ptr::null(), 0, &raw mut out),
        ]
    };
    assert_eq!(statuses, [XLSX_GRAFT_ERR_INVALID_ARG; 2]);
    assert_eq!(out, sentinel());
}

#[test]
fn the_result_is_the_same_bytes_every_time_and_a_null_close_does_nothing() {
    let mut out: *mut XlsxGraftView = ptr::null_mut();
    let path = b"/no/such/book.xlsx";
    // SAFETY: `path` is alive for the call, `out` a writable slot.
    let status =
        unsafe { xlsx_graft_tree_path(path.as_ptr(), path.len(), ptr::null(), 0, &raw mut out) };
    assert_eq!(status, XLSX_GRAFT_OK);
    let (first, text) = result(out);
    assert!(text.contains(r#""error":{"code":"io""#), "{text}");
    assert_eq!(result(out).0, first);
    // SAFETY: `out` is a live handle, closed once.
    let closed = unsafe { xlsx_graft_view_close(out) };
    assert_eq!(closed, XLSX_GRAFT_OK);
    // SAFETY: `path` is alive for the call, `out` a writable slot.
    let status = unsafe {
        xlsx_graft_describe_sheet_path(path.as_ptr(), path.len(), ptr::null(), 0, &raw mut out)
    };
    assert_eq!(status, XLSX_GRAFT_OK);
    assert!(result(out).1.contains(r#""error":{"code":"io""#));
    // SAFETY: `out` is a live handle, closed once.
    assert_eq!(unsafe { xlsx_graft_view_close(out) }, XLSX_GRAFT_OK);
    // SAFETY: a null handle is accepted.
    let status = unsafe { xlsx_graft_view_close(ptr::null_mut()) };
    assert_eq!(status, XLSX_GRAFT_OK);
}

#[test]
fn options_that_are_not_a_json_object_of_known_keys_leave_out_view_unwritten() {
    let mut out = sentinel();
    let large = vec![b' '; xlsx_graft_host::MAX_REQUEST_BYTES + 1];
    for options in [&large[..], b"\xff", b"[]"] {
        let path = b"/no/such/book.xlsx";
        // SAFETY: `path` and `options` are alive for the call, `out` a
        // writable slot.
        let statuses = unsafe {
            [
                xlsx_graft_tree_path(
                    path.as_ptr(),
                    path.len(),
                    options.as_ptr(),
                    options.len(),
                    &raw mut out,
                ),
                xlsx_graft_describe_sheet_path(
                    path.as_ptr(),
                    path.len(),
                    options.as_ptr(),
                    options.len(),
                    &raw mut out,
                ),
                xlsx_graft_tree_bytes(
                    ptr::null(),
                    0,
                    options.as_ptr(),
                    options.len(),
                    &raw mut out,
                ),
                xlsx_graft_describe_sheet_bytes(
                    ptr::null(),
                    0,
                    options.as_ptr(),
                    options.len(),
                    &raw mut out,
                ),
            ]
        };
        assert_eq!(statuses, [XLSX_GRAFT_ERR_INVALID_ARG; 4]);
        assert_eq!(out, sentinel());
    }
}

#[test]
fn keys_that_from_arguments_refuses_return_request_invalid_and_leave_out_view_unwritten() {
    let mut out = sentinel();
    let options = br#"{"maxRows":0}"#;
    // SAFETY: `options` is alive for the call, `out` a writable slot.
    let status = unsafe {
        xlsx_graft_describe_sheet_bytes(
            ptr::null(),
            0,
            options.as_ptr(),
            options.len(),
            &raw mut out,
        )
    };
    assert_eq!(status, XLSX_GRAFT_ERR_REQUEST_INVALID);
    assert_eq!(out, sentinel());
}

#[test]
fn a_path_that_is_not_utf8_is_refused_before_the_options_and_keeps_the_earlier_message() {
    crate::message::failed("an earlier failure");
    let mut out = sentinel();
    let path = b"\xff";
    let options = br#"{"maxRows":0}"#;
    // SAFETY: `path` and `options` are alive for the call, `out` a writable
    // slot.
    let status = unsafe {
        xlsx_graft_describe_sheet_path(
            path.as_ptr(),
            path.len(),
            options.as_ptr(),
            options.len(),
            &raw mut out,
        )
    };
    assert_eq!(status, XLSX_GRAFT_ERR_INVALID_ARG);
    assert_eq!(out, sentinel());
    let (mut ptr, mut len) = (ptr::null(), 0);
    // SAFETY: both arguments are writable slots.
    unsafe { crate::abi::xlsx_graft_error_message(&raw mut ptr, &raw mut len) };
    // SAFETY: the library lent `len` bytes, alive until the next failure.
    let message = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert_eq!(message, b"an earlier failure");
}

#[test]
fn a_null_path_of_length_0_is_an_empty_path_that_cannot_be_opened() {
    let mut out: *mut XlsxGraftView = ptr::null_mut();
    // SAFETY: null with a length of 0 is no bytes; `out` a writable slot.
    let status = unsafe { xlsx_graft_tree_path(ptr::null(), 0, ptr::null(), 0, &raw mut out) };
    assert_eq!(status, XLSX_GRAFT_OK);
    assert!(result(out).1.contains(r#""error":{"code":"io""#));
    // SAFETY: `out` is a live handle, closed once.
    assert_eq!(unsafe { xlsx_graft_view_close(out) }, XLSX_GRAFT_OK);
}

#[test]
fn a_path_that_is_not_utf8_with_a_null_out_view_is_null_arg_first() {
    let path = b"\xff";
    // SAFETY: `path` is alive for the call; the null `out_view` is what is
    // tested.
    let status = unsafe {
        xlsx_graft_describe_sheet_path(path.as_ptr(), 1, ptr::null(), 0, ptr::null_mut())
    };
    assert_eq!(status, XLSX_GRAFT_ERR_NULL_ARG);
}

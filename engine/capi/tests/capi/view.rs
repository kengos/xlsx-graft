//! The view handle, driven end to end as a binding drives it: the tree and
//! the cells of a sheet of a workbook from a path and from bytes, the status
//! and message of options that are refused, and many views at once, through
//! the library as its dependents link it.

use std::ptr;
use xlsx_graft_capi::*;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

/// A slot value that is not null and not a handle, to tell a call that wrote
/// `*out_view` from one that did not. Never read or closed.
fn sentinel() -> *mut XlsxGraftView {
    ptr::NonNull::<XlsxGraftView>::dangling().as_ptr()
}

/// Copies out the result of `view` and closes it, as a binding does.
fn finish(status: i32, view: *mut XlsxGraftView) -> String {
    assert_eq!(status, XLSX_GRAFT_OK);
    let (mut ptr, mut len) = (ptr::null(), 0);
    // SAFETY: `view` is a live handle; the out pointers are writable slots.
    let status = unsafe { xlsx_graft_view_result(view, &raw mut ptr, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the library lent `len` UTF-8 bytes, alive until the close.
    let text = String::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec());
    // SAFETY: `view` is a live handle, closed once.
    assert_eq!(unsafe { xlsx_graft_view_close(view) }, XLSX_GRAFT_OK);
    text.unwrap()
}

/// The tree of the workbook at `path`.
fn tree_path(path: &str) -> String {
    let mut view = ptr::null_mut();
    // SAFETY: the path is alive for the call, `view` a writable slot.
    let status =
        unsafe { xlsx_graft_tree_path(path.as_ptr(), path.len(), ptr::null(), 0, &raw mut view) };
    finish(status, view)
}

/// The tree of the workbook in `data`.
fn tree_bytes(data: &[u8]) -> String {
    let mut view = ptr::null_mut();
    // SAFETY: the bytes are alive for the call, `view` a writable slot.
    let status =
        unsafe { xlsx_graft_tree_bytes(data.as_ptr(), data.len(), ptr::null(), 0, &raw mut view) };
    finish(status, view)
}

/// The status of viewing the cells of a sheet of the workbook at `path`
/// under `options`, and the view it wrote.
fn describe_path_status(path: &str, options: &str) -> (i32, *mut XlsxGraftView) {
    let mut view = sentinel();
    // SAFETY: both strings are alive for the call, `view` a writable slot.
    let status = unsafe {
        xlsx_graft_describe_sheet_path(
            path.as_ptr(),
            path.len(),
            options.as_ptr(),
            options.len(),
            &raw mut view,
        )
    };
    (status, view)
}

/// The cells of a sheet of the workbook at `path` under `options`.
fn describe_path(path: &str, options: &str) -> String {
    let (status, view) = describe_path_status(path, options);
    finish(status, view)
}

/// The cells of a sheet of the workbook in `data` under `options`.
fn describe_bytes(data: &[u8], options: &str) -> String {
    let mut view = ptr::null_mut();
    // SAFETY: the bytes and options are alive for the call, `view` a
    // writable slot.
    let status = unsafe {
        xlsx_graft_describe_sheet_bytes(
            data.as_ptr(),
            data.len(),
            options.as_ptr(),
            options.len(),
            &raw mut view,
        )
    };
    finish(status, view)
}

/// This thread's message of the last failure.
fn error_message() -> String {
    let (mut ptr, mut len) = (ptr::null(), 0);
    // SAFETY: the out pointers are writable slots.
    let status = unsafe { xlsx_graft_error_message(&raw mut ptr, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: the library lent `len` UTF-8 bytes, alive until the next failure.
    String::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()).unwrap()
}

#[test]
fn the_tree_is_viewed_from_a_path_and_from_bytes() {
    for name in ["table.xlsx", "external-link.xlsx"] {
        let path = format!("{FIXTURES}/{name}");
        let by_path = tree_path(&path);
        assert!(
            by_path.starts_with(r#"{"ok":true,"error":null,"diagnostics":["#),
            "{by_path}"
        );
        assert_eq!(tree_bytes(&std::fs::read(&path).unwrap()), by_path);
    }
}

#[test]
fn the_cells_of_a_sheet_are_viewed_from_a_path_and_from_bytes() {
    let path = format!("{FIXTURES}/merged.xlsx");
    let data = std::fs::read(&path).unwrap();
    let all = describe_path(&path, "");
    assert!(all.starts_with(r#"{"ok":true,"error":null"#), "{all}");
    assert_eq!(describe_bytes(&data, "{}"), all);
    let window = describe_bytes(&data, r#"{"range":"A1:C3"}"#);
    assert!(window.contains(r#""shown":"A1:C3""#), "{window}");
    let rows = describe_path(&path, r#"{"maxRows":2}"#);
    assert!(rows.contains(r#""shown":"A1:E2""#), "{rows}");
    assert_ne!(rows, all);
    let table = format!("{FIXTURES}/table.xlsx");
    assert_eq!(describe_path(&path, r#"{"sheetIndex":0}"#), all);
    let missing = describe_path(&path, r#"{"sheetIndex":99}"#);
    assert!(
        missing.contains(r#""error":{"code":"sheet_not_found""#),
        "{missing}"
    );
    let nope = describe_path(&table, r#"{"sheet":"Nope"}"#);
    assert!(
        nope.contains(r#""error":{"code":"sheet_not_found""#),
        "{nope}"
    );
}

#[test]
fn keys_that_from_arguments_refuses_return_request_invalid_with_the_reason() {
    let path = format!("{FIXTURES}/merged.xlsx");
    let cases = [
        (r#"{"range":"nope"}"#, "range \"nope\""),
        (r#"{"maxRows":0}"#, "maxRows 0 is not between 1 and 10000"),
        (
            r#"{"sheet":"a","sheetIndex":0}"#,
            "describe_sheet takes sheet or sheetIndex, not both",
        ),
        (
            r#"{"sheet":null}"#,
            "describe_sheet's sheet must be a string",
        ),
    ];
    for (options, reason) in cases {
        let (status, view) = describe_path_status(&path, options);
        assert_eq!(status, XLSX_GRAFT_ERR_REQUEST_INVALID, "{options}");
        assert_eq!(view, sentinel());
        assert!(error_message().starts_with(reason), "{}", error_message());
    }
    for options in ["[]", r#"{"x":1}"#, "{"] {
        let (status, view) = describe_path_status(&path, options);
        assert_eq!(status, XLSX_GRAFT_ERR_INVALID_ARG, "{options}");
        assert_eq!(view, sentinel());
    }
    let mut view = sentinel();
    let options = r#"{"sheet":"a"}"#;
    // SAFETY: both strings are alive for the call, `view` a writable slot.
    let status = unsafe {
        xlsx_graft_tree_path(
            path.as_ptr(),
            path.len(),
            options.as_ptr(),
            options.len(),
            &raw mut view,
        )
    };
    assert_eq!(status, XLSX_GRAFT_ERR_INVALID_ARG);
    assert_eq!(view, sentinel());
}

/// The status of viewing the cells of a sheet of the workbook at the bytes
/// `path` under the bytes `options`, and the slot after the call.
fn describe_raw(path: &[u8], options: &[u8]) -> (i32, *mut XlsxGraftView) {
    let mut view = sentinel();
    // SAFETY: both byte strings are alive for the call, `view` a writable
    // slot.
    let status = unsafe {
        xlsx_graft_describe_sheet_path(
            path.as_ptr(),
            path.len(),
            options.as_ptr(),
            options.len(),
            &raw mut view,
        )
    };
    (status, view)
}

/// Makes a call that returns `XLSX_GRAFT_ERR_REQUEST_INVALID`, so that this
/// thread's message is one the next call must leave in place when it writes
/// none, and returns that message.
fn keep_a_request_invalid_message() -> String {
    let (status, _) = describe_raw(b"/no/such/book.xlsx", br#"{"maxRows":0}"#);
    assert_eq!(status, XLSX_GRAFT_ERR_REQUEST_INVALID);
    error_message()
}

/// Asserts that the call returned `INVALID_ARG`, left the slot unwritten and
/// wrote no message (the one of `keep_a_request_invalid_message` remains).
fn assert_invalid_arg_without_message(path: &[u8], options: &[u8]) {
    let primed = keep_a_request_invalid_message();
    let (status, view) = describe_raw(path, options);
    assert_eq!(status, XLSX_GRAFT_ERR_INVALID_ARG);
    assert_eq!(view, sentinel());
    assert_eq!(error_message(), primed);
}

/// Asserts that the call returned `REQUEST_INVALID`, left the slot unwritten
/// and wrote a message that starts with `reason`.
fn assert_request_invalid(path: &[u8], options: &[u8], reason: &str) {
    let (status, view) = describe_raw(path, options);
    assert_eq!(status, XLSX_GRAFT_ERR_REQUEST_INVALID);
    assert_eq!(view, sentinel());
    assert!(error_message().starts_with(reason), "{}", error_message());
}

#[test]
fn a_path_that_is_not_utf8_wins_over_a_fault_of_a_value_and_leaves_the_earlier_message() {
    assert_invalid_arg_without_message(b"\xff", br#"{"maxRows":0}"#);
}

#[test]
fn options_over_2_mib_with_a_key_the_view_does_not_take_are_invalid_arg() {
    let mut options = br#"{"x":1}"#.to_vec();
    options.resize(xlsx_graft_host::MAX_REQUEST_BYTES, b' ');
    options.push(b' ');
    assert_invalid_arg_without_message(b"/no/such/book.xlsx", &options);
}

#[test]
fn an_unpaired_surrogate_escape_and_a_number_beyond_a_double_are_not_json_so_invalid_arg() {
    for options in [&br#"{"sheet":"\ud800"}"#[..], br#"{"maxRows":1e400}"#] {
        assert_invalid_arg_without_message(b"/no/such/book.xlsx", options);
    }
}

#[test]
fn a_key_the_view_does_not_take_wins_over_a_fault_of_a_value() {
    assert_invalid_arg_without_message(b"/no/such/book.xlsx", br#"{"x":1,"maxRows":0}"#);
}

#[test]
fn a_sheet_of_the_wrong_type_wins_over_a_maxrows_out_of_bounds() {
    assert_request_invalid(
        b"/no/such/book.xlsx",
        br#"{"sheet":1,"maxRows":0}"#,
        "describe_sheet's sheet must be a string",
    );
}

#[test]
fn sheet_with_sheet_index_wins_over_a_range_that_is_not_an_a1_range() {
    assert_request_invalid(
        b"/no/such/book.xlsx",
        br#"{"sheet":"a","sheetIndex":0,"range":"nope"}"#,
        "describe_sheet takes sheet or sheetIndex, not both",
    );
}

#[test]
fn a_maxrows_of_the_wrong_type_wins_over_a_range_that_is_not_an_a1_range() {
    assert_request_invalid(
        b"/no/such/book.xlsx",
        br#"{"range":"nope","maxRows":"x"}"#,
        "describe_sheet's maxRows must be an integer from 1 to 10000",
    );
}

#[test]
fn a_range_that_is_not_an_a1_range_wins_over_a_maxrows_out_of_bounds() {
    assert_request_invalid(
        b"/no/such/book.xlsx",
        br#"{"range":"nope","maxRows":0}"#,
        "range \"nope\"",
    );
}

#[test]
fn a_fault_of_a_value_wins_over_a_file_that_does_not_exist() {
    assert_request_invalid(
        b"/no/such/book.xlsx",
        br#"{"maxRows":0}"#,
        "maxRows 0 is not between 1 and 10000",
    );
}

#[test]
fn a_file_that_does_not_exist_is_ok_with_an_io_error_and_leaves_the_earlier_message() {
    let primed = keep_a_request_invalid_message();
    let (status, view) = describe_raw(b"/no/such/book.xlsx", b"{}");
    assert_eq!(status, XLSX_GRAFT_OK);
    assert!(finish(status, view).contains(r#""error":{"code":"io""#));
    assert_eq!(error_message(), primed);
}

#[test]
fn a_view_is_made_and_read_from_many_threads_at_once() {
    let data = std::fs::read(format!("{FIXTURES}/merged.xlsx")).unwrap();
    let expected = describe_bytes(&data, "{}");
    let options = "{}";
    let mut view = ptr::null_mut();
    // SAFETY: the bytes are alive for the call, `view` a writable slot.
    let status = unsafe {
        xlsx_graft_describe_sheet_bytes(
            data.as_ptr(),
            data.len(),
            options.as_ptr(),
            options.len(),
            &raw mut view,
        )
    };
    assert_eq!(status, XLSX_GRAFT_OK);
    let shared = view as usize;
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                assert_eq!(describe_bytes(&data, "{}"), expected);
                let (mut ptr, mut len) = (ptr::null(), 0);
                // SAFETY: the handle is live until the close below, and does
                // not change; the out pointers are writable slots.
                let status = unsafe {
                    xlsx_graft_view_result(
                        shared as *const XlsxGraftView,
                        &raw mut ptr,
                        &raw mut len,
                    )
                };
                assert_eq!(status, XLSX_GRAFT_OK);
                // SAFETY: on OK the library lent `len` bytes, alive until the close.
                let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
                assert_eq!(bytes, expected.as_bytes());
            });
        }
    });
    // SAFETY: every thread has finished; the handle is closed once.
    assert_eq!(unsafe { xlsx_graft_view_close(view) }, XLSX_GRAFT_OK);
}

#[test]
fn the_handle_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<XlsxGraftView>();
}

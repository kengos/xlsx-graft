//! Viewing a workbook through the C ABI: each call makes a view handle,
//! copies its result out, and closes it before it returns, so no view
//! handle outlives the call (docs/sdk.md § Viewing through the C ABI).

use super::{Outcome, read};
use xlsx_graft_capi::{
    XLSX_GRAFT_OK, XlsxGraftView, xlsx_graft_describe_sheet_bytes, xlsx_graft_describe_sheet_path,
    xlsx_graft_tree_bytes, xlsx_graft_tree_path, xlsx_graft_view_close, xlsx_graft_view_result,
};

/// Shows the parts of the workbook at `path` and their relationships;
/// `options` is `{}` (empty text, text of only white space, and `null` are
/// read as `{}`). The result's JSON is the value.
#[must_use]
pub fn tree_path(path: &str, options: &str) -> Outcome {
    let mut handle = std::ptr::null_mut();
    // SAFETY: both strings are alive for the call, `handle` a writable slot.
    let status = unsafe {
        xlsx_graft_tree_path(
            path.as_ptr(),
            path.len(),
            options.as_ptr(),
            options.len(),
            &raw mut handle,
        )
    };
    // SAFETY: `handle` is what the call wrote, when its status is OK.
    unsafe { finish(status, handle) }
}

/// Shows the parts of the workbook in `data`, which the C ABI reads in
/// place during the call.
#[must_use]
pub fn tree_bytes(data: &[u8], options: &str) -> Outcome {
    let mut handle = std::ptr::null_mut();
    // SAFETY: `data` and `options` are alive and unchanged for the call,
    // `handle` a writable slot.
    let status = unsafe {
        xlsx_graft_tree_bytes(
            data.as_ptr(),
            data.len(),
            options.as_ptr(),
            options.len(),
            &raw mut handle,
        )
    };
    // SAFETY: `handle` is what the call wrote, when its status is OK.
    unsafe { finish(status, handle) }
}

/// Shows the cells of one sheet of the workbook at `path`; `options` is a
/// JSON object of `sheet`, `sheetIndex`, `range`, `maxRows` and
/// `maxColumns`. The result's JSON is the value; keys whose values
/// `SheetRequest::from_arguments` refuses are `XLSX_GRAFT_ERR_REQUEST_INVALID`
/// with the reason as the message.
#[must_use]
pub fn describe_sheet_path(path: &str, options: &str) -> Outcome {
    let mut handle = std::ptr::null_mut();
    // SAFETY: both strings are alive for the call, `handle` a writable slot.
    let status = unsafe {
        xlsx_graft_describe_sheet_path(
            path.as_ptr(),
            path.len(),
            options.as_ptr(),
            options.len(),
            &raw mut handle,
        )
    };
    // SAFETY: `handle` is what the call wrote, when its status is OK.
    unsafe { finish(status, handle) }
}

/// Shows the cells of one sheet of the workbook in `data`, which the C ABI
/// reads in place during the call.
#[must_use]
pub fn describe_sheet_bytes(data: &[u8], options: &str) -> Outcome {
    let mut handle = std::ptr::null_mut();
    // SAFETY: `data` and `options` are alive and unchanged for the call,
    // `handle` a writable slot.
    let status = unsafe {
        xlsx_graft_describe_sheet_bytes(
            data.as_ptr(),
            data.len(),
            options.as_ptr(),
            options.len(),
            &raw mut handle,
        )
    };
    // SAFETY: `handle` is what the call wrote, when its status is OK.
    unsafe { finish(status, handle) }
}

/// The outcome of a call that made `handle`: its result copied out, then
/// the handle closed. On any status but `XLSX_GRAFT_OK` no handle was
/// written, so none is read or closed.
///
/// # Safety
///
/// When `status` is `XLSX_GRAFT_OK`, `handle` must be the live view handle
/// the call wrote.
unsafe fn finish(status: i32, handle: *mut XlsxGraftView) -> Outcome {
    if status != XLSX_GRAFT_OK {
        // SAFETY: `read` does not read `ptr` and `len` when the status is not OK.
        return unsafe { read(status, std::ptr::null(), 0) };
    }
    let (mut ptr, mut len) = (std::ptr::null(), 0);
    // SAFETY: the handle is live; the out pointers are writable slots.
    let lent = unsafe { xlsx_graft_view_result(handle, &raw mut ptr, &raw mut len) };
    // SAFETY: on OK the library lent `len` UTF-8 bytes, alive until the
    // close, which comes after the copy.
    let outcome = unsafe { read(lent, ptr, len) };
    // SAFETY: the handle is live and closed once. Dropping it drops a
    // `String`, which does not panic, so the close returns `XLSX_GRAFT_OK`.
    unsafe { xlsx_graft_view_close(handle) };
    outcome
}

#[cfg(test)]
mod tests;

//! The view handle's entry points: view a workbook's tree or the cells of
//! one of its sheets from a path or bytes, lend the result, close it.
//!
//! A view handle does not change once made, and a view shares nothing with
//! another. So these entry points, unlike the book's, which mark a book as
//! unusable after a panic, do not mark a view handle as unusable, and
//! `xlsx_graft_view_result` may be called on one view handle from several
//! threads at once; only the close must not overlap another call on that
//! handle.

use super::{bytes, missing, refuse_null, run, text};
use crate::view::XlsxGraftView;

/// Writes the handle `view` into `*out`.
///
/// # Safety
///
/// `out` must be non-null and a writable, aligned slot.
unsafe fn lend(out: *mut *mut XlsxGraftView, view: XlsxGraftView) {
    // SAFETY: `out` is non-null, aligned and writable (this function's
    // `# Safety`).
    unsafe { out.write(Box::into_raw(Box::new(view))) };
}

/// Views the tree of the workbook at the UTF-8 path `path` (`path_len`
/// bytes, not NUL-terminated) under `options` (`options_len` bytes of UTF-8
/// JSON; a tree takes no key, so the accepted options are `{}`, and empty
/// text, text of only white space, and `null` are read as `{}`) and writes
/// a view handle holding the result's JSON into `*out_view`.
///
/// Which status each fault of the path and the options returns is the table
/// in the VIEWS section of `include/xlsx_graft.h` (a tree has faults 1 to 6
/// and 14); `*out_view` is written only on [`XLSX_GRAFT_OK`](crate::XLSX_GRAFT_OK).
///
/// # Safety
///
/// `path` and `options` must each be null or point at that many readable
/// bytes (a null one is no bytes when its length is 0, and refused
/// otherwise), and `out_view` be null or point at one writable, aligned
/// slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_tree_path(
    path: *const u8,
    path_len: usize,
    options: *const u8,
    options_len: usize,
    out_view: *mut *mut XlsxGraftView,
) -> i32 {
    run(|| {
        refuse_null(
            missing(path, path_len) || missing(options, options_len) || out_view.is_null(),
        )?;
        // SAFETY: neither `path` nor `options` is missing, and the caller's contract
        // gives each its bytes for the call.
        let (path, options) = unsafe { (text(path, path_len)?, bytes(options, options_len)) };
        let view = XlsxGraftView::tree_path(path, options)?;
        // SAFETY: `out_view` is non-null, and a writable slot by contract.
        unsafe { lend(out_view, view) };
        Ok(())
    })
}

/// Views the tree of the workbook in the `len` bytes at `data` under
/// `options`, as [`xlsx_graft_tree_path`] does a file.
///
/// The bytes are not copied: they are read during the call only, and may be
/// freed once it returns.
///
/// # Safety
///
/// `data` and `options` must each be null or point at that many readable
/// bytes (a null one is no bytes when its length is 0, and refused
/// otherwise) that stay alive and unchanged until the call returns, and
/// `out_view` be null or point at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_tree_bytes(
    data: *const u8,
    len: usize,
    options: *const u8,
    options_len: usize,
    out_view: *mut *mut XlsxGraftView,
) -> i32 {
    run(|| {
        refuse_null(missing(data, len) || missing(options, options_len) || out_view.is_null())?;
        // SAFETY: neither `data` nor `options` is missing, and the caller's contract
        // gives each its bytes for the call.
        let (data, options) = unsafe { (bytes(data, len), bytes(options, options_len)) };
        let view = XlsxGraftView::tree_bytes(data, options)?;
        // SAFETY: `out_view` is non-null, and a writable slot by contract.
        unsafe { lend(out_view, view) };
        Ok(())
    })
}

/// Views the cells of one sheet of the workbook at the UTF-8 path `path`
/// (`path_len` bytes, not NUL-terminated) under `options` (`options_len`
/// bytes of UTF-8 JSON: an object of `sheet`, `sheetIndex`, `range`,
/// `maxRows` and `maxColumns`, each key optional; empty text, text of only
/// white space and `null` are read as `{}`) and writes
/// a view handle holding the result's JSON into `*out_view`.
///
/// Which status each fault of the path and the options returns is the table
/// in the VIEWS section of `include/xlsx_graft.h`; `*out_view` is written
/// only on [`XLSX_GRAFT_OK`](crate::XLSX_GRAFT_OK).
///
/// # Safety
///
/// `path` and `options` must each be null or point at that many readable
/// bytes (a null one is no bytes when its length is 0, and refused
/// otherwise), and `out_view` be null or point at one writable, aligned
/// slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_describe_sheet_path(
    path: *const u8,
    path_len: usize,
    options: *const u8,
    options_len: usize,
    out_view: *mut *mut XlsxGraftView,
) -> i32 {
    run(|| {
        refuse_null(
            missing(path, path_len) || missing(options, options_len) || out_view.is_null(),
        )?;
        // SAFETY: neither `path` nor `options` is missing, and the caller's contract
        // gives each its bytes for the call.
        let (path, options) = unsafe { (text(path, path_len)?, bytes(options, options_len)) };
        let view = XlsxGraftView::describe_sheet_path(path, options)?;
        // SAFETY: `out_view` is non-null, and a writable slot by contract.
        unsafe { lend(out_view, view) };
        Ok(())
    })
}

/// Views the cells of one sheet of the workbook in the `len` bytes at
/// `data` under `options`, as [`xlsx_graft_describe_sheet_path`] does a
/// file.
///
/// The bytes are not copied: they are read during the call only, and may be
/// freed once it returns.
///
/// # Safety
///
/// `data` and `options` must each be null or point at that many readable
/// bytes (a null one is no bytes when its length is 0, and refused
/// otherwise) that stay alive and unchanged until the call returns, and
/// `out_view` be null or point at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_describe_sheet_bytes(
    data: *const u8,
    len: usize,
    options: *const u8,
    options_len: usize,
    out_view: *mut *mut XlsxGraftView,
) -> i32 {
    run(|| {
        refuse_null(missing(data, len) || missing(options, options_len) || out_view.is_null())?;
        // SAFETY: neither `data` nor `options` is missing, and the caller's contract
        // gives each its bytes for the call.
        let (data, options) = unsafe { (bytes(data, len), bytes(options, options_len)) };
        let view = XlsxGraftView::describe_sheet_bytes(data, options)?;
        // SAFETY: `out_view` is non-null, and a writable slot by contract.
        unsafe { lend(out_view, view) };
        Ok(())
    })
}

/// Writes the result as UTF-8 JSON: a pointer into `*out_ptr` and the
/// length into `*out_len`. The bytes are not NUL-terminated, belong to the
/// handle, and live until its close; every call returns the same bytes.
///
/// # Safety
///
/// `view` must be null or a view handle not yet closed, and each out
/// pointer null or at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_view_result(
    view: *const XlsxGraftView,
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    run(|| {
        refuse_null(view.is_null() || out_ptr.is_null() || out_len.is_null())?;
        // SAFETY: `view` is non-null, and a live handle by contract.
        let json = unsafe { &*view }.json();
        // SAFETY: the out pointers are non-null, and writable by contract.
        unsafe {
            out_ptr.write(json.as_ptr());
            out_len.write(json.len());
        }
        Ok(())
    })
}

/// Closes the view handle; what it lent is gone. A null `view` does
/// nothing; either way the call returns [`XLSX_GRAFT_OK`](crate::XLSX_GRAFT_OK)
/// (or [`XLSX_GRAFT_ERR_PANIC`](crate::XLSX_GRAFT_ERR_PANIC)).
///
/// # Safety
///
/// `view` must be null or a view handle not yet closed. It is closed after
/// the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_view_close(view: *mut XlsxGraftView) -> i32 {
    run(|| {
        if !view.is_null() {
            // SAFETY: a live handle is a `Box` the library leaked, and the
            // caller gives it back only once.
            drop(unsafe { Box::from_raw(view) });
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests;

//! xlsx-graft C ABI: the shared library the FFI SDKs load.
//!
//! A thin host over the engine, like the CLI and the MCP server. It exports
//! [`xlsx_graft_version`] and the handles that read a workbook's rows: a
//! book ([`XlsxGraftBook`]) and the rows of one of its sheets
//! ([`XlsxGraftRows`]), whose entry points are in `abi/`. The logic of the
//! handles is safe Rust in `book.rs` and `rows.rs`; `abi/` only marshals.
//!
//! The rules of the surface are the caller's contract as much as ours, so
//! `include/xlsx_graft.h` states each one again for the C side:
//!
//! * **Nothing is NUL-terminated.** A string crosses as a pointer plus a
//!   length.
//! * **What the library returns, the library owns.** The version string is
//!   static and is never freed; what a handle lends lives as the header
//!   says, and a handle lives until its close.
//! * **A failure is a status, never an unwind.** Every entry point runs its
//!   body inside the crate's panic guard (`guard.rs`), so a panic becomes
//!   [`XLSX_GRAFT_ERR_PANIC`].

// A crate of the workspace that writes `unsafe` (docs/guidelines.md
// § unsafe), so the discipline is declared rather than assumed: an `unsafe fn`
// gets no implicit unsafe body, and every unsafe block states why it is sound.
#![allow(
    unsafe_code,
    reason = "the C ABI takes raw pointers from the caller and writes through them"
)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod abi;
mod book;
mod guard;
mod message;
mod rows;
mod sheet;
mod status;
mod value;

pub use abi::book::{
    xlsx_graft_book_close, xlsx_graft_book_diagnostics, xlsx_graft_book_open_bytes,
    xlsx_graft_book_open_path, xlsx_graft_book_sheets,
};
pub use abi::rows::{
    xlsx_graft_rows_close, xlsx_graft_rows_next, xlsx_graft_rows_open, xlsx_graft_rows_open_name,
};
pub use abi::xlsx_graft_error_message;
pub use book::XlsxGraftBook;
pub use rows::XlsxGraftRows;
pub use sheet::{
    XLSX_GRAFT_SHEET_CHARTSHEET, XLSX_GRAFT_SHEET_DIALOGSHEET, XLSX_GRAFT_SHEET_MACROSHEET,
    XLSX_GRAFT_SHEET_UNKNOWN, XLSX_GRAFT_SHEET_WORKSHEET, XLSX_GRAFT_STATE_HIDDEN,
    XLSX_GRAFT_STATE_VERY_HIDDEN, XLSX_GRAFT_STATE_VISIBLE, XlsxGraftSheet,
};
pub use status::{
    XLSX_GRAFT_ERR_INVALID_ARG, XLSX_GRAFT_ERR_IO, XLSX_GRAFT_ERR_NULL_ARG, XLSX_GRAFT_ERR_PANIC,
    XLSX_GRAFT_ERR_SHEET_AMBIGUOUS, XLSX_GRAFT_ERR_SHEET_NOT_FOUND,
    XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET, XLSX_GRAFT_ERR_SHEET_UNREADABLE,
    XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE, XLSX_GRAFT_OK,
};
pub use value::{
    XLSX_GRAFT_VALUE_BOOLEAN, XLSX_GRAFT_VALUE_DATE, XLSX_GRAFT_VALUE_DATETIME,
    XLSX_GRAFT_VALUE_ELAPSED, XLSX_GRAFT_VALUE_EMPTY, XLSX_GRAFT_VALUE_ERROR,
    XLSX_GRAFT_VALUE_NUMBER, XLSX_GRAFT_VALUE_STRING, XLSX_GRAFT_VALUE_TIME, XlsxGraftValue,
};

/// Writes the engine version: a pointer to its UTF-8 bytes into `*out_ptr`
/// and their length into `*out_len`.
///
/// The bytes are not NUL-terminated, are static, and must not be freed.
/// Returns [`XLSX_GRAFT_OK`], [`XLSX_GRAFT_ERR_NULL_ARG`] (writing nothing)
/// when either pointer is null, or [`XLSX_GRAFT_ERR_PANIC`] as every entry
/// point can.
///
/// # Safety
///
/// Each of `out_ptr` and `out_len` must be null or point at one writable,
/// aligned slot of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_version(out_ptr: *mut *const u8, out_len: *mut usize) -> i32 {
    guard::guard(|| {
        if out_ptr.is_null() || out_len.is_null() {
            return XLSX_GRAFT_ERR_NULL_ARG;
        }
        let version = xlsx_graft_core::version();
        // SAFETY: both pointers were checked non-null above, and the caller's
        // contract (# Safety) makes each one a writable, aligned slot.
        unsafe {
            out_ptr.write(version.as_ptr());
            out_len.write(version.len());
        }
        XLSX_GRAFT_OK
    })
}

#[cfg(test)]
mod tests;

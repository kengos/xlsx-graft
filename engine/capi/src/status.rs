//! The status codes every entry point returns.
//!
//! The values are part of the ABI: `include/xlsx_graft.h` defines the same
//! numbers, and a binding branches on them. A new code is appended; an
//! existing one never changes its value.

/// The call completed and wrote its out-parameters.
pub const XLSX_GRAFT_OK: i32 = 0;
/// A pointer argument that must not be null was null. Nothing was written.
pub const XLSX_GRAFT_ERR_NULL_ARG: i32 = 1;
/// A panic was caught at the boundary. The library is still usable; the call
/// is not, and its out-parameters may have been partly written.
pub const XLSX_GRAFT_ERR_PANIC: i32 = 2;
/// The host failed, not the workbook: a file could not be opened, read or
/// written. `xlsx_graft_error_message` says why.
pub const XLSX_GRAFT_ERR_IO: i32 = 3;
/// A string argument is not UTF-8.
pub const XLSX_GRAFT_ERR_INVALID_ARG: i32 = 4;
/// The package or its workbook part cannot be read; the book's diagnostics
/// say why.
pub const XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE: i32 = 5;
/// No sheet has the name, or none is at the index.
pub const XLSX_GRAFT_ERR_SHEET_NOT_FOUND: i32 = 6;
/// More than one sheet has the name, letter case aside.
pub const XLSX_GRAFT_ERR_SHEET_AMBIGUOUS: i32 = 7;
/// The sheet is a chartsheet, a dialog sheet or a macro sheet, which have no
/// rows to read.
pub const XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET: i32 = 8;
/// The sheet's part is not known or cannot be read; the book's diagnostics
/// say why.
pub const XLSX_GRAFT_ERR_SHEET_UNREADABLE: i32 = 9;
/// The rows handle's book was closed, which closed its rows: the handle
/// reads no more, and only its close is left to call.
pub const XLSX_GRAFT_ERR_CLOSED: i32 = 10;

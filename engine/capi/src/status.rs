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

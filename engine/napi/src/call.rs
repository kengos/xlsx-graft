//! The one place this crate crosses into the C ABI.
//!
//! The C entry points lend pointers into memory the library owns; this module
//! copies what they lend into an owned [`Outcome`], so nothing borrowed from
//! the library reaches JavaScript.

use xlsx_graft_capi::{XLSX_GRAFT_OK, xlsx_graft_version};

/// What one call produced, owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The C ABI's status code, passed through unchanged: the JS SDK maps it
    /// to a diagnostic as the other SDKs do.
    pub status: i32,
    /// The call's value, present only when `status` is `XLSX_GRAFT_OK`.
    pub value: Option<String>,
}

/// The engine version, such as `"0.0.0"`.
#[must_use]
pub fn version() -> Outcome {
    let mut ptr = std::ptr::null();
    let mut len = 0;
    // SAFETY: both arguments are writable, aligned slots of the declared
    // types, which is the entry point's whole contract.
    let status = unsafe { xlsx_graft_version(&raw mut ptr, &raw mut len) };
    // SAFETY: on XLSX_GRAFT_OK the library wrote a pointer to `len` static
    // UTF-8 bytes; on any other status `read` looks at neither.
    unsafe { read(status, ptr, len) }
}

/// Copies a lent string out, when the status says one was written.
///
/// The C ABI says to read the out-parameters only on `XLSX_GRAFT_OK`, so any
/// other status, including one this crate does not know yet, comes back
/// without a value.
///
/// # Safety
///
/// When `status` is `XLSX_GRAFT_OK`, `ptr` must be non-null and point at
/// `len` readable bytes (`len` at most `isize::MAX`) that stay alive for the
/// call.
unsafe fn read(status: i32, ptr: *const u8, len: usize) -> Outcome {
    if status != XLSX_GRAFT_OK {
        return Outcome {
            status,
            value: None,
        };
    }
    // SAFETY: the caller's contract for XLSX_GRAFT_OK.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    // The library documents its strings as UTF-8, so a replacement decode
    // only matters if the engine has a bug, and then it keeps the status.
    let value = String::from_utf8_lossy(bytes).into_owned();
    Outcome {
        status,
        value: Some(value),
    }
}

#[cfg(test)]
mod tests;

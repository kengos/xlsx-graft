//! xlsx-graft C ABI: the shared library the FFI SDKs load.
//!
//! A thin host over [`xlsx_graft_core`], like the CLI and the MCP server. It
//! marshals and nothing else. For now it exports one function,
//! [`xlsx_graft_version`].
//!
//! The rules of the surface are the caller's contract as much as ours, so
//! `include/xlsx_graft.h` states each one again for the C side:
//!
//! * **Nothing is NUL-terminated.** A string crosses as a pointer plus a
//!   length.
//! * **What the library returns, the library owns.** The version string is
//!   static and is never freed.
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

mod guard;
mod status;

pub use status::{XLSX_GRAFT_ERR_NULL_ARG, XLSX_GRAFT_ERR_PANIC, XLSX_GRAFT_OK};

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

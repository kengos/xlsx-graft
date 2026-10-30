//! The entry points of the book and rows handles, and what they share:
//! taking the caller's pointers, and turning a body's `Result` into a
//! status inside the panic guard. Each entry point only marshals; what it
//! decides is in `book.rs` and `rows.rs`.
//!
//! A handle is a `Box` lent to the caller as a raw pointer, and taken back
//! by its close. A null handle is refused with
//! [`XLSX_GRAFT_ERR_NULL_ARG`], and closing a null handle does nothing, so
//! a binding that nulls its copy on close cannot use or close it twice.

pub mod book;
pub mod rows;

use crate::guard::guard;
use crate::message::lent;
use crate::shared::Shared;
use crate::status::{
    XLSX_GRAFT_ERR_INVALID_ARG, XLSX_GRAFT_ERR_NULL_ARG, XLSX_GRAFT_ERR_PANIC, XLSX_GRAFT_OK,
};
use std::panic::AssertUnwindSafe;
use std::rc::Rc;

/// Runs `body` inside the panic guard: its error is the status, and
/// `Ok` is [`XLSX_GRAFT_OK`].
///
/// A body takes handles, which are not unwind safe: after a panic a handle
/// may be half-changed. A call on a handle runs through [`run_on`], which
/// then refuses every later call on the book and its rows but their
/// closes, which is what makes the assertion hold.
fn run(body: impl FnOnce() -> Result<(), i32>) -> i32 {
    guard(AssertUnwindSafe(|| body().err().unwrap_or(XLSX_GRAFT_OK)))
}

/// As [`run`], for a call on a handle whose book and rows share `shared`
/// (`None` for a null handle, which `body` refuses): once a call on them
/// has panicked, every later one returns [`XLSX_GRAFT_ERR_PANIC`] without
/// touching them, and a call that panics poisons them so.
fn run_on(shared: Option<Rc<Shared>>, body: impl FnOnce() -> Result<(), i32>) -> i32 {
    if shared.as_deref().is_some_and(Shared::is_poisoned) {
        return XLSX_GRAFT_ERR_PANIC;
    }
    poisoning(shared, run(body))
}

/// `status`, poisoning `shared` when it is [`XLSX_GRAFT_ERR_PANIC`]: a
/// close that panicked part way may leave the other handles of the book
/// half-changed, so it poisons them too, though it is not refused.
fn poisoning(shared: Option<Rc<Shared>>, status: i32) -> i32 {
    if status == XLSX_GRAFT_ERR_PANIC
        && let Some(shared) = shared
    {
        shared.poison();
    }
    status
}

/// Refuses the call with [`XLSX_GRAFT_ERR_NULL_ARG`] when `null` holds:
/// one of its pointers that must not be null is.
fn refuse_null(null: bool) -> Result<(), i32> {
    if null {
        return Err(XLSX_GRAFT_ERR_NULL_ARG);
    }
    Ok(())
}

/// Whether the input `ptr` of `len` bytes is missing: null, though it has
/// bytes. A null pointer of no bytes is an empty input, as the empty
/// arrays some FFIs pass as null.
fn missing(ptr: *const u8, len: usize) -> bool {
    ptr.is_null() && len != 0
}

/// The `len` bytes at `ptr`; none when `len` is 0, whatever `ptr` is.
///
/// # Safety
///
/// When `len` is not 0, `ptr` must be non-null and point at `len` readable
/// bytes (`len` at most `isize::MAX`) that stay alive and unchanged for
/// `'a`.
unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        return &[];
    }
    // SAFETY: the caller's contract.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// The `len` bytes at `ptr` as UTF-8, or [`XLSX_GRAFT_ERR_INVALID_ARG`].
///
/// # Safety
///
/// As for [`bytes`].
unsafe fn text<'a>(ptr: *const u8, len: usize) -> Result<&'a str, i32> {
    // SAFETY: the caller's contract.
    let bytes = unsafe { bytes(ptr, len) };
    std::str::from_utf8(bytes).map_err(|_| XLSX_GRAFT_ERR_INVALID_ARG)
}

/// Writes why the last call on this thread that returned
/// [`XLSX_GRAFT_ERR_IO`](crate::XLSX_GRAFT_ERR_IO) failed: a pointer to its
/// UTF-8 bytes into `*out_ptr` and their length into `*out_len`, no bytes
/// before any such call.
///
/// The bytes are not NUL-terminated, belong to the library, and stay alive
/// until the next call on this thread that fails so, or the thread's end.
/// Returns [`XLSX_GRAFT_OK`], or [`XLSX_GRAFT_ERR_NULL_ARG`] (writing
/// nothing) when either pointer is null.
///
/// # Safety
///
/// Each of `out_ptr` and `out_len` must be null or point at one writable,
/// aligned slot of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_error_message(
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    run(|| {
        refuse_null(out_ptr.is_null() || out_len.is_null())?;
        let (ptr, len) = lent();
        // SAFETY: both pointers were checked non-null above, and the
        // caller's contract makes each one a writable, aligned slot.
        unsafe {
            out_ptr.write(ptr);
            out_len.write(len);
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests;

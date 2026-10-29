//! Why the host failed: the message behind [`XLSX_GRAFT_ERR_IO`], kept per
//! thread until the next failure on the same thread replaces it.
//!
//! A failure to open a file has no handle to carry its reason, so the reason
//! waits here, as `errno` does; `xlsx_graft_error_message` lends it out.

use crate::status::XLSX_GRAFT_ERR_IO;
use std::cell::RefCell;
use std::fmt::Display;

thread_local! {
    /// The message of the last failure on this thread; empty before one.
    static MESSAGE: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Keeps `error` as this thread's message, and returns
/// [`XLSX_GRAFT_ERR_IO`].
pub(crate) fn failed(error: impl Display) -> i32 {
    let text = error.to_string();
    MESSAGE.with_borrow_mut(|message| *message = text);
    XLSX_GRAFT_ERR_IO
}

/// This thread's message: a pointer to its UTF-8 bytes and their length,
/// alive until the next failure on this thread, or its end.
pub(crate) fn lent() -> (*const u8, usize) {
    MESSAGE.with_borrow(|message| (message.as_ptr(), message.len()))
}

#[cfg(test)]
mod tests;

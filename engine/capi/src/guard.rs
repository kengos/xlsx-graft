//! The panic guard every entry point runs its body inside.
//!
//! A panic that reaches the edge of an `extern "C"` function aborts the
//! process (it was undefined behaviour before Rust 1.81), taking the host
//! down with it. So it stops here and becomes [`XLSX_GRAFT_ERR_PANIC`]. That
//! is also why no profile building this crate may set `panic = "abort"`: the
//! guard would never get the chance to catch anything.

use crate::status::XLSX_GRAFT_ERR_PANIC;
use std::panic::{UnwindSafe, catch_unwind};

/// Runs `body` and returns its status, or [`XLSX_GRAFT_ERR_PANIC`] if it
/// panicked.
///
/// `body` must be [`UnwindSafe`]: a body that needs to cross a `&mut` has to
/// say so with `AssertUnwindSafe` at its own call site.
pub(crate) fn guard(body: impl FnOnce() -> i32 + UnwindSafe) -> i32 {
    catch_unwind(body).unwrap_or(XLSX_GRAFT_ERR_PANIC)
}

#[cfg(test)]
mod tests;

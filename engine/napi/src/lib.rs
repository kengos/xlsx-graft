//! xlsx-graft Node.js addon: the N-API host the JS SDK loads.
//!
//! Node.js has no FFI on the lines the JS SDK supports, so it cannot load the
//! C ABI library the way the other SDKs do; it needs a native addon. This
//! crate links `xlsx-graft-capi` as an rlib and calls its entry points
//! rather than the engine beside them, so the status codes reaching
//! JavaScript are the ones every other SDK reads, and the addon carries the
//! whole engine in one file that cannot drift from it.
//!
//! Two parts, split by what the gates can see:
//!
//! * `call` crosses into the C ABI and copies what it lends into owned
//!   values: an [`Outcome`], and the [`Book`] and [`Rows`] handles with the
//!   rows they read. It builds without features, so the test and coverage
//!   gates cover it like any other module.
//! * `shim` (the `shim` feature) is the N-API surface. It only maps those
//!   values to JavaScript objects and classes, so that it can stay out of
//!   the coverage gate: the JS SDK's tests run it through the real addon.
//!
//! Linking the C ABI crate also carries its `xlsx_graft_*` exports into the
//! addon. They are harmless there: Node.js looks only for the N-API
//! registration, and the JS SDK calls nothing else.

// The C ABI is `unsafe` to call (docs/guidelines.md § unsafe), so the
// discipline is declared as in engine/capi: an `unsafe fn` gets no implicit
// unsafe body, and every unsafe block states why it is sound.
#![allow(
    unsafe_code,
    reason = "the addon calls the C ABI's entry points, which take raw pointers"
)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod call;

// The C ABI's unit tests' workbooks, the same file, for this crate's own.
#[cfg(test)]
#[allow(dead_code, reason = "the addon's tests use only some of the builders")]
#[path = "../../capi/src/tests/workbook.rs"]
mod workbook;

#[cfg(feature = "shim")]
pub mod shim;

// Public so the calls stay reachable in a build without the shim, which is
// how the test and coverage gates build this crate.
pub use call::{Book, Cell, Next, Outcome, Row, Rows, Sheet, Sheets, version};

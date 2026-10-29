//! End-to-end tests of the C ABI, driven the way a binding drives it: through
//! the crate's public `extern "C"` functions and the header, never its
//! internals. This binary links the rlib, so whether the shared library
//! exports those functions is `make engine:capi-abi`'s job.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]
#![allow(
    unsafe_code,
    reason = "calling the exported functions is calling `unsafe extern \"C\" fn`s"
)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod handles;
mod header;
mod layout;
mod version;
// The unit tests' workbooks, the same file: this binary links the library
// without its `cfg(test)` code.
#[allow(dead_code, reason = "each binary uses only some of the builders")]
#[path = "../../src/tests/workbook.rs"]
mod workbook;

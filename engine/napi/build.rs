//! Build script: the N-API link setup, and only when the shim is built.
//!
//! With the `shim` feature off, which is how the test and coverage gates
//! build this crate, the body is empty and `napi-build` is not a dependency.

fn main() {
    #[cfg(feature = "shim")]
    napi_build::setup();
}

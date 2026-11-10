//! The verify functions of the N-API surface: each returns the result's
//! JSON in `value`, which the JS SDK parses. Each makes and closes its
//! verify handle inside the call (`call/verify.rs`).

use super::JsOutcome;
use crate::call;
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

/// Verifies the workbook at `path`; `options` is
/// `{"policy": ..., "rules": ...}` as JSON. A path with a lone surrogate is
/// read with U+FFFD in its place, so it names another file.
#[napi(catch_unwind)]
pub fn verify_path(path: String, options: String) -> JsOutcome {
    call::verify_path(&path, &options).into()
}

/// Verifies the workbook in `data`, read in place: JavaScript waits on this
/// call, so the buffer does not change during it. A buffer over shared memory
/// (`SharedArrayBuffer`) must not be written by another thread during the
/// call.
#[napi(catch_unwind)]
pub fn verify_bytes(data: Buffer, options: String) -> JsOutcome {
    call::verify_bytes(&data, &options).into()
}

/// Checks the policy `policy`, a JSON object, alone.
#[napi(catch_unwind)]
pub fn check_policy(policy: String) -> JsOutcome {
    call::check_policy(&policy).into()
}

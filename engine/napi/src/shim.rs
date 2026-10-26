//! The N-API surface: the only part of this crate JavaScript sees.
//!
//! Compiled only with the `shim` feature, so the coverage gate never builds
//! it; the JS SDK's tests run it through the real addon instead. It must
//! stay a mapping from [`Outcome`] to a JavaScript object: a rule that lives
//! here is a rule no Rust test covers.

use crate::call::{self, Outcome};
use napi_derive::napi;

/// One call's result, as the JS SDK receives it.
#[napi(object)]
pub struct JsOutcome {
    /// The C ABI's status code.
    pub status: i32,
    /// The call's value, present only when `status` is `XLSX_GRAFT_OK`.
    pub value: Option<String>,
}

impl From<Outcome> for JsOutcome {
    fn from(outcome: Outcome) -> Self {
        JsOutcome {
            status: outcome.status,
            value: outcome.value,
        }
    }
}

/// The engine version. A panic in the addon becomes a JavaScript exception
/// rather than an abort.
#[napi(catch_unwind)]
pub fn version() -> JsOutcome {
    call::version().into()
}

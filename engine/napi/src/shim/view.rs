//! The view functions of the N-API surface: each returns the result's JSON
//! in `value`, which the JS SDK parses. Each makes and closes its view
//! handle inside the call (`call/view.rs`).

use super::JsOutcome;
use crate::call;
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

/// Shows the parts of the workbook at `path` and their relationships;
/// `options` is `{}` (empty text, text of only white space, and `null` are
/// read as `{}`). A path with a lone surrogate is
/// read with U+FFFD in its place, so it names another file.
#[napi(catch_unwind)]
pub fn tree_path(path: String, options: String) -> JsOutcome {
    call::tree_path(&path, &options).into()
}

/// Shows the parts of the workbook in `data`, read in place: JavaScript
/// waits on this call, so the buffer does not change during it. A buffer over
/// shared memory (`SharedArrayBuffer`) must not be written by another thread
/// during the call.
#[napi(catch_unwind)]
pub fn tree_bytes(data: Buffer, options: String) -> JsOutcome {
    call::tree_bytes(&data, &options).into()
}

/// Shows the cells of one sheet of the workbook at `path`; `options` is a
/// JSON object of `sheet`, `sheetIndex`, `range`, `maxRows` and
/// `maxColumns`. A path with a lone surrogate is read with U+FFFD in its
/// place, so it names another file.
#[napi(catch_unwind)]
pub fn describe_sheet_path(path: String, options: String) -> JsOutcome {
    call::describe_sheet_path(&path, &options).into()
}

/// Shows the cells of one sheet of the workbook in `data`, read in place, as
/// [`tree_bytes`] reads it.
#[napi(catch_unwind)]
pub fn describe_sheet_bytes(data: Buffer, options: String) -> JsOutcome {
    call::describe_sheet_bytes(&data, &options).into()
}

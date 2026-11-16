//! [`View`]: the result of a view of a workbook, and the JSON every host
//! prints it as.
//!
//! ```json
//! {
//!   "ok": true,
//!   "error": null,
//!   "diagnostics": [],
//!   "complete": true
//! }
//! ```
//!
//! (`complete` is a key of a tree's content; another view has its own.)
//!
//! `ok`, `error` and `diagnostics` are the same for every view, as
//! [`Report`](crate::Report) has them; the keys after them are the view's
//! own (for a tree, see [`Tree`](crate::Tree)). The keys and their types are
//! the contract; a key may be added, never renamed or retyped.
//!
//! A view has no `failOn` and no `counts`. Its `ok` is true when it has no
//! `error` and no diagnostic that is an error, including one that was not
//! listed because it was past the limit of diagnostics; a warning or a
//! notice does not change it. A [`Report`](crate::Report)'s `ok` is decided
//! by its `failOn`.

use crate::report::Failure;
use serde::Serialize;
use xlsx_graft_diagnostics::{Diagnostics, Record};

/// The result of a view of a workbook: `{ok, error, diagnostics}` and then
/// the keys of `content`, which has no key named `ok`, `error` or
/// `diagnostics`.
#[derive(Debug, Serialize)]
pub struct View<T> {
    /// No `error`, and no diagnostic that is an error (an error diagnostic
    /// not listed because the limit of diagnostics was reached also makes it
    /// false).
    pub ok: bool,
    /// Why the engine could not look at the workbook, when it could not.
    pub error: Option<Failure>,
    /// Every problem found, in the order found.
    pub diagnostics: Vec<Record>,
    /// What the view shows, written beside the keys above.
    #[serde(flatten)]
    pub content: T,
}

impl<T> View<T> {
    /// The view of a run that gave `content` and `diagnostics`, stopped by
    /// `error` when it was.
    pub(crate) fn finished(content: T, diagnostics: Diagnostics, error: Option<Failure>) -> Self {
        Self {
            ok: !diagnostics.has_errors() && error.is_none(),
            error,
            diagnostics: diagnostics.finish(),
            content,
        }
    }
}

impl<T: Default> View<T> {
    /// The view of a request the host could not start, as a file it had to
    /// read (the workbook, standard input) could not be read: not `ok`, no
    /// diagnostic, `error` an `io` failure saying `message`, and the default
    /// content.
    #[must_use]
    pub fn io_failure(message: String) -> Self {
        Self::finished(T::default(), Diagnostics::new(), Some(Failure::io(message)))
    }
}

impl<T: Serialize> View<T> {
    /// The view as the JSON every host prints, in one line.
    ///
    /// # Errors
    ///
    /// An error does not happen when `T` is written as a map (a
    /// [`Tree`](crate::Tree) is): every field serializes. `serde_json`'s own
    /// error is returned rather than hidden.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests;

//! [`ReferenceError`]: why a URI names no reference page.

use thiserror::Error;

/// A reference URI that names no page.
///
/// The error does not repeat the URI: the caller has it, and a URI from a
/// client is untrusted input of any length.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReferenceError {
    /// The URI does not start with [`URI_PREFIX`](crate::URI_PREFIX), or
    /// the page name after the prefix is not the name of an embedded page.
    #[error("no reference page has this URI; the page URIs are xlsx-graft://reference/<page>")]
    UnknownUri,
}

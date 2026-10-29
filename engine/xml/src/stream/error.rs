//! [`XmlStreamError`]: what reading a streamed part can end in.

use crate::malformed;
use std::io;
use thiserror::Error;
use xlsx_graft_diagnostics::Diagnostic;

/// Why an [`XmlStreamReader`](crate::XmlStreamReader) stopped: a problem in
/// the part, or a failure of the reader it reads from.
#[derive(Debug, Error)]
pub enum XmlStreamError {
    /// The part's XML is refused, with the diagnostic for the caller to
    /// push with the part it read.
    #[error("the XML is refused: {0:?}")]
    Invalid(Diagnostic),
    /// The reader the part is read from failed, with the kind and message
    /// of its error. A zip entry that stops on a problem of its own fails
    /// this way too, having reported it already;
    /// [`XmlStreamReader::get_ref`](crate::XmlStreamReader::get_ref) reaches
    /// it to ask.
    #[error("reading the XML failed: {0}")]
    Io(#[source] io::Error),
}

impl From<Diagnostic> for XmlStreamError {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Invalid(diagnostic)
    }
}

impl XmlStreamError {
    /// The error quick-xml's I/O error `error` stands for: the problem
    /// the source stopped at, `failed`, or else a failure of the reader
    /// underneath.
    pub(super) fn from_source(failed: Option<&Diagnostic>, error: &io::Error) -> Self {
        match failed {
            Some(diagnostic) => Self::Invalid(diagnostic.clone()),
            None => Self::io(error),
        }
    }

    /// The diagnostic for this error from a reader over a slice, which
    /// cannot fail: an I/O error, should one come, is `xml_malformed` at
    /// `offset` with its message.
    pub(crate) fn into_diagnostic(self, offset: u64) -> Diagnostic {
        match self {
            Self::Invalid(diagnostic) => diagnostic,
            Self::Io(error) => malformed(offset, &error.to_string()),
        }
    }

    /// [`XmlStreamError::Io`] with the kind and message of `error`, which
    /// quick-xml shares behind an `Arc`.
    pub(super) fn io(error: &io::Error) -> Self {
        Self::Io(io::Error::new(error.kind(), error.to_string()))
    }

    /// The same error again, for a reader that has stopped: an I/O error
    /// keeps its kind and message.
    pub(super) fn again(&self) -> Self {
        match self {
            Self::Invalid(diagnostic) => Self::Invalid(diagnostic.clone()),
            Self::Io(error) => Self::io(error),
        }
    }
}

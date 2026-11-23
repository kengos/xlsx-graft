//! [`PartRewriter`] and [`PartWriter`]: how the content of a part the edit
//! changes or adds is produced.

use crate::error::WriteError;
use std::io::{BufRead, Write};

/// Writes the new content of a part from its original content.
///
/// [`PackageEdit::finish`](crate::PackageEdit::finish) calls it once, with
/// the part's inflated bytes and the writer the new content is deflated
/// into. It may read the original in part or not at all. An error that
/// comes from `original` or `out` passes through `?` as it is
/// ([`WriteError`] has `From<io::Error>` and `From<XmlStreamError>`); a
/// rewriter that refuses the content returns [`WriteError::Invalid`] with
/// the diagnostic to report.
pub trait PartRewriter {
    /// Reads the part's original content from `original` and writes the new
    /// content to `out`.
    ///
    /// # Errors
    ///
    /// A [`WriteError`]: [`WriteError::Invalid`] to refuse the content, or
    /// the error of `original` or `out`.
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError>;
}

/// Writes the content of a part that is added to the package.
///
/// [`PackageEdit::finish`](crate::PackageEdit::finish) calls it once, with
/// the writer the content is deflated into.
pub trait PartWriter {
    /// Writes the part's content to `out`.
    ///
    /// # Errors
    ///
    /// A [`WriteError`]: [`WriteError::Invalid`] to refuse the content, or
    /// the error of `out`.
    fn write(&mut self, out: &mut dyn Write) -> Result<(), WriteError>;
}

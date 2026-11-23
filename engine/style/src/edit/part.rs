//! [`StylesEdit`] as a [`PartRewriter`]: what `PackageEdit::replace` takes to
//! write the styles part of a package again, and how the errors of
//! [`StylesEdit::rewrite`] become the errors of a rewriter.

use super::{StyleError, StylesEdit};
use std::io::{self, BufRead, Write};
use xlsx_graft_write::{PartRewriter, WriteError};

impl PartRewriter for StylesEdit {
    /// Writes the styles part as [`StylesEdit::rewrite`] does, with its
    /// errors as [`write_error`] gives them.
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        StylesEdit::rewrite(self, original, out).map_err(write_error)
    }
}

/// The [`WriteError`] for `error`, a failure of [`StylesEdit::rewrite`].
///
/// - [`StyleError::Invalid`] is [`WriteError::Invalid`] with the same
///   diagnostic: the original styles part is refused, a problem of the
///   workbook, which `PackageEdit::finish` pushes at the part.
/// - [`StyleError::Io`] is what [`WriteError::from`] makes of the
///   [`io::Error`], so an `io::Error` that wraps a [`WriteError`] (as a
///   `DeflateWriter` returns) is converted back to that [`WriteError`].
/// - Every other [`StyleError`] is [`WriteError::Io`] holding it. Only
///   [`StylesEdit::cell_format`] returns them, never `rewrite`, and no
///   [`WriteError`] fits them: they are not a problem of the workbook, so
///   not `Invalid`.
fn write_error(error: StyleError) -> WriteError {
    match error {
        StyleError::Invalid(diagnostic) => WriteError::Invalid(diagnostic),
        StyleError::Io(error) => WriteError::from(error),
        other => WriteError::Io(io::Error::other(other)),
    }
}

#[cfg(test)]
mod tests;

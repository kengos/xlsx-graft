//! [`WriteError`]: the failures that writing a zip or editing a package can
//! end in.

use std::io;
use thiserror::Error;
use xlsx_graft_cell::{CellError, CellRef};
use xlsx_graft_diagnostics::Diagnostic;
use xlsx_graft_package::PackageError;
use xlsx_graft_spool::SpoolError;
use xlsx_graft_xml::XmlStreamError;

/// A failure of the host, a name that no zip can hold, a call that cannot be
/// carried out on the package it is made on, or the content a rewriter
/// refuses: never a problem in the workbook itself, which is a diagnostic.
#[derive(Debug, Error)]
pub enum WriteError {
    /// Writing the zip, or reading the archive an entry is copied from,
    /// failed.
    #[error("writing the zip failed: {0}")]
    Io(#[source] io::Error),
    /// The scratch storage the deflated content is held in failed.
    #[error("the scratch storage of the zip failed: {0}")]
    Spool(#[from] SpoolError),
    /// An earlier call failed or left a partial entry, so this call writes
    /// nothing. For a `ZipWriter` or a `PackageEdit`, what was written is not
    /// a zip. For a [`RowsWriter`](crate::RowsWriter), the rows stored are
    /// incomplete.
    #[error("an earlier call failed or left a partial entry: the output is not whole")]
    Unusable,
    /// An entry name is longer than the 65,535 bytes a zip name field holds
    /// (APPNOTE 6.3.10 §4.4.10).
    #[error("an entry name of {length} bytes is longer than a zip holds")]
    EntryNameTooLong {
        /// The length of the name in bytes.
        length: usize,
    },
    /// A part the call names is not in the package: the part to replace or
    /// remove, or the source of a link. The caller made a mistake.
    #[error("the package has no part {name}")]
    PartMissing {
        /// The part name as the caller gave it.
        name: String,
    },
    /// A name the call would take is not free. It is the name of the part to
    /// add, the name of the Relationships part a link would make, or the name
    /// of the Relationships part that a part added would have as its own and
    /// that the package holds already. It overlaps a name of the package the
    /// edit writes: a part of the package (the name of a part removed
    /// included), a name the package refused an entry for (a duplicate, or
    /// derived from another name, or no part name), a part added, or a
    /// Relationships part to be made. It overlaps when it is the same part,
    /// ASCII letter case aside, or one is the other and `/` and one or more
    /// segments (ECMA-376-2:2021 §6.2.2.3). The caller made a mistake.
    #[error(
        "the name {name} is the same part as a name of the package, or a folder of one, or in a folder of one"
    )]
    PartExists {
        /// The name refused. For a Relationships part a link would make, it
        /// is built from the spelling the package has of the link's source;
        /// for the Relationships part of the part added, from the spelling the
        /// caller gave the part; for the part itself, as the caller gave it.
        name: String,
    },
    /// The call names a Relationships part, which is written by the edit
    /// from the links of the parts it adds and the parts it removes, never
    /// replaced, removed or added by the caller.
    #[error("{name} is a Relationships part, which the edit writes itself")]
    PartReserved {
        /// The part name as the caller gave it.
        name: String,
    },
    /// A link names a source whose relationships are not all known: its
    /// Relationships part could not be read whole, so a new relationship
    /// might take an `Id` it already uses. For the package itself, `name`
    /// is `/_rels/.rels`; otherwise the name of the source part.
    #[error("the relationships of {name} are not all known, so one cannot be added to them")]
    RelationshipsUnknown {
        /// The source of the link: the part's name, or `/_rels/.rels`.
        name: String,
    },
    /// A part cannot be removed because the package's Relationships parts
    /// were not all read whole, so the relationships that target the part
    /// are not all known and some might be left pointing at nothing.
    #[error("the relationships of the package are not all known, so a part cannot be removed")]
    GraphIncomplete,
    /// The `rel_type` of a link has a character XML 1.0 does not allow (a
    /// control character such as U+0001, or U+FFFE or U+FFFF), which would
    /// make the Relationships part not well-formed. Nothing was changed. The
    /// caller made a mistake.
    #[error("the relationship type {rel_type:?} has a character XML does not allow")]
    RelationshipTypeInvalid {
        /// The type as the caller gave it.
        rel_type: String,
    },
    /// [`RowsWriter::push`](crate::RowsWriter::push) was given a row number
    /// that is not after the number of the last row pushed (a row without
    /// cells included). Nothing was written, and the writer is as it was
    /// before the call. The caller made a mistake.
    #[error("row {row} is not after row {last}, the last row pushed")]
    RowOutOfOrder {
        /// The row number given.
        row: u32,
        /// The number of the last row pushed.
        last: u32,
    },
    /// [`RowsWriter::push`](crate::RowsWriter::push) was given a row number
    /// that is 0 or past 1,048,576, the last row of a sheet. Nothing was
    /// written, and the writer is as it was before the call. The caller made
    /// a mistake.
    #[error("row {row} is outside the sheet (1 to 1048576)")]
    RowOutOfRange {
        /// The row number given.
        row: u32,
    },
    /// A cell given to [`RowsWriter::push`](crate::RowsWriter::push) is in
    /// column 0 or past 16,384 (XFD), the last column of a sheet. Nothing was
    /// written, and the writer is as it was before the call. The caller made
    /// a mistake.
    #[error("column {column} of row {row} is outside the sheet (1 to 16384)")]
    ColumnOutOfRange {
        /// The number of the row being pushed.
        row: u32,
        /// The column given.
        column: u16,
    },
    /// A cell given to [`RowsWriter::push`](crate::RowsWriter::push) is in a
    /// column that is not after the column of the cell before it in the row.
    /// Nothing was written, and the writer is as it was before the call. The
    /// caller made a mistake.
    #[error(
        "column {column} of row {row} is not after column {last}, the column of the cell before it"
    )]
    CellOutOfOrder {
        /// The number of the row being pushed.
        row: u32,
        /// The column given.
        column: u16,
        /// The column of the cell before it.
        last: u16,
    },
    /// `write_cell` refused the value of a cell given to
    /// [`RowsWriter::push`](crate::RowsWriter::push): a number that is not
    /// finite, an integer whose absolute value is past 2^53, or a date that
    /// has no serial date-time in the date system. Nothing was written, and the writer is as it was before the
    /// call. The caller made a mistake.
    #[error("the cell {at} is refused: {source}")]
    CellRefused {
        /// The cell whose value was refused.
        at: CellRef,
        /// Why the value was refused.
        #[source]
        source: CellError,
    },
    /// The built-in blank workbook ([`blank::package`](crate::blank::package))
    /// could not be opened as a package: a mistake of the engine, not of the
    /// caller.
    #[error("the built-in blank workbook could not be opened")]
    BlankUnreadable,
    /// A rewriter or writer refused the content of a part. The diagnostic
    /// says why, and `PackageEdit::finish` pushes it at the part and returns
    /// `Ok(None)`.
    #[error("the content of a part is refused ({}): {}", .0.code(), .0.message())]
    Invalid(Diagnostic),
}

impl From<PackageError> for WriteError {
    fn from(error: PackageError) -> Self {
        let PackageError::Io(error) = error;
        Self::from(error)
    }
}

impl From<io::Error> for WriteError {
    /// The [`WriteError`] that `error` carries, when it carries one (as the
    /// errors of [`DeflateWriter`](crate::DeflateWriter)'s `write` do); any
    /// other [`io::Error`] as [`WriteError::Io`].
    fn from(error: io::Error) -> Self {
        match error.downcast::<Self>() {
            Ok(carried) => carried,
            Err(error) => Self::Io(error),
        }
    }
}

impl From<XmlStreamError> for WriteError {
    /// [`WriteError::Invalid`] for an XML problem of the part, so a
    /// rewriter can pass it on with `?`; for a failure of the reader or
    /// writer underneath, what [`WriteError::from`] makes of its
    /// [`io::Error`].
    fn from(error: XmlStreamError) -> Self {
        match error {
            XmlStreamError::Invalid(diagnostic) => Self::Invalid(diagnostic),
            XmlStreamError::Io(error) => Self::from(error),
        }
    }
}

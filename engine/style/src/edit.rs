//! [`StylesEdit`]: an existing styles part read, cell formats added to it
//! without repeating an item it has, and the part written again with what was
//! added. It is made of `scan` (reading the part), `list` (the items read and
//! added), `assign` (`cell_format`) and `rewrite` (writing). `part` makes a
//! `StylesEdit` a [`PartRewriter`](xlsx_graft_write::PartRewriter).

mod assign;
mod list;
mod part;
mod rewrite;
mod scan;

use scan::{Scan, scan};
use std::io::{self, BufRead, Write};
use thiserror::Error;
use xlsx_graft_diagnostics::Diagnostic;
use xlsx_graft_xml::{XmlLimits, XmlStreamError};

use crate::Style;

/// The limits under which a [`StylesEdit`] reads and writes a styles part,
/// and which [`StylesEdit::cell_format`] checks.
///
/// `max_cell_formats`, `max_fonts` and `max_number_formats` limit what a call
/// of [`StylesEdit::cell_format`] adds: the call is refused when an item it
/// adds would make the list longer than the limit. A call that adds nothing
/// is never refused, even for a part that is over a limit already.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleLimits {
    /// The limits the part's XML is read and written under.
    pub xml: XmlLimits,
    /// The most cell formats (`xf` in `cellXfs`) the part may have after a
    /// call adds one: 65,490 by default, the most Excel opens.
    pub max_cell_formats: u32,
    /// The most fonts the part may have after a call adds one: 512 by
    /// default.
    pub max_fonts: u32,
    /// The most explicit number formats (`numFmt`) the part may have after a
    /// call adds one: 200 by default, the fewest of the limits of Excel's
    /// languages.
    pub max_number_formats: u32,
    /// The most items one list of the part may have when it is read
    /// (`TooMany` otherwise); `max_items_read` bounds the memory the edit
    /// uses: 65,536 by default.
    pub max_items_read: u32,
}

impl Default for StyleLimits {
    fn default() -> Self {
        Self {
            xml: XmlLimits::default(),
            max_cell_formats: 65_490,
            max_fonts: 512,
            max_number_formats: 200,
            max_items_read: 65_536,
        }
    }
}

/// What a list of the styles part holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StyleKind {
    /// An explicit number format (`numFmt`).
    NumberFormat,
    /// A font.
    Font,
    /// A fill.
    Fill,
    /// A border.
    Border,
    /// A cell format (`xf` of `cellXfs`).
    CellFormat,
}

/// Why a [`StylesEdit`] call failed.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StyleError {
    /// The styles part cannot be read (an `xml_*` diagnostic) or its shape
    /// is one an edit does not accept (`xml_schema_invalid`): a list with a
    /// child that is not its item, text where there is none, a list that is
    /// there twice or inside an element of another namespace, a root that is not
    /// a `styleSheet`.
    #[error("the styles part is refused: {0:?}")]
    Invalid(Diagnostic),
    /// A list is, or would become, longer than a limit. A new `numFmtId` that
    /// would not fit in a `u32` is also `TooMany`, with `kind` `NumberFormat`
    /// and `limit` the `max_number_formats`; then `limit` is not a limit on
    /// a number of items.
    #[error("the styles part would have more than {limit} of {kind:?}")]
    TooMany {
        /// What the list holds.
        kind: StyleKind,
        /// The limit.
        limit: u32,
    },
    /// A value of the style is out of range. The field is the name of the
    /// XML attribute or element that would hold it: `textRotation`, `tint`,
    /// `sz`, `name` or `formatCode`.
    #[error("the style's {field} is out of range")]
    InvalidValue {
        /// The attribute or element.
        field: &'static str,
    },
    /// The base cell format is not in the part's `cellXfs`.
    #[error("the styles part has no cell format {base}")]
    NoCellFormat {
        /// The index that was asked for.
        base: u32,
    },
    /// Reading the original part or writing the new one failed.
    #[error("reading or writing the styles part failed: {0}")]
    Io(#[from] io::Error),
}

impl From<Diagnostic> for StyleError {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Invalid(diagnostic)
    }
}

impl From<XmlStreamError> for StyleError {
    fn from(error: XmlStreamError) -> Self {
        match error {
            XmlStreamError::Invalid(diagnostic) => Self::Invalid(diagnostic),
            XmlStreamError::Io(error) => Self::Io(error),
        }
    }
}

/// `TooMany` for `kind` when a list of `count` items is longer than `limit`.
fn limit(kind: StyleKind, count: usize, limit: u32) -> Result<(), StyleError> {
    if u64::try_from(count).unwrap_or(u64::MAX) > u64::from(limit) {
        return Err(StyleError::TooMany { kind, limit });
    }
    Ok(())
}

/// An existing styles part (`xl/styles.xml`), read so that cell formats can
/// be added to it.
///
/// The part is not changed in place: [`read`](Self::read) reads it,
/// [`cell_format`](Self::cell_format) records what to add, and
/// [`rewrite`](Self::rewrite) writes the part again with only that added to
/// the end of the lists it belongs to. What the part already has is not
/// moved, renumbered or removed, so a cell's `s` keeps its meaning.
#[derive(Debug)]
pub struct StylesEdit {
    limits: StyleLimits,
    scan: Scan,
}

impl StylesEdit {
    /// Reads the styles part `bytes` under `limits`.
    ///
    /// # Errors
    ///
    /// [`StyleError::Invalid`] when the XML cannot be read or the part is
    /// not shaped as an edit needs; [`StyleError::TooMany`] when a list has
    /// more than `max_items_read` items.
    pub fn read(bytes: &[u8], limits: &StyleLimits) -> Result<Self, StyleError> {
        Ok(Self {
            scan: scan(bytes, limits)?,
            limits: limits.clone(),
        })
    }

    /// The index of the cell format that is the cell format `base` with
    /// `style` laid over it: the first one the part has that is equal, or a
    /// new one added after the last (with the font, fill, border and number
    /// format it needs, when the part does not have them). A cell's `s` is
    /// such an index. A part with no cell format accepts `base` 0, which is
    /// then an empty one.
    ///
    /// Nothing is added when the call fails.
    ///
    /// # Errors
    ///
    /// [`StyleError::InvalidValue`] for a value of `style` that is out of
    /// range; [`StyleError::NoCellFormat`] when the part has no cell format
    /// `base`; [`StyleError::TooMany`] when an item to add would exceed a
    /// limit.
    pub fn cell_format(&mut self, base: u32, style: &Style) -> Result<u32, StyleError> {
        self.scan.cell_format(&self.limits, base, style)
    }

    /// Writes the part to `out`: `original`, which must hold the same bytes
    /// [`read`](Self::read) was given, with what [`cell_format`] added. A
    /// part that nothing was added to is written as it was, byte for byte
    /// when it is UTF-8; one in UTF-16 is written as UTF-8.
    ///
    /// [`cell_format`]: Self::cell_format
    ///
    /// # Errors
    ///
    /// [`StyleError::Invalid`] when `original` cannot be read as XML, and
    /// with `xml_schema_invalid` when it is not the part `read` was given:
    /// the reason is `MissingElement` when it lacks a list that
    /// [`read`](Self::read) found and a call of [`cell_format`] added to,
    /// and `UnexpectedElement` when it has a list that `read` did not find
    /// and a call added; [`StyleError::Io`] when reading it or writing `out`
    /// fails. After an error, `out` holds only part of the new part, and the
    /// caller must discard `out`.
    pub fn rewrite(
        &self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), StyleError> {
        rewrite::rewrite(&self.scan, &self.limits.xml, original, out)
    }
}

#[cfg(test)]
mod tests;

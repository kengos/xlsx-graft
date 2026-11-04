//! The elements of a worksheet the checks read as they start or end: a
//! column, a row and a cell, its value checked as the cell ends, as
//! [`SheetCheck`] says.

use super::SheetCheck;
use super::count::{Unsigned, Utf16};
use super::place::{Refused, letters};
use crate::collect::attribute;
use xlsx_graft_diagnostics::{Diagnostic, Echo};
use xlsx_graft_read::CellRef;
use xlsx_graft_xml::events::BytesStart;

/// The most UTF-16 code units of text Excel keeps in a cell.
const MAX_TEXT_UNITS: u64 = 32_767;

/// What a cell's `t` says its value is, as far as it is checked.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Kind {
    Shared,
    Inline,
    #[default]
    Other,
}

/// The cell being read.
#[derive(Debug, Default)]
pub(super) struct Cell {
    /// Its A1 reference; the place it names when it is refused, or its
    /// `r` as written in a row refused.
    reference: String,
    pub(super) kind: Kind,
    /// Its `v`, for a shared string.
    pub(super) index: Unsigned,
    /// The length of its inline string.
    pub(super) length: Utf16,
}

impl SheetCheck<'_> {
    /// Checks a column's `style`.
    pub(super) fn column(&mut self, start: &BytesStart<'_>) {
        let Some((index, count)) = self.style(start, "style") else {
            return;
        };
        let label = |name: &str| {
            let text = attribute(start, name).unwrap_or_default();
            let column = Unsigned::parse(&text).and_then(|column| u16::try_from(column).ok());
            let column = column.filter(|column| (1..=CellRef::MAX_COLUMN).contains(column));
            column.map_or(text, letters)
        };
        let columns = format!("{}:{}", label("min"), label("max"));
        self.push(Diagnostic::ColumnStyleMissing {
            columns: Echo::new(&columns),
            index,
            count,
        });
    }

    /// Places a row and checks its `s`, in force with `customFormat` alone.
    pub(super) fn row(&mut self, start: &BytesStart<'_>) {
        let written = attribute(start, "r");
        let row = match self.place.row(written.as_deref()) {
            Ok(number) => number.to_string(),
            Err(refused) => self.refuse(refused),
        };
        let custom = attribute(start, "customFormat");
        let custom = custom.is_some_and(|value| {
            matches!(value.trim_matches([' ', '\t', '\r', '\n']), "1" | "true")
        });
        if custom && let Some((index, count)) = self.style(start, "s") {
            self.push(Diagnostic::RowStyleMissing {
                row: Echo::new(&row),
                index,
                count,
            });
        }
    }

    /// Places a cell, checks its `s` and starts reading its value.
    pub(super) fn cell(&mut self, start: &BytesStart<'_>) {
        let written = attribute(start, "r");
        let reference = match self.place.cell(written.as_deref()) {
            Ok(Some(reference)) => reference,
            Ok(None) => written.unwrap_or_default(),
            Err(refused) => self.refuse(refused),
        };
        if let Some((index, count)) = self.style(start, "s") {
            self.push(Diagnostic::CellStyleMissing {
                reference: Echo::new(&reference),
                index,
                count,
            });
        }
        let kind = match attribute(start, "t").as_deref() {
            Some("s") => Kind::Shared,
            Some("inlineStr") => Kind::Inline,
            _ => Kind::Other,
        };
        self.cell = Cell {
            reference,
            kind,
            ..Cell::default()
        };
    }

    /// Pushes why a row or cell was refused: the place it names.
    fn refuse(&mut self, refused: Refused) -> String {
        self.push(refused.diagnostic);
        refused.place
    }

    /// Checks the cell read: its shared string, or its inline string.
    pub(super) fn finish_cell(&mut self) {
        let cell = std::mem::take(&mut self.cell);
        let strings = self.counts.strings;
        match cell.kind {
            Kind::Shared => {
                if let (Some(index), Some(count)) = (cell.index.value(), strings)
                    && index >= count
                {
                    self.push(Diagnostic::SharedStringMissing { index });
                }
            }
            Kind::Inline if cell.length.units() > MAX_TEXT_UNITS => {
                self.push(Diagnostic::InlineStringTooLong {
                    reference: Echo::new(&cell.reference),
                    length: cell.length.units(),
                });
            }
            _ => {}
        }
    }

    /// The style index in the attribute `name` of `start`, and how many
    /// cell formats there are, when it is at or past them.
    fn style(&self, start: &BytesStart<'_>, name: &str) -> Option<(u64, u64)> {
        let index = Unsigned::parse(&attribute(start, name)?)?;
        let count = self.counts.styles?;
        (index >= count).then_some((index, count))
    }
}

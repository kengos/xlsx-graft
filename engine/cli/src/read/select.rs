//! Which sheet `read` reads, and [`Refusal`]: why it reads none to its
//! end.
//!
//! A sheet is asked for by name, compared as sheet names are, letter case
//! aside, or by its index in the workbook's list; the first when neither
//! is given. A name two sheets have names neither (the workbook's
//! `sheet_name_duplicate` says so too), since readers would differ on which
//! it means. Only a worksheet has rows to read: a chartsheet or a dialog
//! sheet holds no cells, and a macro sheet's are not read.

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use xlsx_graft_read::{Book, Sheet, SheetKind, SheetLookup};

/// Why `read` read no sheet to its end. Its code is the contract; its
/// message is English for a person and may change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Refusal {
    /// The package or its workbook part cannot be read, which the
    /// diagnostics say.
    WorkbookUnreadable,
    /// No sheet has the name.
    NoSuchName(String),
    /// There is no sheet at the index; the workbook has `count`.
    NoSuchIndex { index: usize, count: usize },
    /// More than one sheet has the name.
    Duplicate(String),
    /// The sheet of the name is of the kind, not a worksheet.
    NotWorksheet { name: String, kind: SheetKind },
    /// The sheet of the name has no part known, or one that cannot be
    /// read, which the diagnostics say.
    SheetUnreadable(String),
}

impl Refusal {
    /// The stable wire string.
    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::WorkbookUnreadable => "workbook_unreadable",
            Self::NoSuchName(_) | Self::NoSuchIndex { .. } => "sheet_not_found",
            Self::Duplicate(_) => "sheet_ambiguous",
            Self::NotWorksheet { .. } => "sheet_not_worksheet",
            Self::SheetUnreadable(_) => "sheet_unreadable",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkbookUnreadable => {
                write!(f, "the workbook cannot be read; the diagnostics say why")
            }
            Self::NoSuchName(name) => write!(f, "no sheet is named \"{name}\""),
            Self::NoSuchIndex { index, count } => write!(
                f,
                "there is no sheet at index {index}: the workbook has {count}, from index 0"
            ),
            Self::Duplicate(name) => write!(
                f,
                "more than one sheet is named \"{name}\", letter case aside"
            ),
            Self::NotWorksheet { name, kind } => {
                let kind = match kind {
                    SheetKind::Chartsheet => "a chartsheet, which holds no cells",
                    SheetKind::Dialogsheet => "a dialog sheet, which holds no cells",
                    _ => "a macro sheet, whose cells are not read",
                };
                write!(f, "sheet \"{name}\" is {kind}")
            }
            Self::SheetUnreadable(name) => {
                write!(
                    f,
                    "sheet \"{name}\" cannot be read; the diagnostics say why"
                )
            }
        }
    }
}

/// `{"code": ..., "message": ...}`.
impl Serialize for Refusal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("code", self.code())?;
        map.serialize_entry("message", &self.to_string())?;
        map.end()
    }
}

/// The sheet of `book` named `name`, else the one at `index`, else the
/// first, whatever its kind: its index and the sheet.
pub(super) fn sheet<'b>(
    book: &'b Book<BufReader<File>>,
    name: Option<&str>,
    index: Option<usize>,
) -> Result<(usize, &'b Sheet), Refusal> {
    let index = match name.map(|name| (name, book.sheet(name))) {
        None => index.unwrap_or(0),
        Some((_, SheetLookup::Found(index))) => index,
        Some((name, SheetLookup::Missing)) => return Err(Refusal::NoSuchName(name.to_owned())),
        Some((name, SheetLookup::Duplicate)) => return Err(Refusal::Duplicate(name.to_owned())),
    };
    let count = book.sheets().len();
    let sheet = (book.sheets().get(index)).ok_or(Refusal::NoSuchIndex { index, count })?;
    Ok((index, sheet))
}

/// Refuses `sheet` unless it is a worksheet, or one whose part is not
/// known, which reading then finds unreadable.
pub(super) fn worksheet(sheet: &Sheet) -> Result<(), Refusal> {
    match sheet.part().map(|part| part.kind()) {
        Some(SheetKind::Worksheet) | None => Ok(()),
        Some(kind) => Err(Refusal::NotWorksheet {
            name: sheet.name().to_owned(),
            kind,
        }),
    }
}

//! Choosing the sheet of a [`Book`] to read ([`SheetChoice`]), and
//! [`Refusal`]: why none is read to its end. Every host that reads rows
//! (the CLI, the C ABI) chooses through here, so each refuses the same
//! sheets with the same words.
//!
//! A sheet is asked for by name, compared as sheet names are, letter case
//! aside, or by its index in the workbook's list. A name two sheets have
//! names neither (the workbook's `sheet_name_duplicate` says so too), since
//! readers would differ on which it means. Only a worksheet has rows to
//! read: a chartsheet or a dialog sheet holds no cells, and a macro
//! sheet's are not read.

use crate::{Book, Sheet, SheetKind, SheetLookup, SheetRows};
use std::fmt;
use std::io::{Read, Seek};

/// Which sheet of a [`Book`] to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetChoice<'a> {
    /// The sheet of this name, letter case aside.
    Name(&'a str),
    /// The sheet at this index of [`Book::sheets`], hidden ones included.
    Index(usize),
}

/// Why no sheet was read to its end. Its [`code`](Self::code) is the
/// contract every host prints; its message (`Display`) is English for a
/// person and may change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The package or its workbook part cannot be read, which the
    /// diagnostics say.
    WorkbookUnreadable,
    /// No sheet has the name.
    NoSuchName(String),
    /// There is no sheet at the index.
    NoSuchIndex {
        /// The index asked for.
        index: usize,
        /// How many sheets the workbook has.
        count: usize,
    },
    /// More than one sheet has the name.
    Duplicate(String),
    /// The sheet is not a worksheet.
    NotWorksheet {
        /// The sheet's name.
        name: String,
        /// Its kind.
        kind: SheetKind,
    },
    /// The sheet of the name has no part known, or one that cannot be
    /// read, which the diagnostics say.
    SheetUnreadable(String),
}

impl Refusal {
    /// The stable wire string.
    #[must_use]
    pub fn code(&self) -> &'static str {
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

impl<R: Read + Seek> Book<R> {
    /// The sheet `choice` names, whatever its kind, and its index in
    /// [`sheets`](Self::sheets).
    ///
    /// # Errors
    ///
    /// [`Refusal::NoSuchName`], [`Refusal::Duplicate`] or
    /// [`Refusal::NoSuchIndex`].
    pub fn choose(&self, choice: SheetChoice<'_>) -> Result<(usize, &Sheet), Refusal> {
        let index = match choice {
            SheetChoice::Index(index) => index,
            SheetChoice::Name(name) => match self.sheet(name) {
                SheetLookup::Found(index) => index,
                SheetLookup::Missing => return Err(Refusal::NoSuchName(name.to_owned())),
                SheetLookup::Duplicate => return Err(Refusal::Duplicate(name.to_owned())),
            },
        };
        let count = self.sheets().len();
        let sheet = (self.sheets().get(index)).ok_or(Refusal::NoSuchIndex { index, count })?;
        Ok((index, sheet))
    }

    /// A reader of the rows of the sheet at `index`, or why there is none.
    ///
    /// # Errors
    ///
    /// [`Refusal::NoSuchIndex`] when there is no sheet at `index`,
    /// [`Refusal::NotWorksheet`] when it is not a worksheet, and
    /// [`Refusal::SheetUnreadable`] when its part is not known or cannot be
    /// read.
    pub fn open_rows(&self, index: usize) -> Result<SheetRows<R>, Refusal> {
        let (_, sheet) = self.choose(SheetChoice::Index(index))?;
        let name = || sheet.name().to_owned();
        match sheet.part().map(|part| part.kind()) {
            Some(SheetKind::Worksheet) | None => {}
            Some(kind) => return Err(Refusal::NotWorksheet { name: name(), kind }),
        }
        self.rows(index)
            .ok_or_else(|| Refusal::SheetUnreadable(name()))
    }
}

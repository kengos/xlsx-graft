//! [`Book`]: what a formula's names are resolved against, read before
//! the formulas are judged: the workbook's defined names and tables, and
//! which part each of its sheets is.
//!
//! The defined names are those the workbook part, the part the package's
//! `officeDocument` relationship targets, defines where Excel reads them
//! (`defined.rs`): a `definedName` elsewhere, in another part or out of
//! place in this one, is no name Excel looks a word up in, so a word that
//! names only such a one is judged as a function. So is one that a
//! workbook part that breaks its schema ([`WorkbookPart`]) defines, which
//! Excel may drop as it repairs the part. The tables are those of
//! the parts whose media type is a table's (ECMA-376-1:2016 §12.3.21),
//! the name and display name of the first `table` in each, a table part
//! holding one, whether or not a sheet relates to it: a table's name
//! resolves a word with no `(` alone, so one too many hides no call.
//! Leaving out a name errs towards refusing: a word it would
//! have resolved is denied as a function the table does not know. Every
//! `definedName`'s formula, wherever it is, is judged all the same, at
//! its own place.
//!
//! The sheets are the workbook part's, read by its schema
//! ([`WorkbookPart`]), in their namespace and order: the index of each is
//! what a `localSheetId` counts. A formula in a sheet's part sees that
//! sheet's names first; one in another part, a table's, a chart's or a
//! drawing's, sees the workbook's. A part two sheets name is no sheet's,
//! since which of the two Excel takes it for is not guessed. Seeing no
//! sheet errs towards refusing as well: the word sees the workbook's
//! names, then every scope's and is judged as a function too, and a
//! sheet's own defined names are judged at their own places all the same.
//!
//! What reading these parts finds is not reported here, but for a
//! problem of a zip entry, which the entry reports only when it is first
//! read, a workbook part larger than `maxXmlBytes`, which is read in
//! memory here alone, and a workbook part that defines more names than
//! `maxDefinedNames`, whose names past it are no names. When the workbook
//! part does not hold to its schema, no part is a sheet; every part is
//! read again as it is checked, and what stops that reading is reported
//! then.

mod defined;

use crate::collect::FormulaSink;
use crate::parts;
use crate::{Limits, Names};
use std::collections::BTreeMap;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_package::{Package, PackageError};
use xlsx_graft_read::{ReadError, ReadLimits, Workbook, WorkbookPart};

/// The media type of a table part (ECMA-376-1:2016 §12.3.21).
const TABLE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";

/// The names of a workbook, and which part each of its sheets is.
#[derive(Debug, Default)]
pub(crate) struct Book {
    /// The defined names and tables.
    pub(crate) names: Names,
    /// The index of each sheet, as a `localSheetId` counts it, by the name
    /// of its part; `None` for a part two sheets name.
    sheets: BTreeMap<String, Option<u32>>,
}

impl Book {
    /// Reads the names and sheets of the workbook in `package` under
    /// `limits`, as the module says, pushing into `diagnostics` a problem
    /// of a zip entry read (which reading it again would not report), a
    /// workbook part too large to read in memory and one that defines more
    /// than `maxDefinedNames`.
    pub(crate) fn read<R: Read + Seek>(
        package: &mut Package<R>,
        limits: &Limits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Self, PackageError> {
        let mut unreported = Diagnostics::new();
        let book = Self::gather(package, limits, &mut unreported, diagnostics);
        // An entry reports its damage once, when it is read first: here.
        for record in unreported.finish() {
            if record.location.entry.is_some() {
                diagnostics.push(record.diagnostic, record.location);
            }
        }
        book
    }

    /// [`read`](Self::read), pushing what it does not report into
    /// `unreported`.
    fn gather<R: Read + Seek>(
        package: &mut Package<R>,
        limits: &Limits,
        unreported: &mut Diagnostics,
        diagnostics: &mut Diagnostics,
    ) -> Result<Self, PackageError> {
        let mut book = Self::default();
        let xml = &limits.package.xml;
        if let Some(workbook) = Workbook::find(package, unreported) {
            let read_limits = ReadLimits {
                xml: *xml,
                ..ReadLimits::default()
            };
            let part = match WorkbookPart::read(package, &workbook, &read_limits, unreported) {
                Ok(Some(part)) => Some(part),
                // Only the shared strings spill to a spool.
                Ok(None) | Err(ReadError::Spool(_)) => None,
                Err(ReadError::Package(error)) => return Err(error),
            };
            let sheets = part.as_ref().map(WorkbookPart::sheets).unwrap_or_default();
            for (sheet, index) in sheets.iter().zip(0_u32..) {
                let part = sheet
                    .part()
                    .and_then(|part| package.parts().get(part.index()));
                if let Some(part) = part {
                    let name = part.name().as_str().to_owned();
                    let entry = book.sheets.entry(name).or_insert(Some(index));
                    *entry = entry.filter(|&first| first == index);
                }
            }
            let index = workbook.part();
            let names = Names::default();
            let (read, count) = defined::read(package, index, limits, names, diagnostics)?;
            // A part that breaks its schema may be repaired, its names
            // dropped: none is believed.
            book.names = if part.is_some() {
                read
            } else {
                Names::default()
            };
            let max = limits.max_defined_names;
            if count > max.value {
                let part = package.parts().get(index).map(|part| part.name().as_str());
                let location = Location::default().with_part(part.unwrap_or_default());
                let diagnostic = Diagnostic::DefinedNamesTooMany { count, limit: max };
                diagnostics.push(diagnostic, location);
            }
        }
        let tables: Vec<usize> = (package.parts().iter().enumerate())
            .filter(|(_, part)| part.content_type().is_some_and(|t| t.essence() == TABLE))
            .map(|(index, _)| index)
            .collect();
        let mut gather = Tables {
            names: std::mem::take(&mut book.names),
            first: false,
        };
        let max = limits.max_formula_bytes.value;
        for index in tables {
            gather.first = true;
            let collected = parts::collect(package, index, xml, max, &mut gather, unreported);
            collected?;
        }
        book.names = gather.names;
        Ok(book)
    }

    /// The index of the sheet whose part is `part`, if any.
    pub(crate) fn sheet(&self, part: &str) -> Option<u32> {
        self.sheets.get(part).copied().flatten()
    }
}

/// The `localSheetId` written `text`, if it is an `xsd:unsignedInt`: XML
/// white space around it is collapsed away.
pub(crate) fn local_sheet_id(text: &str) -> Option<u32> {
    text.trim_matches([' ', '\t', '\n', '\r']).parse().ok()
}

/// A sink that keeps the names of the first table of a table part.
#[derive(Debug)]
struct Tables {
    /// The names kept so far.
    names: Names,
    /// Whether the part being read has given no table yet: only its first
    /// gives names.
    first: bool,
}

impl FormulaSink for Tables {
    fn formula(&mut self, _part: &str, _at: &str, _text: &str) {}

    fn too_long(&mut self, _part: &str, _at: &str, _size: u64) {}

    fn markup(&mut self, _part: &str, _at: &str) {}

    fn defined_name(&mut self, _: &str, _: &str, _: &str, _: Option<&str>, _: &str) {}

    fn table(&mut self, _part: &str, names: &[String]) {
        if std::mem::take(&mut self.first) {
            for name in names {
                self.names = std::mem::take(&mut self.names).table(name);
            }
        }
    }
}

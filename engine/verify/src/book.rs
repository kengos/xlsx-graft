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
//! What the workbook holds that its sheets' cells refer to is read here
//! too, as the [`Worksheets`] check the cells against it: how many shared
//! strings there are, each held to the length Excel keeps
//! ([`strings::read`](crate::strings::read)), and how many cell formats
//! the styles part holds ([`Styles`]). A count not known, the part not
//! read to its end (stopped by the XML reader's limits as any part is),
//! refused by its schema, or styles past `maxXmlBytes`, `maxCellFormats`
//! or `maxNumberFormats` (each reported), is not checked against; neither is
//! a workbook without a styles part, as what Excel makes of a cell's `s`
//! then was not tried. What [`Structure`] checks is read here too
//! (`tables.rs`): the workbook part's sheets and defined names, and the
//! first table of each table part, with the sheet whose part relates to
//! it.
//!
//! What reading these parts finds is reported, but for what stops
//! reading a part as XML ([`stops_reading`]): every part is read again as
//! it is checked, and that is reported then. So the readers' own findings
//! come out once each: a problem of a zip entry, which the entry reports
//! only when it is first read; a workbook part or a styles part larger
//! than `maxXmlBytes`, or styles past the styles reader's limits, which
//! are read in memory here alone; two relationships of one type that
//! leave the workbook, its shared strings or its styles unknown
//! (`relationship_type_duplicate`); a workbook part that defines more
//! names than `maxDefinedNames`, whose names past it are no names; a
//! shared string too long; and what the workbook part's sheets are
//! reported for (`sheet_name_duplicate` and the like). A part's schema is
//! not held to here: the workbook and styles readers' hold it to more
//! than Excel does. When the workbook part does not hold to its schema,
//! no part is a sheet and [`Structure`] is not checked. A shared strings
//! or styles part whose media type is not XML, and that does not read as
//! XML, is not counted, and why is not reported: its media type is the
//! package's to report.

mod defined;
mod tables;

use crate::parts::stops_reading;
use crate::sheet::Counts;
use crate::strings;
use crate::worksheets::Worksheets;
use crate::{Limits, Names, SheetEntry, Structure};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek};
use tables::{TABLE, Table, Tables};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit, Location};
use xlsx_graft_package::{Package, PackageError, Presence, Target};
use xlsx_graft_read::{ReadError, ReadLimits, SHARED_STRINGS, Styles, Workbook, WorkbookPart};

/// The names of a workbook, and which part each of its sheets is.
#[derive(Debug, Default)]
pub(crate) struct Book {
    /// The defined names and tables.
    pub(crate) names: Names,
    /// The index of each sheet, as a `localSheetId` counts it, by the name
    /// of its part; `None` for a part two sheets name.
    sheets: BTreeMap<String, Option<u32>>,
    /// What [`Structure`] checks, but the tables; `None` when the workbook
    /// part is not read by its schema.
    structure: Option<Structure>,
    /// The first table of each table part, in the parts' order, of those
    /// [`Structure`] checks.
    tables: Vec<Table>,
}

impl Book {
    /// Reads the names and sheets of the workbook in `package` under
    /// `limits`, as the module says, pushing into `diagnostics` what it
    /// says: the book, and its sheets' parts to check.
    pub(crate) fn read<R: Read + Seek>(
        package: &mut Package<R>,
        limits: &Limits,
        diagnostics: &mut Diagnostics,
    ) -> Result<(Self, Worksheets), PackageError> {
        let mut unreported = unbounded();
        let book = Self::gather(package, limits, &mut unreported, diagnostics);
        report(unreported, diagnostics);
        book
    }

    /// [`read`](Self::read), pushing what it does not report into
    /// `unreported`.
    fn gather<R: Read + Seek>(
        package: &mut Package<R>,
        limits: &Limits,
        unreported: &mut Diagnostics,
        diagnostics: &mut Diagnostics,
    ) -> Result<(Self, Worksheets), PackageError> {
        let mut book = Self::default();
        let mut worksheets = Worksheets::new(limits.max_merged_cells);
        // The sheet each table part belongs to, and that sheet's part.
        let mut owners: BTreeMap<usize, Option<(u32, usize)>> = BTreeMap::new();
        let xml = &limits.package.xml;
        if let Some(workbook) = Workbook::find(package, unreported) {
            let read_limits = ReadLimits {
                xml: *xml,
                max_cell_formats: limits.max_cell_formats,
                max_number_formats: limits.max_number_formats,
                ..ReadLimits::default()
            };
            let part = match WorkbookPart::read(package, &workbook, &read_limits, unreported) {
                Ok(Some(part)) => Some(part),
                // Only the shared strings spill to a spool.
                Ok(None) | Err(ReadError::Spool(_)) => None,
                Err(ReadError::Package(error)) => return Err(error),
            };
            let sheets = part.as_ref().map(WorkbookPart::sheets).unwrap_or_default();
            // The sheet of each part the sheets name; `None` for a part two
            // of them name.
            let mut named: BTreeMap<usize, Option<u32>> = BTreeMap::new();
            for (sheet, index) in sheets.iter().zip(0_u32..) {
                if let Some(at) = sheet.part().map(|part| part.index()) {
                    let entry = named.entry(at).or_insert(Some(index));
                    *entry = entry.filter(|&first| first == index);
                }
            }
            let parts = named.iter().filter_map(|(&at, &index)| {
                let part = package.parts().get(at)?;
                Some((at, part, index))
            });
            for (at, part, index) in parts {
                book.sheets.insert(part.name().as_str().to_owned(), index);
                let mut links = BTreeSet::new();
                for relationship in part.relationships() {
                    if let Target::Internal(target) = relationship.target()
                        && (package.parts().get(*target))
                            .and_then(|part| part.content_type())
                            .is_some_and(|t| t.essence() == TABLE)
                    {
                        links.insert(relationship.id().to_owned());
                        let owner = owners.entry(*target).or_insert(index.map(|i| (i, at)));
                        *owner = owner.filter(|&(first, of)| Some(first) == index && of == at);
                    }
                }
                worksheets.add(at, links);
            }
            // What the workbook part's sheets were found for comes before
            // what is found after.
            report(std::mem::replace(unreported, unbounded()), diagnostics);
            let index = workbook.part();
            let names = Names::default();
            // The part was read just now: what reading it finds is reported.
            let again = &mut Diagnostics::new();
            let (read, entries, count) = defined::read(package, index, limits, names, again)?;
            let workbook_part = (package.parts().get(index))
                .map(|part| part.name().as_str().to_owned())
                .unwrap_or_default();
            // A part that breaks its schema may be repaired, its names
            // dropped: none is believed.
            if part.is_some() {
                book.names = read;
                book.structure = Some(Structure {
                    workbook_part: workbook_part.clone(),
                    sheets: (sheets.iter())
                        .map(|sheet| SheetEntry {
                            name: sheet.name().to_owned(),
                            sheet_id: sheet.sheet_id(),
                        })
                        .collect(),
                    defined_names: entries,
                    tables: Vec::new(),
                });
            }
            let max = limits.max_defined_names;
            if count > max.value {
                let location = Location::default().with_part(&workbook_part);
                let diagnostic = Diagnostic::DefinedNamesTooMany { count, limit: max };
                diagnostics.push(diagnostic, location);
            }
            worksheets.counts = counts(package, &workbook, &read_limits, unreported, diagnostics)?;
            report(std::mem::replace(unreported, unbounded()), diagnostics);
        }
        let tables: Vec<usize> = (package.parts().iter().enumerate())
            .filter(|(_, part)| part.content_type().is_some_and(|t| t.essence() == TABLE))
            .map(|(index, _)| index)
            .collect();
        let mut gather = Tables::new(std::mem::take(&mut book.names));
        let max = limits.max_formula_bytes.value;
        for index in tables {
            gather.part(index, owners.get(&index).copied().flatten());
            let collected =
                crate::parts::collect(package, index, xml, max, &mut gather, unreported);
            collected?;
        }
        (book.names, book.tables) = gather.finish();
        Ok((book, worksheets))
    }

    /// The index of the sheet whose part is `part`, if any.
    pub(crate) fn sheet(&self, part: &str) -> Option<u32> {
        self.sheets.get(part).copied().flatten()
    }
}

/// What the workbook in `package` holds that its sheets' cells refer to,
/// read under `limits` as the module says, a shared string too long pushed
/// into `diagnostics` and the rest into `unreported`.
fn counts<R: Read + Seek>(
    package: &mut Package<R>,
    workbook: &Workbook,
    limits: &ReadLimits,
    unreported: &mut Diagnostics,
    diagnostics: &mut Diagnostics,
) -> Result<Counts, PackageError> {
    let strings = match workbook.related(package, &SHARED_STRINGS, unreported) {
        Presence::Part(index) => {
            strings::read(package, index, &limits.xml, unreported, diagnostics)?
        }
        Presence::Absent => Some(0),
        Presence::Unknown => None,
    };
    let styles = match Styles::read(package, workbook, limits, unreported) {
        Ok(Styles::Formats(formats)) => u64::try_from(formats.len()).ok(),
        // Only the shared strings spill to a spool.
        Ok(Styles::Absent | Styles::Unknown) | Err(ReadError::Spool(_)) => None,
        Err(ReadError::Package(error)) => return Err(error),
    };
    Ok(Counts { strings, styles })
}

/// A collector of what the parts read ahead are found for, which keeps
/// every record: as many as the sheets the workbook part lists and the
/// parts the package holds, which both hold already.
fn unbounded() -> Diagnostics {
    Diagnostics::with_limit(Limit::default_of(u64::MAX))
}

/// Pushes into `diagnostics` what of `unreported` reading the parts again
/// does not report: all but what stops reading a part as XML.
fn report(unreported: Diagnostics, diagnostics: &mut Diagnostics) {
    for record in unreported.finish() {
        if !stops_reading(&record.diagnostic) {
            diagnostics.push(record.diagnostic, record.location);
        }
    }
}

/// The `localSheetId` written `text`, if it is an `xsd:unsignedInt`: XML
/// white space around it is collapsed away.
pub(crate) fn local_sheet_id(text: &str) -> Option<u32> {
    text.trim_matches([' ', '\t', '\n', '\r']).parse().ok()
}

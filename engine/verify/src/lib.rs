//! xlsx-graft verify: refuses what an untrusted workbook may not carry, and
//! finds what Excel repairs in it.
//!
//! [`verify`] opens a package under the [`Limits`] in force and pushes a
//! diagnostic for every problem it finds into the
//! [`Diagnostics`](xlsx_graft_diagnostics::Diagnostics) it is handed,
//! rather than stopping at the first. Opening it is the package reader's
//! own work, so what that refuses is refused here too: a zip bomb
//! (`zip_ratio_exceeded`, `zip_entry_too_large`, `zip_total_too_large`)
//! and a DOCTYPE in any XML read (`xml_doctype`), which is how an XML
//! external entity would come in.
//!
//! # Denied by default
//!
//! Content of these kinds ([`PartKind`]) is denied unless the template
//! allows it ([`Rules::allow_part`]); what the host's policy forbids
//! ([`Rules::forbid_part`]) is refused even then. Each part gets one
//! diagnostic per kind it holds, however many ways it shows it:
//! `part_denied`, or `part_forbidden` when the policy forbids the kind.
//!
//! - **Macros**: a VBA project and its signatures, a macro-enabled main
//!   part, an XLM macro sheet and a dialog sheet.
//! - **External links**: an external workbook reference part, and a
//!   relationship that points outside the package, also when its type
//!   names another kind: a relationship of an OLE object that points
//!   outside is an external link even where OLE is allowed. A hyperlink to
//!   an `http://`, `https://` or `mailto:` address is the one such
//!   relationship admitted, and only where its part uses it as a hyperlink
//!   (`hyperlink`, `hlinkClick`, `hlinkHover`) and as nothing else. An
//!   external workbook reference that holds a `ddeLink` is DDE instead,
//!   and one that holds an `oleLink` OLE, the relationship its link
//!   points outside by an external link as ever.
//! - **OLE**: an embedded object or package, and a worksheet's
//!   `oleObject` that refers to one.
//! - **ActiveX**: a control's persistence part (and its `ocx` root) and
//!   its binary.
//! - **Data connections**: the connections part, a query table, an XML
//!   map's `DataBinding`, and a Power Query definition (a `DataMashup`
//!   element, which Excel keeps in a custom XML part).
//! - **Web publishing**: a `webPublishItem` or `webPublishObject` element.
//!
//! A part's kind comes from its media type, from the type of every
//! relationship that targets it, and from the elements of its XML,
//! whichever says so: a part cannot pass by misnaming itself in one of
//! them. Every part but a Relationships part is read whole as XML, as a
//! stream, whatever its media type, its elements matched by local name in
//! any namespace, and the kinds it shows before reading stops count. Why
//! reading stops is reported for a part whose media type is XML; for
//! another, only when it shows the part is XML all the same (a DOCTYPE, a
//! limit crossed, or a declaration naming an encoding XML does not use,
//! other than UTF-32, or one the bytes are not in), so an image or a VML
//! drawing that is not well-formed raises nothing.
//!
//! # Formulas
//!
//! [`FormulaJudge`] judges a formula: what it calls, names and links to,
//! its names resolved against the workbook's ([`Names`]), by [`Rules`]
//! (their part side for a DDE link or another workbook, so `parts.allow`
//! admits those in a formula too; their function side for a function) and
//! a [`FunctionTable`]. [`DefaultFunctions`] is the engine's own table,
//! the safety column of `xlsx-graft-compat`'s function table: it passes
//! only a function reviewed safe, so a function nobody reviewed is denied
//! until a review marks it safe.
//!
//! [`verify`] finds the formulas as it reads the parts and judges each
//! at its place: those of cells, defined names, data validations,
//! conditional formats, table columns, pivot caches, charts, revisions,
//! linked objects, controls and drawings (the macros they name included),
//! their names resolved against the workbook part's defined names and the
//! table parts' tables, read first. A DDE link or an external reference
//! written in a formula (`=cmd|' /C calc'!A0`, `'C:\x\[b.xlsx]S'!A1`) is
//! refused as that kind of content, and a function as the rules and the
//! table say. A function the table schedules a later version of
//! xlsx-graft to deny is accepted with a notice
//! (`function_deny_scheduled`), unless the policy denies such functions
//! now ([`Rules::deny_scheduled`]). INDIRECT and OFFSET are accepted with a
//! notice that verify does not see which cells the reference they compute
//! points to (`function_reference_unchecked`). A formula longer than
//! `max_formula_bytes` (`formula_too_long`), or with an element inside it
//! (`formula_malformed`), is refused unread, and a workbook part that
//! defines more names than `max_defined_names` is refused
//! (`defined_names_too_many`).
//!
//! # Targets
//!
//! [`verify`] checks each function a formula calls against the versions of
//! the applications the template's `targets:` and `bestEffort:` hold
//! ([`Targets`]), by the compatibility table of `xlsx-graft-compat`, where
//! it judges the function: through the same names, so the functions of a
//! defined name's formula, a `LAMBDA`'s among them, are checked where a
//! formula reaches it. A function some of those versions lack, or are not
//! confirmed to have, the features it needs included (`dynamic_array` for
//! `FILTER`), is an error for `targets:` (`target_function_unsupported`)
//! and a warning for `bestEffort:` (`best_effort_function_unsupported`),
//! one per part, defined name reached through, function, application and
//! list, at the first place found, counting every time. A function the
//! table has no row for, one Excel 2007 already had, is not checked.
//!
//! It also checks the features of the table each part holds, found in the
//! same reading by the part's role, its media type or the type of a
//! relationship that targets it, and the elements [MS-XLSX] extends that
//! role with, in every part the package reaches through its relationships
//! (every part, when some may be left out): dynamic arrays, threaded comments, a PivotTable's
//! or a table's slicers, timelines, the Data Model, sparklines and
//! pictures placed in cells. One some versions lack, or are not confirmed
//! to have, is an error for `targets:` (`target_feature_unsupported`) and a
//! warning for `bestEffort:` (`best_effort_feature_unsupported`), one per
//! feature, application and list, at the first part that holds it,
//! counting the parts. A feature found is not blamed again on each
//! function that needs it: `FILTER` in a workbook found to use dynamic
//! arrays is checked as `FILTER` alone. Dynamic arrays are found by the
//! metadata Excel writes for them, which a formula that only may spill, a
//! single cell's included, is given as well. A template without
//! `targets:` is checked against [`DEFAULT_TARGETS`]
//! ([`Targets::default`] when it has no `bestEffort:` either).
//!
//! # What Excel repairs
//!
//! [`verify`] also finds what Excel repairs as it opens the workbook (an
//! error) or reads otherwise than it says without a word (a notice), as
//! Excel for Mac 16.114.3 did with a workbook damaged one way at a time;
//! what it opens as it is, it is not checked for. Every problem is found,
//! not only the first, each once.
//!
//! - **The package and its XML**, as it is opened and each part read: a
//!   relationship whose target is not there (`relationship_target_missing`),
//!   a part without a media type (`part_content_type_missing`), and XML
//!   that is not well-formed, a character XML does not allow included
//!   (`xml_malformed`).
//! - **The workbook**: a sheet's name two sheets share, letter case aside
//!   (`sheet_name_duplicate`), and what else the workbook part's sheets are
//!   refused for as the workbook reader reads them; and what [`Structure`]
//!   checks of the workbook part's sheets and defined names and the table
//!   parts' tables: a sheet's name or `sheetId`, a defined name's name,
//!   scope or sheets, and tables that overlap or share a name or an `id`.
//! - **Each worksheet**, as its part is read: where its rows and cells
//!   stand and in what order (`cell_reference_invalid`,
//!   `cell_order_invalid`), a shared string or a cell format a cell refers
//!   to that is not there (`shared_string_missing`, `cell_style_missing`,
//!   and for a column and a row), merged ranges that overlap
//!   (`merged_cells_overlap`), and an inline string longer than the 32,767
//!   characters Excel keeps in a cell (`inline_string_too_long`). A sheet
//!   with more merged ranges than `max_merged_cells` is refused
//!   (`merged_cells_too_many`), the ranges past it not checked.
//! - **The shared strings table**: a string longer than Excel keeps
//!   (`shared_string_too_long`).
//! - **The styles**: more cell formats than `max_cell_formats`
//!   (`cell_formats_too_many`) or number formats than
//!   `max_number_formats` (`number_formats_too_many`), as the styles are
//!   read to count the cell formats a cell may refer to.
//!
//! The workbook reader stops at the first problem of a worksheet; these
//! checks go on, and the worksheet is read by them alone, so a problem
//! both see is reported once. The shared strings table and the styles are
//! read before the worksheets, as the workbook part and the table parts
//! are, so a cell's reference is checked as the cell is read.
//!
//! # Not yet
//!
//! Not checked: Office add-ins (`webextension`), a ribbon (`customUI`),
//! attached toolbars (`attachedToolbars`), and the formulas a form
//! control keeps in its control properties or in VML (`fmlaMacro`,
//! `fmlaLink`, `fmlaRange` and the like), which no schema of ECMA-376
//! types.
//! What an embedded object or package holds is not read: allowing OLE
//! admits whatever it embeds, links a compound file keeps inside it
//! included.

mod book;
mod collect;
mod count;
mod formulas;
mod functions;
mod judging;
mod limits;
mod parts;
mod rules;
mod sheet;
mod strings;
mod structure;
mod used;
mod worksheets;

pub use formulas::{FormulaJudge, Names, Place};
pub use functions::{DefaultFunctions, FunctionClass, FunctionTable};
pub use limits::Limits;
pub use rules::{Rules, Ruling};
pub use structure::{NameEntry, SheetEntry, Structure, TableEntry};
pub use xlsx_graft_compat::{DEFAULT_TARGETS, Targets};
pub use xlsx_graft_diagnostics::PartKind;

use book::Book;
use collect::FormulaSink;
use judging::Judging;
use std::io::{Read, Seek};
use thiserror::Error;
use used::Used;
use worksheets::Worksheets;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Package, PackageError};

/// A failure of the host, apart from any problem in the workbook.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// Reading or seeking the package failed.
    #[error(transparent)]
    Package(#[from] PackageError),
}

/// Verifies the workbook `reader` holds, pushing a diagnostic for each
/// problem into `diagnostics`.
///
/// `functions` classifies the functions the formulas call, and `targets`
/// holds the versions of the applications they are checked against.
///
/// # Errors
///
/// [`VerifyError::Package`] when reading or seeking `reader` fails;
/// `diagnostics` then holds what was found up to there.
pub fn verify<R: Read + Seek>(
    reader: R,
    limits: &Limits,
    rules: &Rules,
    functions: &dyn FunctionTable,
    targets: &Targets,
    diagnostics: &mut Diagnostics,
) -> Result<(), VerifyError> {
    let Some(mut package) = Package::open(reader, &limits.package, diagnostics)? else {
        return Ok(());
    };
    let (mut book, mut worksheets) = Book::read(&mut package, limits, diagnostics)?;
    let mut judging = Judging::new(rules, functions, targets, &book, limits.max_formula_bytes);
    let used = read_parts(
        &mut package,
        limits,
        rules,
        &mut judging,
        &mut worksheets,
        diagnostics,
    )?;
    judging.finish(&used.features(), diagnostics);
    used.judge(targets, diagnostics);
    if let Some(structure) = book.structure(&package, &worksheets) {
        structure.check(diagnostics);
    }
    Ok(())
}

/// Checks the parts of `package`, handing the formulas they hold to
/// `formulas`, checking those `worksheets` holds as worksheets and handing
/// back the features they hold.
fn read_parts<R: Read + Seek>(
    package: &mut Package<R>,
    limits: &Limits,
    rules: &Rules,
    formulas: &mut dyn FormulaSink,
    worksheets: &mut Worksheets,
    diagnostics: &mut Diagnostics,
) -> Result<Used, VerifyError> {
    let (xml, max) = (&limits.package.xml, limits.max_formula_bytes.value);
    let used = parts::check(package, xml, rules, max, formulas, worksheets, diagnostics);
    used.map_err(VerifyError::from)
}

#[cfg(test)]
mod tests;

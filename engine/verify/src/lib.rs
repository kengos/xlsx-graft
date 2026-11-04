//! xlsx-graft verify: refuses what an untrusted workbook may not carry.
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
//!   (`hyperlink`, `hlinkClick`, `hlinkHover`) and as nothing else. An external workbook reference that holds a
//!   `ddeLink` is DDE instead, and one that holds an `oleLink` OLE, the
//!   relationship its link points outside by an external link as ever.
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
//! a [`FunctionTable`].
//!
//! [`verify`] finds the formulas as it reads the parts and judges each
//! at its place: those of cells, defined names, data validations,
//! conditional formats, table columns, pivot caches, charts, revisions,
//! linked objects, controls and drawings (the macros they name included),
//! their names resolved against the workbook part's defined names and the
//! table parts' tables, read first. A DDE link or an external reference
//! written in a formula (`=cmd|' /C calc'!A0`, `'C:\x\[b.xlsx]S'!A1`) is
//! refused as that kind of content, and a function as the rules and the
//! table say. A formula longer than `max_formula_bytes`
//! (`formula_too_long`), or with an element inside it
//! (`formula_malformed`), is refused unread, and a workbook part that
//! defines more names than `max_defined_names` is refused
//! (`defined_names_too_many`).
//!
//! # What Excel repairs
//!
//! [`Structure`] checks what the workbook part says of its sheets and
//! defined names, and the table parts of their tables, for what Excel
//! repairs as it opens the workbook (an error) or reads otherwise than it
//! says without a word (a notice): a sheet's name or `sheetId`, a defined
//! name's name, scope or sheets, and tables that overlap or share a name
//! or an `id`. [`verify`] does not call it yet.
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
mod formulas;
mod functions;
mod judging;
mod parts;
mod rules;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "verify reads its sheets through it once #13 wires it in"
    )
)]
mod sheet;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "verify checks the shared strings once the checks of #13 are wired in"
    )
)]
mod strings;
mod structure;

pub use formulas::{FormulaJudge, Names, Place};
pub use functions::{FunctionClass, FunctionTable};
pub use rules::{Rules, Ruling};
pub use structure::{NameEntry, SheetEntry, Structure, TableEntry};
pub use xlsx_graft_diagnostics::PartKind;

use book::Book;
use collect::FormulaSink;
use judging::Judging;
use std::io::{Read, Seek};
use thiserror::Error;
use xlsx_graft_diagnostics::{Diagnostics, Limit};
use xlsx_graft_package::{Package, PackageError};

/// The limits [`verify`] holds a workbook to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The package reader's, whose XML limits also hold each XML part
    /// read.
    pub package: xlsx_graft_package::Limits,
    /// The most bytes of one formula kept, as UTF-8: a longer one is
    /// counted, not kept. Policy name `maxFormulaBytes`.
    pub max_formula_bytes: Limit,
    /// The most defined names of the workbook part read: past it, the
    /// workbook is refused, and the names past it are no names a formula
    /// is judged by. Policy name `maxDefinedNames`.
    pub max_defined_names: Limit,
    /// The most merged ranges of one worksheet kept to check: past it,
    /// the sheet is refused, and the ranges past it are not checked.
    /// Policy name `maxMergedCells`.
    pub max_merged_cells: Limit,
}

impl Limits {
    /// The default `max_formula_bytes`, 64 KiB: Excel keeps a formula to
    /// 8,192 characters, at most 24 KiB as UTF-8, and under 56 KiB with
    /// every character written as an `_xHHHH_` escape.
    pub const DEFAULT_MAX_FORMULA_BYTES: u64 = 64 * 1024;

    /// The default `max_defined_names`, 65,536: each name kept costs a few
    /// hundred bytes beside its text.
    pub const DEFAULT_MAX_DEFINED_NAMES: u64 = 65_536;

    /// The default `max_merged_cells`, 262,144: each range kept costs some
    /// 100 bytes as overlaps are looked for, under 30 MiB at the most, and
    /// the columns they reach 1 MiB at the most.
    pub const DEFAULT_MAX_MERGED_CELLS: u64 = 262_144;
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            package: xlsx_graft_package::Limits::default(),
            max_formula_bytes: Limit::default_of(Self::DEFAULT_MAX_FORMULA_BYTES),
            max_defined_names: Limit::default_of(Self::DEFAULT_MAX_DEFINED_NAMES),
            max_merged_cells: Limit::default_of(Self::DEFAULT_MAX_MERGED_CELLS),
        }
    }
}

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
/// `functions` classifies the functions the formulas call.
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
    diagnostics: &mut Diagnostics,
) -> Result<(), VerifyError> {
    let Some(mut package) = Package::open(reader, &limits.package, diagnostics)? else {
        return Ok(());
    };
    let book = Book::read(&mut package, limits, diagnostics)?;
    let mut judging = Judging::new(rules, functions, &book, limits.max_formula_bytes);
    read_parts(&mut package, limits, rules, &mut judging, diagnostics)?;
    judging.finish(diagnostics);
    Ok(())
}

/// Checks the parts of `package`, handing the formulas they hold to
/// `formulas`.
fn read_parts<R: Read + Seek>(
    package: &mut Package<R>,
    limits: &Limits,
    rules: &Rules,
    formulas: &mut dyn FormulaSink,
    diagnostics: &mut Diagnostics,
) -> Result<(), VerifyError> {
    let (xml, max) = (&limits.package.xml, limits.max_formula_bytes.value);
    parts::check(package, xml, rules, max, formulas, diagnostics)?;
    Ok(())
}

#[cfg(test)]
mod tests;

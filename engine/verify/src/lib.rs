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
//! # Not yet
//!
//! The formulas are not read yet, so a DDE link or an external reference
//! written in a formula (`=cmd|' /C calc'!A0`, `'C:\x\[b.xlsx]S'!A1`) is
//! not refused here: that is the formula check's. [`FunctionTable`] and
//! the function side of [`Rules`] are the shapes it takes. Not checked
//! either: Office add-ins (`webextension`), a ribbon (`customUI`),
//! attached toolbars (`attachedToolbars`), and the macro name a form
//! control gives in VML or in its control properties (`fmlaMacro`), or a
//! drawing's shape in its `macro` attribute, which formulas will read.
//! What an embedded object or package holds is not read: allowing OLE
//! admits whatever it embeds, links a compound file keeps inside it
//! included.

mod functions;
mod parts;
mod rules;

pub use functions::{FunctionClass, FunctionTable};
pub use rules::{Rules, Ruling};
pub use xlsx_graft_diagnostics::PartKind;

use std::io::{Read, Seek};
use thiserror::Error;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Package, PackageError};

/// The limits [`verify`] holds a workbook to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Limits {
    /// The package reader's, whose XML limits also hold each XML part
    /// read.
    pub package: xlsx_graft_package::Limits,
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
/// `functions` classifies the functions the formulas call, for the formula
/// check that is not here yet.
///
/// # Errors
///
/// [`VerifyError::Package`] when reading or seeking `reader` fails.
pub fn verify<R: Read + Seek>(
    reader: R,
    limits: &Limits,
    rules: &Rules,
    _functions: &dyn FunctionTable,
    diagnostics: &mut Diagnostics,
) -> Result<(), VerifyError> {
    let Some(mut package) = Package::open(reader, &limits.package, diagnostics)? else {
        return Ok(());
    };
    parts::check(&mut package, &limits.package.xml, rules, diagnostics)?;
    Ok(())
}

#[cfg(test)]
mod tests;

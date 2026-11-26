//! [`Workbook`]: what a structure edit reads of a workbook, in memory: the
//! workbook part's sheets, defined names, views and whether it has `calcPr`
//! (ECMA-376-1:2016 §18.2), the relationships of the part, and the names of
//! the tables.
//!
//! The reading is small on purpose. It takes the elements a structure edit
//! changes (`sheets`, `definedNames`, `bookViews`, `calcPr`), each from its
//! one place in the workbook part, and skips every other element with all it
//! holds. A workbook it cannot read whole is reported as a diagnostic and
//! gives `None`, never a partial model: an edit that changed a workbook it
//! did not understand could lose what it did not read. That includes a
//! workbook whose relationships were not all read: an edit chooses the `Id`
//! of a relationship it adds from the ones the model holds, so it needs all
//! of them.
//!
//! The sheets are found as `xlsx-graft-read` finds them, and a sheet that
//! cannot be tied to a part pushes the diagnostic the reader pushes, also
//! when the relationships of the workbook part were not all read (only
//! `sheet_relationship_missing` is left out then, as the reader leaves it
//! out). The writer is published before the reader
//! (`scripts/release/crates.txt`), so it cannot depend on it: the rules are
//! written again in `link.rs`, and `engine/fixtures-gen/tests/model` checks
//! that the two agree on the workbooks of `fixtures/`.
//!
//! - **`walk.rs`**: the XML of the workbook part.
//! - **`link.rs`**: the workbook part, found by the package's
//!   `officeDocument` relationship, and each sheet's `r:id` tied to a
//!   relationship.
//! - **`parts.rs`**: the types of what is read: sheets, defined names, views
//!   and tables.
//! - **`tables.rs`**: the names of the tables of the table parts.
//!
//! A workbook that defines more than 65,536 names also gives `None`.

mod link;
mod parts;
mod tables;
mod walk;

use crate::error::WriteError;
use crate::limits::WriteLimits;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostics, Location};
use xlsx_graft_package::{Package, Relationship};

pub use parts::{BookView, DefinedName, Sheet, SheetKind, SheetState, Table};
pub(crate) use walk::MAX_DEFINED_NAMES;

/// The workbook part of a package, as a structure edit reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workbook {
    part: usize,
    strict: bool,
    sheets: Vec<Sheet>,
    defined_names: Vec<DefinedName>,
    views: Vec<BookView>,
    calc_pr: bool,
    defined_names_element: bool,
    book_views_element: bool,
    relationships: Vec<Relationship>,
    tables: Vec<Table>,
}

impl Workbook {
    /// Reads the workbook part of `package`, under the XML limits of
    /// `limits`, pushing each problem found. `None` when the workbook cannot
    /// be read whole, which is pushed already or was by the package: no
    /// `officeDocument` relationship, two `officeDocument` relationships
    /// that target different parts, a workbook part that is too large,
    /// damaged, defines more names than the limit or breaks the shape this
    /// reading holds it to, a sheet that cannot be tied to a part, a table
    /// part whose XML is not well formed, or relationships of the workbook
    /// part that were not all read (the package pushed why; the diagnostics
    /// of the sheets that cannot be tied to a part are still pushed, except
    /// `sheet_relationship_missing`).
    ///
    /// # Errors
    ///
    /// [`WriteError::Io`] when reading the package fails.
    pub fn read<R: Read + Seek>(
        package: &mut Package<R>,
        limits: &WriteLimits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Self>, WriteError> {
        let Some(index) = link::find(package, diagnostics) else {
            return Ok(None);
        };
        let xml = &limits.xml;
        let Some(bytes) = package.read_xml_part(index, xml.max_bytes, diagnostics)? else {
            return Ok(None);
        };
        let part = package.parts().get(index);
        let location = (part.map(|part| Location::default().with_part(part.name().as_str())))
            .unwrap_or_default();
        let walked = match walk::read(&bytes, xml) {
            Ok(walked) => walked,
            Err(diagnostic) => {
                diagnostics.push(diagnostic, location);
                return Ok(None);
            }
        };
        // When the package did not read every relationship of the part, a
        // sheet's `r:id` that matches none may refer to a relationship the
        // package did not keep, so `sheet_relationship_missing` is not
        // pushed for it.
        let known = part.is_some_and(|part| part.relationships_known());
        let relationships = part.map_or_else(Vec::new, |part| part.relationships().to_vec());
        let rels = part.map_or_else(String::new, |part| {
            part.name().relationships_part().as_str().to_owned()
        });
        let linked = link::link(
            &relationships,
            known,
            &rels,
            walked.sheets,
            &location,
            diagnostics,
        );
        let Some(sheets) = linked else {
            return Ok(None);
        };
        // A later edit chooses the `Id` of a relationship it adds from
        // `relationships`, so it needs every one the part has.
        if !known {
            return Ok(None);
        }
        let Some(tables) = tables::read(package, xml, diagnostics)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            part: index,
            strict: walked.strict,
            sheets,
            defined_names: walked.defined_names,
            views: walked.views,
            calc_pr: walked.calc_pr,
            defined_names_element: walked.defined_names_element,
            book_views_element: walked.book_views_element,
            relationships,
            tables,
        }))
    }

    /// The index of the workbook part in
    /// [`Package::parts`](xlsx_graft_package::Package::parts).
    #[must_use]
    pub fn part(&self) -> usize {
        self.part
    }

    /// Whether the root is in the Strict SpreadsheetML namespace.
    #[must_use]
    pub fn strict(&self) -> bool {
        self.strict
    }

    /// The sheets, in the order the part lists them.
    #[must_use]
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }

    /// The defined names, in the order the part lists them.
    #[must_use]
    pub fn defined_names(&self) -> &[DefinedName] {
        &self.defined_names
    }

    /// The workbook views, in order.
    #[must_use]
    pub fn views(&self) -> &[BookView] {
        &self.views
    }

    /// Whether the part has a `calcPr`.
    #[must_use]
    pub fn calc_pr(&self) -> bool {
        self.calc_pr
    }

    /// Whether the part has a `definedNames` element, empty or not.
    #[must_use]
    pub fn defined_names_element(&self) -> bool {
        self.defined_names_element
    }

    /// Whether the part has a `bookViews` element, empty or not.
    #[must_use]
    pub(crate) fn book_views_element(&self) -> bool {
        self.book_views_element
    }

    /// The relationships of the workbook part, in the order its
    /// Relationships part declares them.
    #[must_use]
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    /// The tables, in the order of the parts that hold them ([`Package::parts`]),
    /// each with its names: one for each element named `table` of a table
    /// part.
    ///
    /// [`Package::parts`]: xlsx_graft_package::Package::parts
    #[must_use]
    pub fn tables(&self) -> &[Table] {
        &self.tables
    }
}

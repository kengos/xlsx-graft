//! The workbook part, found by the package's `officeDocument`
//! relationship ([`find`]), and each sheet the walk listed tied to a part
//! through its `r:id` ([`link`]).
//!
//! What counts as a problem, and the diagnostic each pushes, is the reader's
//! (`xlsx-graft-read`'s `Workbook::find` and its workbook part's `link.rs`),
//! in the same order: the writer is published before the reader, so the rules
//! are written again here, and `engine/fixtures-gen/tests/model` checks the two
//! agree. When the relationships of the workbook part were not all read, a
//! sheet whose `r:id` matches no relationship pushes nothing (the package
//! pushed why), as in the reader; the model still gives no sheets, since it
//! cannot hold a sheet without a part.
//!
//! Names are compared lowercased as Unicode has it (`str::to_lowercase`),
//! the same in every locale: Excel requires sheet names to differ letter case
//! aside in any locale (MS-OI29500 on ECMA-376-1:2016 §18.2.19, note d) and
//! does not say how it folds case.

use super::walk::Listed;
use super::{Sheet, SheetKind, SheetState};
use std::collections::BTreeMap;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};
use xlsx_graft_package::{OFFICE_DOCUMENT, Package, Relationship, Target};

/// The relationship types of each kind of sheet: Transitional and Strict
/// (ECMA-376-1:2016 §12.3), and the two of a macro sheet ([MS-OFFMACRO2]
/// 2.2.1.1 and 2.2.1.2).
const KINDS: [(&str, SheetKind); 8] = [
    (
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet",
        SheetKind::Worksheet,
    ),
    (
        "http://purl.oclc.org/ooxml/officeDocument/relationships/worksheet",
        SheetKind::Worksheet,
    ),
    (
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet",
        SheetKind::Chartsheet,
    ),
    (
        "http://purl.oclc.org/ooxml/officeDocument/relationships/chartsheet",
        SheetKind::Chartsheet,
    ),
    (
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/dialogsheet",
        SheetKind::Dialogsheet,
    ),
    (
        "http://purl.oclc.org/ooxml/officeDocument/relationships/dialogsheet",
        SheetKind::Dialogsheet,
    ),
    (
        "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet",
        SheetKind::Macrosheet,
    ),
    (
        "http://schemas.microsoft.com/office/2006/relationships/xlIntlMacrosheet",
        SheetKind::Macrosheet,
    ),
];

/// The index of the workbook part: the part the package's internal
/// `officeDocument` relationships target. `None` when there is none, which
/// the package model reported already as `main_document_missing` unless its
/// relationships were not all read, or when two target different parts
/// (`relationship_type_duplicate`, pushed here, at the second one).
pub(super) fn find<R: Read + Seek>(
    package: &Package<R>,
    diagnostics: &mut Diagnostics,
) -> Option<usize> {
    let mut targets = (package.relationships().iter())
        .filter(|relationship| OFFICE_DOCUMENT.contains(&relationship.rel_type()))
        .filter_map(|relationship| match relationship.target() {
            Target::Internal(part) => Some((relationship, *part)),
            Target::External(_) => None,
        });
    let (_, first) = targets.next()?;
    if let Some((second, _)) = targets.find(|&(_, part)| part != first) {
        let location = Location::default()
            .with_rels_part("/_rels/.rels")
            .with_relationship_id(second.id());
        let rel_type = Echo::new(second.rel_type());
        diagnostics.push(Diagnostic::RelationshipTypeDuplicate { rel_type }, location);
        return None;
    }
    Some(first)
}

/// The sheets of `listed`, each with its part among `relationships`, the
/// ones the Relationships part `rels` of the workbook part declares, pushing
/// each problem found; `None` when there is one, or when a sheet has no part.
/// `known` is whether `relationships` are all the part has: when not, a
/// sheet whose `r:id` matches none is not pushed as
/// `sheet_relationship_missing`. A problem in the part's XML points to
/// `location`.
pub(super) fn link(
    relationships: &[Relationship],
    known: bool,
    rels: &str,
    listed: Vec<Listed>,
    location: &Location,
    diagnostics: &mut Diagnostics,
) -> Option<Vec<Sheet>> {
    // Whether a duplicate name or part was found. It is a flag and not a comparison of
    // the number of records before and after, because a collector at its
    // limit stops keeping records.
    let mut failed = false;
    // The `Id`s of one part are unique: the package model leaves out the
    // ones that are not.
    let by_id: BTreeMap<&str, &Relationship> = (relationships.iter())
        .map(|relationship| (relationship.id(), relationship))
        .collect();
    // The index in `names` of the first sheet of each folded name, and of
    // each part. A name is held once, in `names`, as the reader holds them.
    let mut first_named: BTreeMap<String, usize> = BTreeMap::new();
    let mut first_on: BTreeMap<usize, usize> = BTreeMap::new();
    let mut names: Vec<String> = Vec::with_capacity(listed.len());
    let mut tied: Vec<Option<Tied>> = Vec::with_capacity(listed.len());
    for (index, listed) in listed.into_iter().enumerate() {
        let found = match by_id.get(listed.relationship_id.as_str()) {
            Some(relationship) => {
                let at = Location::default()
                    .with_rels_part(rels)
                    .with_relationship_id(relationship.id());
                target(relationship, at, diagnostics)
            }
            None => {
                if known {
                    let relationship_id = Echo::new(&listed.relationship_id);
                    let diagnostic = Diagnostic::SheetRelationshipMissing { relationship_id };
                    diagnostics.push(diagnostic, location.clone());
                }
                None
            }
        };
        let name_of = |other: Option<&usize>| {
            let other = other.and_then(|other| names.get(*other));
            Echo::new(other.map_or("", String::as_str))
        };
        let other = first_named.get(&listed.name.to_lowercase());
        if other.is_some() {
            failed = true;
            let (name, other) = (Echo::new(&listed.name), name_of(other));
            let diagnostic = Diagnostic::SheetNameDuplicate { name, other };
            diagnostics.push(diagnostic, location.clone());
        } else {
            first_named.insert(listed.name.to_lowercase(), index);
        }
        if let Some((part, _)) = found {
            let other = first_on.get(&part);
            if other.is_some() {
                failed = true;
                let (name, other) = (Echo::new(&listed.name), name_of(other));
                let diagnostic = Diagnostic::SheetPartDuplicate { name, other };
                diagnostics.push(diagnostic, location.clone());
            } else {
                first_on.insert(part, index);
            }
        }
        tied.push(found.map(|(part, kind)| Tied {
            sheet_id: listed.sheet_id,
            state: listed.state,
            relationship_id: listed.relationship_id,
            part,
            kind,
        }));
        names.push(listed.name);
    }
    // A sheet with no part gives no model, whether or not a diagnostic was
    // pushed for it (`target` pushes one; a `r:id` that matches no
    // relationship does when `known`), so `collect` gives `None` for it.
    if failed {
        return None;
    }
    (names.into_iter().zip(tied))
        .map(|(name, tied)| tied.map(|tied| tied.into_sheet(name)))
        .collect()
}

/// What a sheet is tied to, before its name is added.
struct Tied {
    sheet_id: u32,
    state: SheetState,
    relationship_id: String,
    part: usize,
    kind: SheetKind,
}

impl Tied {
    /// The sheet named `name`.
    fn into_sheet(self, name: String) -> Sheet {
        Sheet {
            name,
            sheet_id: self.sheet_id,
            state: self.state,
            relationship_id: self.relationship_id,
            part: self.part,
            kind: self.kind,
        }
    }
}

/// The part `relationship`, the one a sheet's `r:id` names, targets, and
/// its kind; `None`, pushed at `at`, the relationship, when it is not of a
/// sheet's type or is external. Its type may be Transitional or Strict
/// whatever the workbook's kind.
fn target(
    relationship: &Relationship,
    at: Location,
    diagnostics: &mut Diagnostics,
) -> Option<(usize, SheetKind)> {
    let rel_type = relationship.rel_type();
    let Some(&(_, kind)) = KINDS.iter().find(|(known, _)| *known == rel_type) else {
        let rel_type = Echo::new(rel_type);
        diagnostics.push(Diagnostic::SheetRelationshipTypeInvalid { rel_type }, at);
        return None;
    };
    match relationship.target() {
        Target::Internal(index) => Some((*index, kind)),
        Target::External(_) => {
            let relationship_id = Echo::new(relationship.id());
            let diagnostic = Diagnostic::SheetRelationshipMissing { relationship_id };
            diagnostics.push(diagnostic, at);
            None
        }
    }
}

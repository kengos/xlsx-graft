//! [`link`]: each sheet the walk listed, its `r:id` looked up among the
//! workbook part's relationships and its kind taken from the type of the
//! one it names, never from a part name; and the sheets two of which share
//! a name, letter case aside, or a part.
//!
//! Names are compared lowercased as Unicode has it (`str::to_lowercase`),
//! the same in every locale: Excel requires them to differ letter case
//! aside in any locale (MS-OI29500 on §18.2.19, note d) and does not say
//! how it folds case, so a pair this folding alone makes equal (the Kelvin
//! sign and `k`) may be one Excel takes apart.

use super::walk::Listed;
use super::{Sheet, SheetKind, SheetPart};
use crate::Workbook;
use crate::workbook::rels_of;
use std::collections::BTreeMap;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};
use xlsx_graft_package::{Package, Relationship, Target};

/// The relationship types of each kind of sheet: Transitional and Strict
/// (§12.3), and the two of a macro sheet ([MS-OFFMACRO2] 2.2.1.1 and
/// 2.2.1.2).
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

/// The sheets of `listed`, each with its part among the relationships of
/// `workbook` in `package`, pushing each problem found; a problem in the
/// part's XML points to `location`.
pub(super) fn link<R: Read + Seek>(
    package: &Package<R>,
    workbook: &Workbook,
    listed: Vec<Listed>,
    location: &Location,
    diagnostics: &mut Diagnostics,
) -> Vec<Sheet> {
    let part = package.parts().get(workbook.part());
    let relationships = part.map_or(&[][..], |part| part.relationships());
    let known = part.is_some_and(|part| part.relationships_known());
    let rels = part.map(|part| rels_of(part.name().as_str()));
    // The `Id`s of one part are unique: the package model leaves out the
    // ones that are not.
    let by_id: BTreeMap<&str, &Relationship> = (relationships.iter())
        .map(|relationship| (relationship.id(), relationship))
        .collect();
    let mut names = BTreeMap::new();
    let mut parts = BTreeMap::new();
    let mut sheets = Vec::with_capacity(listed.len());
    for listed in listed {
        let part = match by_id.get(listed.relationship_id.as_str()) {
            Some(relationship) => {
                let at = Location::default()
                    .with_rels_part(rels.as_deref().unwrap_or_default())
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
        let name = listed.name;
        let folded = name.to_lowercase();
        if let Some(other) = names.get(&folded) {
            let other = sheet_name(&sheets, *other);
            let diagnostic = Diagnostic::SheetNameDuplicate {
                name: Echo::new(&name),
                other,
            };
            diagnostics.push(diagnostic, location.clone());
        } else {
            names.insert(folded, sheets.len());
        }
        if let Some(index) = part.map(|part: SheetPart| part.index) {
            if let Some(other) = parts.get(&index) {
                let other = sheet_name(&sheets, *other);
                let diagnostic = Diagnostic::SheetPartDuplicate {
                    name: Echo::new(&name),
                    other,
                };
                diagnostics.push(diagnostic, location.clone());
            } else {
                parts.insert(index, sheets.len());
            }
        }
        sheets.push(Sheet {
            name,
            sheet_id: listed.sheet_id,
            state: listed.state,
            part,
        });
    }
    sheets
}

/// The part `relationship`, the one a sheet's `r:id` names, targets, and
/// its kind; `None`, pushed at `at`, the relationship, when it is not of a
/// sheet's type or is external. Its type may be Transitional or Strict
/// whatever the workbook's kind, as the crate root finds a part.
fn target(
    relationship: &Relationship,
    at: Location,
    diagnostics: &mut Diagnostics,
) -> Option<SheetPart> {
    let rel_type = relationship.rel_type();
    let Some(&(_, kind)) = KINDS.iter().find(|(known, _)| *known == rel_type) else {
        let rel_type = Echo::new(rel_type);
        diagnostics.push(Diagnostic::SheetRelationshipTypeInvalid { rel_type }, at);
        return None;
    };
    match relationship.target() {
        Target::Internal(index) => Some(SheetPart {
            index: *index,
            kind,
        }),
        Target::External(_) => {
            let relationship_id = Echo::new(relationship.id());
            let diagnostic = Diagnostic::SheetRelationshipMissing { relationship_id };
            diagnostics.push(diagnostic, at);
            None
        }
    }
}

/// The name of the sheet at `index` of `sheets`, for a diagnostic.
fn sheet_name(sheets: &[Sheet], index: usize) -> Echo {
    Echo::new(sheets.get(index).map_or("", |sheet| sheet.name.as_str()))
}

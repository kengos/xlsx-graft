//! [`Workbook`]: the workbook part of a package, found by the package's
//! `officeDocument` relationship, and the parts it relates to by type.

use std::collections::BTreeSet;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};
use xlsx_graft_package::{OFFICE_DOCUMENT, Package, Presence, Relationship, Target};

/// The package's own Relationships part, where `officeDocument` is.
const PACKAGE_RELATIONSHIPS: &str = "/_rels/.rels";

/// The workbook part of a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Workbook {
    part: usize,
}

impl Workbook {
    /// The workbook part `package`'s internal `officeDocument`
    /// relationship targets, as the crate root says; `None` when there is
    /// none, which the package model reported already as
    /// `main_document_missing` unless its relationships were not all read,
    /// or when two target different parts (`relationship_type_duplicate`,
    /// pushed here).
    pub fn find<R: Read + Seek>(
        package: &Package<R>,
        diagnostics: &mut Diagnostics,
    ) -> Option<Self> {
        let relationships = package.relationships();
        let known = package.relationships_known();
        let rels = PACKAGE_RELATIONSHIPS;
        match single(relationships, known, &OFFICE_DOCUMENT, rels, diagnostics) {
            Presence::Part(part) => Some(Self { part }),
            Presence::Absent | Presence::Unknown => None,
        }
    }

    /// The index of the workbook part in
    /// [`Package::parts`](xlsx_graft_package::Package::parts).
    #[must_use]
    pub fn part(&self) -> usize {
        self.part
    }

    /// The part the workbook's internal relationship of one of `types` (the
    /// Transitional and the Strict type of one part type) targets, as the
    /// crate root says. Call it once per part type: a duplicate is pushed
    /// every time.
    pub fn related<R: Read + Seek>(
        &self,
        package: &Package<R>,
        types: &[&str],
        diagnostics: &mut Diagnostics,
    ) -> Presence {
        let Some(part) = package.parts().get(self.part) else {
            return Presence::Unknown;
        };
        let name = part.name().as_str();
        let (folder, file) = name.rsplit_once('/').unwrap_or_default();
        let rels = format!("{folder}/_rels/{file}.rels");
        let known = part.relationships_known();
        single(part.relationships(), known, types, &rels, diagnostics)
    }
}

/// The one part the internal relationships of `types` among
/// `relationships`, declared in `rels`, target; whether there are no
/// others is `known`.
fn single(
    relationships: &[Relationship],
    known: bool,
    types: &[&str],
    rels: &str,
    diagnostics: &mut Diagnostics,
) -> Presence {
    let mut targets = BTreeSet::new();
    let mut first = None;
    for relationship in relationships {
        let Target::Internal(part) = relationship.target() else {
            continue;
        };
        if !types.contains(&relationship.rel_type()) {
            continue;
        }
        first = first.or(Some(*part));
        targets.insert(*part);
        if targets.len() == 2 {
            let location = Location::default()
                .with_rels_part(rels)
                .with_relationship_id(relationship.id());
            let rel_type = Echo::new(relationship.rel_type());
            diagnostics.push(Diagnostic::RelationshipTypeDuplicate { rel_type }, location);
            return Presence::Unknown;
        }
    }
    match first {
        Some(part) => Presence::Part(part),
        None if known => Presence::Absent,
        None => Presence::Unknown,
    }
}

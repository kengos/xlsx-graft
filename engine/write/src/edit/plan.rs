//! [`Plan`]: what the changes of an edit come to in the entries of the
//! zip, the `Override`s and the Relationships parts, worked out from the
//! table of names (`edit/names.rs`) before any entry is read.

use super::PackageEdit;
use super::link::Pending;
use super::names::{Names, Origin};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek};
use xlsx_graft_package::{Package, PartName, RelationshipSource};

/// The media type of a Relationships part (ECMA-376-2:2021 §6.5.2.1).
const RELATIONSHIPS: &str = "application/vnd.openxmlformats-package.relationships+xml";

/// A relationship to write.
#[derive(Debug)]
pub(super) struct NewRelationship {
    pub(super) id: String,
    pub(super) rel_type: String,
    pub(super) target: String,
}

/// What changes in one Relationships part.
#[derive(Debug, Default)]
pub(super) struct RelsChange {
    /// The `Id`s of the relationships to leave out.
    pub(super) drop: BTreeSet<String>,
    /// The relationships to add, after the others.
    pub(super) add: Vec<NewRelationship>,
}

/// The entries of the zip that change and how.
#[derive(Debug, Default)]
pub(super) struct Plan {
    /// The entries to leave out: the parts vacated, which are the parts
    /// removed and their Relationships parts.
    pub(super) removed: BTreeSet<usize>,
    /// The Relationships parts that exist, by entry, with what changes.
    pub(super) rels: BTreeMap<usize, RelsChange>,
    /// The Relationships parts to make, for a source that has none: the
    /// part's name and the relationships to write, in the order of the
    /// first link to each source.
    pub(super) new_rels: Vec<(PartName, Vec<NewRelationship>)>,
    /// The keys of the names of the `Override`s to leave out: every name the
    /// edit occupied or vacated.
    pub(super) drop_overrides: BTreeSet<String>,
    /// The `Override`s to add: a part name and its media type, for each part
    /// added and each Relationships part to make.
    pub(super) add_overrides: Vec<(String, String)>,
}

impl Plan {
    /// Whether the content types stream changes.
    pub(super) fn changes_content_types(&self) -> bool {
        !(self.drop_overrides.is_empty() && self.add_overrides.is_empty())
    }

    /// The plan for `edit`.
    pub(super) fn build<R: Read + Seek>(edit: &PackageEdit<'_, R>) -> Self {
        let (package, names) = (&edit.package, &edit.names);
        let mut plan = Self {
            removed: names.vacated().clone(),
            drop_overrides: names.touched().clone(),
            ..Self::default()
        };
        for part in (package.parts().iter()).filter(|p| names.vacated().contains(&p.entry())) {
            for incoming in part.incoming() {
                plan.drop_relationship(package, names, incoming.source, incoming.index);
            }
        }
        for pending in &edit.links {
            plan.add_relationship(names, pending);
        }
        for added in &edit.added {
            let name = added.name.to_string();
            plan.add_overrides
                .push((name, added.content_type.as_str().to_owned()));
        }
        for (name, _) in &plan.new_rels {
            let overrides = (name.to_string(), RELATIONSHIPS.to_owned());
            plan.add_overrides.push(overrides);
        }
        plan
    }

    /// Leaves out the relationship at `index` of `source` from the
    /// Relationships part of `source`, if the written package holds one: the
    /// relationship targets a part that is vacated.
    fn drop_relationship<R: Read + Seek>(
        &mut self,
        package: &Package<R>,
        names: &Names,
        source: RelationshipSource,
        index: usize,
    ) {
        let (rels, relationships) = match source {
            RelationshipSource::Package => {
                (PartName::package_relationships(), package.relationships())
            }
            RelationshipSource::Part(at) => {
                let Some(part) = package.parts().get(at) else { return };
                (part.name().relationships_part(), part.relationships())
            }
        };
        let id = relationships
            .get(index)
            .map(|relationship| relationship.id());
        let entry = match names.get(&rels) {
            Some(Origin::Original(entry)) => Some(*entry),
            _ => None,
        };
        id.zip(entry).into_iter().for_each(|(id, entry)| {
            let change = self.rels.entry(entry).or_default();
            change.drop.insert(id.to_owned());
        });
    }

    /// Adds the relationship `pending` to the Relationships part of its
    /// source: the one the package has, or the one to make. When the written
    /// package holds neither, because the source is vacated, nothing is
    /// written.
    fn add_relationship(&mut self, names: &Names, pending: &Pending) {
        let rels = pending.source.rels_part();
        let new = NewRelationship {
            id: pending.id.clone(),
            rel_type: pending.rel_type.clone(),
            target: pending.target.clone(),
        };
        match names.get(&rels) {
            Some(Origin::Original(entry)) => {
                self.rels.entry(*entry).or_default().add.push(new);
            }
            Some(Origin::NewRelationships(_)) => {
                let at = (self.new_rels.iter()).position(|(name, _)| name.key() == rels.key());
                match at.and_then(|at| self.new_rels.get_mut(at)) {
                    Some((_, adds)) => adds.push(new),
                    None => self.new_rels.push((rels, vec![new])),
                }
            }
            _ => {}
        }
    }
}

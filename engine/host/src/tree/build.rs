//! Building a [`Tree`] from what [`Package`] reads: its parts, the entries of
//! its zip and its relationships.

use super::{Linked, Tree, TreePart, TreeRelationship, TreeTarget};
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::Echo;
use xlsx_graft_package::{Entry, Package, Part, Relationship, Target};

impl Tree {
    /// The tree of a package that holds `parts`, whose zip has `entries`,
    /// and whose own relationships are `relationships`; `complete` is
    /// [`Package::graph_complete`] and `relationships_known` is
    /// [`Package::relationships_known`].
    ///
    /// Not generic over the package's reader, so that no copy of the
    /// function is made per reader type.
    fn of(
        parts: &[Part],
        entries: &[Entry],
        relationships: &[Relationship],
        complete: bool,
        relationships_known: bool,
    ) -> Self {
        // Each package part's index in `Tree::parts`, `None` for a
        // Relationships part, which is not listed.
        let mut listed = 0_usize;
        let indexes: Vec<Option<usize>> = (parts.iter())
            .map(|part| {
                let index = (!part.name().is_relationships()).then_some(listed);
                listed = listed.saturating_add(usize::from(index.is_some()));
                index
            })
            .collect();
        Self {
            complete,
            relationships_known,
            relationships: tree_relationships(relationships, parts, &indexes),
            parts: (parts.iter())
                .filter(|part| !part.name().is_relationships())
                .map(|part| TreePart::of(parts, &indexes, entries, part, complete))
                .collect(),
        }
    }

    /// The tree of `package`.
    pub(super) fn from_package<R: Read + Seek>(package: &Package<R>) -> Self {
        let entries = package.archive().entries();
        let relationships = package.relationships();
        Self::of(
            package.parts(),
            entries,
            relationships,
            package.graph_complete(),
            package.relationships_known(),
        )
    }
}

impl TreePart {
    fn of(
        parts: &[Part],
        indexes: &[Option<usize>],
        entries: &[Entry],
        part: &Part,
        complete: bool,
    ) -> Self {
        let entry = entries.get(part.entry());
        let referenced = if !part.incoming().is_empty() {
            Some(true)
        } else {
            complete.then_some(false)
        };
        Self {
            name: Echo::new(part.name().as_str()),
            content_type: part.content_type().map(|t| Echo::new(t.as_str())),
            content_type_known: !part.content_type_unknown(),
            size: entry.map_or(0, |e| e.size()),
            compressed_size: entry.map_or(0, |e| e.compressed_size()),
            readable: entry.is_some_and(|e| e.is_readable()),
            referenced,
            relationships_known: part.relationships_known(),
            relationships: tree_relationships(part.relationships(), parts, indexes),
        }
    }
}

/// `list` as tree relationships; one whose target is not in `parts`, or is a
/// Relationships part (so is not in `indexes`), is left out (the package
/// keeps none).
fn tree_relationships(
    list: &[Relationship],
    parts: &[Part],
    indexes: &[Option<usize>],
) -> Vec<TreeRelationship> {
    list.iter()
        .filter_map(|relationship| {
            let target = match relationship.target() {
                Target::Internal(index) => TreeTarget(Linked::Part {
                    index: indexes.get(*index).copied().flatten()?,
                    name: Echo::new(parts.get(*index)?.name().as_str()),
                }),
                Target::External(text) => TreeTarget(Linked::External(Echo::new(text))),
            };
            Some(TreeRelationship {
                id: Echo::new(relationship.id()),
                rel_type: Echo::new(relationship.rel_type()),
                target,
            })
        })
        .collect()
}

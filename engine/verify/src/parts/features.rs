//! The features of the compatibility table a part holds. A part has a
//! role by its media type or by the type of a relationship that targets
//! it, either one, as for the kinds of content (`kinds.rs`); a feature
//! comes from the role itself, or from an element in the one part role
//! [MS-XLSX] extends with it, so the same element in a part of another
//! role (a custom XML part's user data) counts for nothing:
//!
//! A part role is a feature by its presence, as [MS-XLSX] defines it: a
//! Threaded Comments part says there is at least one threaded comment, and
//! a package holds a Slicer Cache or Timeline Cache part per cache the
//! workbook part lists, so what one holds is not read for it.
//!
//! - `threaded_comment`: a Threaded Comments part (§2.1.17).
//! - `slicer`: a Slicer Cache part (§2.1.4); `table_slicer` instead when it
//!   holds a `tableSlicerCache` (§2.4.48), which only a table's slicer
//!   cache holds. A Slicers part (§2.1.5) is a sheet's view of a cache and
//!   does not tell the one from the other, so it counts for neither.
//! - `timeline`: a Timeline Cache part (§2.1.7), its relationship type as
//!   §2.1.7 writes it (dated 2010); the Timelines part, a sheet's view of
//!   it, counts no more than a Slicers part does.
//! - `data_model`: a Data Model part (§2.1.6), by its media type alone:
//!   §2.1.6 gives it no relationship type.
//! - `sparkline`: a `sparkline` element (§2.6.9) in a worksheet (ECMA-376-1
//!   §12.3.24), whose extension list §2.2.4.12 extends with its groups: a
//!   group that holds no sparkline shows none.
//! - `dynamic_array`: a `dynamicArrayProperties` element (§2.4.87) whose
//!   `fDynamic` is true, anywhere in a Metadata part (ECMA-376-1
//!   §12.3.10); Excel writes it in the future metadata `XLDAPR`. It says
//!   the workbook has formulas Excel treats as dynamic arrays, not that
//!   one of them spills over more than one cell: a single-cell one counts
//!   as well.
//! - `cell_image`: a rich value structure (`s`, §2.6.180) of the type
//!   `_localImage` (§2.3.6.1.8), matched in its own letter case as the
//!   string it is, in a Rich Value Structure part (§2.1.11).
//!
//! When every Relationships part was read whole
//! ([`Package::graph_complete`]), only a
//! part the package reaches through its relationships, followed from
//! `/_rels/.rels` with a work list, holds any, and its roles come from the
//! relationships followed: a part no relationship targets
//! (`part_unreferenced`), or one only such a part targets, is never
//! loaded. When some may be left out, every part counts, its roles coming
//! from its media type and every relationship read, since the one that
//! reaches it, or names its role, may be among those left out.
//!
//! The metadata part is not checked against the cells: dynamic array
//! metadata no cell's `cm` refers to counts all the same.
//!
//! Media types are matched by their lowered essence and relationship types
//! with ASCII letter case aside; elements by their namespace and local
//! name, the namespace being the one [MS-XLSX] gives each, so an element
//! of another vocabulary that shares a local name (`s`) counts for nothing.

use super::kinds::{ECMA, after};
use crate::collect::attribute;
use std::collections::BTreeSet;
use std::io::{Read, Seek};
use xlsx_graft_compat::Feature;
use xlsx_graft_package::{Package, Target};
use xlsx_graft_xml::events::BytesStart;

/// The role of a part that holds a feature, or an element of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    /// A worksheet (ECMA-376-1 §12.3.24).
    Worksheet,
    /// The Metadata part (ECMA-376-1 §12.3.10).
    Metadata,
    /// A Rich Value Structure part (§2.1.11).
    RichValueStructure,
    /// A Slicer Cache part (§2.1.4).
    SlicerCache,
    /// A Timeline Cache part (§2.1.7).
    TimelineCache,
    /// A Threaded Comments part (§2.1.17).
    ThreadedComments,
    /// The Data Model part (§2.1.6).
    DataModel,
}

/// The media types of the roles, lowered.
const MEDIA_TYPES: [(&str, Role); 7] = [
    (
        "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
        Role::Worksheet,
    ),
    (
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheetmetadata+xml",
        Role::Metadata,
    ),
    (
        "application/vnd.ms-excel.rdrichvaluestructure+xml",
        Role::RichValueStructure,
    ),
    (
        "application/vnd.ms-excel.slicercache+xml",
        Role::SlicerCache,
    ),
    (
        "application/vnd.ms-excel.timelinecache+xml",
        Role::TimelineCache,
    ),
    (
        "application/vnd.ms-excel.threadedcomments+xml",
        Role::ThreadedComments,
    ),
    (
        "application/vnd.openxmlformats-officedocument.model+data",
        Role::DataModel,
    ),
];

/// The roles ECMA-376 names a relationship type for, by the name after
/// its Transitional or Strict namespace.
const ECMA_RELATIONSHIPS: [(&str, Role); 2] = [
    ("worksheet", Role::Worksheet),
    ("sheetMetadata", Role::Metadata),
];

/// The relationship types [MS-XLSX] defines for the roles.
const RELATIONSHIPS: [(&str, Role); 4] = [
    (
        "http://schemas.microsoft.com/office/2017/06/relationships/rdRichValueStructure",
        Role::RichValueStructure,
    ),
    (
        "http://schemas.microsoft.com/office/2007/relationships/slicerCache",
        Role::SlicerCache,
    ),
    (
        "http://schemas.microsoft.com/office/2010/relationships/TimelineCache",
        Role::TimelineCache,
    ),
    (
        "http://schemas.microsoft.com/office/2017/10/relationships/threadedComment",
        Role::ThreadedComments,
    ),
];

/// The namespace of `sparkline` (x14).
const X14: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
/// The namespace of `tableSlicerCache` (x15).
const X15: &str = "http://schemas.microsoft.com/office/spreadsheetml/2010/11/main";
/// The namespace of `dynamicArrayProperties`.
const DYNAMIC_ARRAY: &str = "http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray";
/// The namespace of a rich value structure.
const RICH_DATA: &str = "http://schemas.microsoft.com/office/spreadsheetml/2017/richdata";

/// The roles of each part of a package, by its index in its parts, and
/// whether the part may be loaded.
pub(super) struct Roles {
    roles: Vec<BTreeSet<Role>>,
    reached: Vec<bool>,
}

impl Roles {
    /// The roles of the parts of `package`, as this module's header says.
    pub(super) fn of<R: Read + Seek>(package: &Package<R>) -> Self {
        let parts = package.parts();
        let mut roles: Vec<BTreeSet<Role>> = (parts.iter())
            .map(|part| {
                let essence = part.content_type().map(|t| t.essence());
                essence.and_then(media_type).into_iter().collect()
            })
            .collect();
        let complete = package.graph_complete();
        let mut reached = vec![!complete; parts.len()];
        let mut next = vec![package.relationships()];
        if !complete {
            next.extend(parts.iter().map(|part| part.relationships()));
        }
        while let Some(relationships) = next.pop() {
            for relationship in relationships {
                let &Target::Internal(target) = relationship.target() else {
                    continue;
                };
                if let Some(set) = roles.get_mut(target) {
                    set.extend(relationship_type(relationship.rel_type()));
                }
                if let Some(seen) = reached.get_mut(target)
                    && !*seen
                {
                    *seen = true;
                    next.extend(parts.get(target).map(|part| part.relationships()));
                }
            }
        }
        Self { roles, reached }
    }

    /// The features the part at `index` holds, the elements it was read to
    /// hold being `shown`.
    pub(super) fn features(&self, index: usize, shown: BTreeSet<Feature>) -> BTreeSet<Feature> {
        let (Some(true), Some(roles)) = (self.reached.get(index), self.roles.get(index)) else {
            return BTreeSet::new();
        };
        features(roles, shown)
    }
}

/// The features a part of `roles` holds, its elements showing `shown`.
fn features(roles: &BTreeSet<Role>, shown: BTreeSet<Feature>) -> BTreeSet<Feature> {
    let mut features: BTreeSet<Feature> = (roles.iter())
        .filter_map(|role| match role {
            Role::ThreadedComments => Some(Feature::ThreadedComment),
            Role::SlicerCache => Some(Feature::Slicer),
            Role::TimelineCache => Some(Feature::Timeline),
            Role::DataModel => Some(Feature::DataModel),
            Role::Worksheet | Role::Metadata | Role::RichValueStructure => None,
        })
        .collect();
    features.extend((shown.into_iter()).filter(|f| home(*f).is_some_and(|r| roles.contains(&r))));
    // `home` keeps `table_slicer` for a slicer cache alone.
    if features.contains(&Feature::TableSlicer) {
        features.remove(&Feature::Slicer);
    }
    features
}

/// The role of the part an element that shows `feature` counts in.
fn home(feature: Feature) -> Option<Role> {
    match feature {
        Feature::Sparkline => Some(Role::Worksheet),
        Feature::DynamicArray => Some(Role::Metadata),
        Feature::CellImage => Some(Role::RichValueStructure),
        Feature::TableSlicer => Some(Role::SlicerCache),
        _ => None,
    }
}

/// The role of a part of the media type `essence`, lowered.
fn media_type(essence: &str) -> Option<Role> {
    (MEDIA_TYPES.iter())
        .find(|(name, _)| *name == essence)
        .map(|&(_, role)| role)
}

/// The role of the target of a relationship of `rel_type`.
fn relationship_type(rel_type: &str) -> Option<Role> {
    if let Some(name) = ECMA.iter().find_map(|prefix| after(rel_type, prefix)) {
        return (ECMA_RELATIONSHIPS.iter())
            .find(|(n, _)| name.eq_ignore_ascii_case(n))
            .map(|&(_, role)| role);
    }
    (RELATIONSHIPS.iter())
        .find(|(name, _)| rel_type.eq_ignore_ascii_case(name))
        .map(|&(_, role)| role)
}

/// The feature the element `start`, of the local name `local` in the
/// namespace `namespace`, makes its part hold.
pub(super) fn element(namespace: &str, local: &str, start: &BytesStart<'_>) -> Option<Feature> {
    match (namespace, local) {
        (X14, "sparkline") => Some(Feature::Sparkline),
        (X15, "tableSlicerCache") => Some(Feature::TableSlicer),
        (DYNAMIC_ARRAY, "dynamicArrayProperties") => {
            let on = attribute(start, "fDynamic").is_some_and(|v| matches!(v.trim(), "1" | "true"));
            on.then_some(Feature::DynamicArray)
        }
        (RICH_DATA, "s") => {
            let local_image = attribute(start, "t").is_some_and(|t| t == "_localImage");
            local_image.then_some(Feature::CellImage)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;

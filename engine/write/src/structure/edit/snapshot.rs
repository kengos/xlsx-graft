//! [`Snapshot`]: what `open` copies from the package for `apply`, so the
//! edit holds no borrow of it: the workbook as the model read it, the names
//! the package holds, the graph of relationships, and the calcChain and
//! app parts.
//!
//! The package is read by `open` and handed to a `PackageEdit` by the
//! caller, which owns it from then on, and `apply` only gets that edit.

use super::app;
use super::ns::{APP_TYPES, RELATIONSHIPS};
use crate::structure::model::Workbook;
use crate::{WriteError, WriteLimits};
use std::collections::BTreeSet;
use std::io::{Read, Seek, sink};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Package, PartName, Relationship, RelationshipSource, Target};

/// One part of the package, for [`cleanup`](super::cleanup).
pub(super) struct Node {
    /// The part's name.
    pub(super) name: PartName,
    /// The parts its relationships target, by index of the package's parts.
    pub(super) targets: Vec<usize>,
    /// The parts whose relationships target it, `None` for the package.
    pub(super) incoming: Vec<Option<usize>>,
}

/// The app part, `docProps/app.xml` or as the package names it.
pub(super) struct App {
    /// Its name.
    pub(super) name: PartName,
    /// Whether it has `TitlesOfParts` or `HeadingPairs`: only then is it
    /// rewritten.
    pub(super) has_titles: bool,
}

/// What `open` copied from the package.
pub(super) struct Snapshot {
    /// The workbook part, as the model read it.
    pub(super) workbook: Workbook,
    /// The workbook part's name.
    pub(super) workbook_name: PartName,
    /// The key of each name of the package that a part added may not take:
    /// the parts' and the entries refused for their names.
    pub(super) taken: BTreeSet<String>,
    /// Each part, by index of the package's parts.
    pub(super) graph: Vec<Node>,
    /// Whether the package read every Relationships part whole, so `graph`
    /// holds every relationship.
    pub(super) graph_complete: bool,
    /// The index of the calcChain part.
    pub(super) calc_chain: Option<usize>,
    /// The app part, when it can be read.
    pub(super) app: Option<App>,
    /// The limits the rewriters read under.
    pub(super) limits: WriteLimits,
}

impl Snapshot {
    /// Copies from `package` what `apply` needs, `workbook` being its
    /// workbook as the model read it.
    pub(super) fn read<R: Read + Seek>(
        package: &mut Package<R>,
        workbook: Workbook,
        limits: &WriteLimits,
    ) -> Result<Self, WriteError> {
        let parts = package.parts();
        let workbook_name = (parts.get(workbook.part()))
            .map(|part| part.name().clone())
            .unwrap_or_else(PartName::package_relationships);
        let taken = (parts.iter().map(|part| part.name().key()))
            .chain(package.unknown_keys())
            .map(str::to_owned)
            .collect();
        let graph = parts.iter().map(node).collect();
        let graph_complete = package.graph_complete();
        let rels = workbook.relationships();
        let calc_chain =
            (RELATIONSHIPS.iter()).find_map(|base| internal(rels, &format!("{base}/calcChain")));
        let app = (APP_TYPES.iter()).find_map(|kind| internal(package.relationships(), kind));
        let app = match app {
            Some(index) => read_app(package, index, limits)?,
            None => None,
        };
        Ok(Self {
            workbook,
            workbook_name,
            taken,
            graph,
            graph_complete,
            calc_chain,
            app,
            limits: *limits,
        })
    }
}

/// The node of `part`.
fn node(part: &xlsx_graft_package::Part) -> Node {
    let targets = part.relationships().iter().filter_map(index).collect();
    let incoming = (part.incoming().iter())
        .map(|incoming| match incoming.source {
            RelationshipSource::Package => None,
            RelationshipSource::Part(index) => Some(index),
        })
        .collect();
    Node {
        name: part.name().clone(),
        targets,
        incoming,
    }
}

/// The part the first relationship of `rel_type` among `relationships`
/// targets inside the package.
fn internal(relationships: &[Relationship], rel_type: &str) -> Option<usize> {
    (relationships.iter())
        .filter(|relationship| relationship.rel_type() == rel_type)
        .find_map(index)
}

/// The part `relationship` targets inside the package.
fn index(relationship: &Relationship) -> Option<usize> {
    match relationship.target() {
        Target::Internal(index) => Some(*index),
        Target::External(_) => None,
    }
}

/// The app part at `index`: `None` when it cannot be read as XML (the
/// diagnostics of reading it are not kept), which leaves it as it is.
fn read_app<R: Read + Seek>(
    package: &mut Package<R>,
    index: usize,
    limits: &WriteLimits,
) -> Result<Option<App>, WriteError> {
    let name = package.parts().get(index).map(|part| part.name().clone());
    let bytes = package.read_xml_part(index, limits.xml.max_bytes, &mut Diagnostics::new())?;
    let dropped =
        bytes.and_then(|bytes| app::rewrite(&mut bytes.as_slice(), &mut sink(), &limits.xml).ok());
    Ok(name
        .zip(dropped)
        .map(|(name, has_titles)| App { name, has_titles }))
}

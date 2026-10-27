//! [`read`]: a Relationships part's XML (opc-relationships.xsd), as the
//! relationships it declares, each `Id` unique.

use crate::Limits;
use crate::schema::{self, ChildShape, Element, Shape};
use std::collections::{BTreeMap, BTreeSet};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};

/// The part's shape: `Relationships` holding `Relationship`, which may hold
/// text (`CT_Relationship` extends `xsd:string`).
const SHAPE: Shape = Shape {
    namespace: "http://schemas.openxmlformats.org/package/2006/relationships",
    root: "Relationships",
    children: &[ChildShape {
        name: "Relationship",
        attributes: &["Id", "Type", "Target", "TargetMode"],
        text: true,
    }],
};

/// A relationship as declared, its target unresolved.
#[derive(Debug)]
pub(crate) struct Raw {
    pub(crate) id: String,
    pub(crate) rel_type: String,
    pub(crate) target: String,
    /// Whether its `TargetMode` is `External`.
    pub(crate) external: bool,
}

/// Reads the part's `bytes`, pushing what is wrong with it: the XML at
/// `location`, a duplicate `Id` at `rels` with the `Id`. `None` when it
/// cannot be read to its end or declares more relationships than the limit;
/// else the relationships, and whether none was left out.
pub(crate) fn read(
    bytes: &[u8],
    limits: &Limits,
    location: &Location,
    rels: &str,
    diagnostics: &mut Diagnostics,
) -> Option<(Vec<Raw>, bool)> {
    let mut raws = Vec::new();
    let mut count: u64 = 0;
    let max = limits.max_relationships.value;
    let read = schema::read(
        bytes,
        &limits.xml,
        &SHAPE,
        location,
        diagnostics,
        |element, diagnostics| {
            count = count.saturating_add(1);
            if count > max {
                return;
            }
            match raw(&element) {
                Ok(raw) => raws.push(raw),
                Err(diagnostic) => diagnostics.push(diagnostic, location.clone()),
            }
        },
    );
    let every_child = read?;
    if count > max {
        let limit = limits.max_relationships;
        diagnostics.push(
            Diagnostic::RelationshipsTooMany { count, limit },
            location.clone(),
        );
        return None;
    }
    // Any relationship refused here, by its attributes or a shared `Id`,
    // leaves the part with fewer than it declares.
    let unique = unique(raws, rels, diagnostics);
    let whole = every_child && u64::try_from(unique.len()).is_ok_and(|len| len == count);
    Some((unique, whole))
}

/// The relationship `element` declares, or the `xml_schema_invalid` that
/// refuses it. `Id`, `Type` and `Target` are collapsed as their schema
/// types (`xsd:ID`, `xsd:anyURI`) have it.
fn raw(element: &Element) -> Result<Raw, Diagnostic> {
    let (id, rel_type, target, mode) = match element.values.as_slice() {
        [Some(id), Some(rel_type), Some(target), mode] => (id, rel_type, target, mode),
        values => return Err(schema::missing_attribute(&SHAPE, element, values)),
    };
    let external = match mode.as_deref() {
        None | Some("Internal") => false,
        Some("External") => true,
        Some(_) => return Err(schema::invalid_value(element.offset, "TargetMode")),
    };
    let id = schema::collapse(id);
    if !is_ncname(&id) {
        return Err(schema::invalid_value(element.offset, "Id"));
    }
    Ok(Raw {
        id,
        rel_type: schema::collapse(rel_type),
        target: schema::collapse(target),
        external,
    })
}

/// Whether `id` is an `xsd:ID`: an XML name without a colon. Non-ASCII
/// characters are admitted wherever they stand, a superset of the
/// `NCName` production.
fn is_ncname(id: &str) -> bool {
    let mut chars = id.chars();
    let start = |c: char| c.is_ascii_alphabetic() || c == '_' || !c.is_ascii();
    chars.next().is_some_and(start)
        && chars.all(|c| start(c) || c.is_ascii_digit() || c == '-' || c == '.')
}

/// `raws` without those whose `Id` another one has, each of which after the
/// first is reported.
fn unique(raws: Vec<Raw>, rels: &str, diagnostics: &mut Diagnostics) -> Vec<Raw> {
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for raw in &raws {
        let count = counts.entry(&raw.id).or_default();
        *count = count.saturating_add(1);
        if *count > 1 {
            let location = Location::default()
                .with_rels_part(rels)
                .with_relationship_id(&raw.id);
            diagnostics.push(Diagnostic::RelationshipIdDuplicate {}, location);
        }
    }
    let repeated: BTreeSet<String> = (counts.into_iter())
        .filter(|&(_, count)| count > 1)
        .map(|(id, _)| id.to_owned())
        .collect();
    raws.into_iter()
        .filter(|raw| !repeated.contains(&raw.id))
        .collect()
}

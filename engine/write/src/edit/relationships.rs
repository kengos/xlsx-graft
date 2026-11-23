//! [`rewrite`] and [`create`]: a Relationships part written again without
//! the relationships to a part removed and with those to a part added, and
//! a new one (ECMA-376-2:2021 §6.5).

use super::children::{self, Children, attribute};
use super::plan::NewRelationship;
use crate::error::WriteError;
use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use xlsx_graft_package::collapse;
use xlsx_graft_xml::XmlLimits;
use xlsx_graft_xml::events::BytesStart;

/// The Relationships part's children: `Relationship`, in this namespace.
const CHILDREN: Children<'static> = Children {
    namespace: "http://schemas.openxmlformats.org/package/2006/relationships",
    name: "Relationship",
};

/// A Relationships part with no relationship, as Excel writes one: the
/// declaration, a CR LF, then the root. [`create`] adds the relationships to
/// it.
const EMPTY: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    "\r\n",
    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#,
);

/// Copies `original` to `out`, leaving out each `Relationship` whose `Id`
/// is in `drop`, and writing each of `add` after the others.
///
/// # Errors
///
/// As [`children::rewrite`].
pub(super) fn rewrite(
    original: &mut dyn BufRead,
    out: &mut dyn Write,
    limits: &XmlLimits,
    drop: &BTreeSet<String>,
    add: &[NewRelationship],
) -> Result<(), WriteError> {
    let attributes: Vec<Vec<(&str, &str)>> = (add.iter())
        .map(|new| {
            vec![
                ("Id", new.id.as_str()),
                ("Type", new.rel_type.as_str()),
                ("Target", new.target.as_str()),
            ]
        })
        .collect();
    let leave_out = |start: &BytesStart<'_>| {
        attribute(start, "Id").is_some_and(|id| drop.contains(&collapse(&id)))
    };
    children::rewrite(original, out, limits, &CHILDREN, leave_out, &attributes)
}

/// Writes to `out` a new Relationships part of the relationships `add`.
///
/// # Errors
///
/// As [`children::rewrite`].
pub(super) fn create(
    out: &mut dyn Write,
    limits: &XmlLimits,
    add: &[NewRelationship],
) -> Result<(), WriteError> {
    rewrite(&mut EMPTY.as_bytes(), out, limits, &BTreeSet::new(), add)
}

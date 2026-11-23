//! [`rewrite`]: the content types stream written again without the
//! `Override`s of the parts removed, and with those of the parts added
//! (ECMA-376-2:2021 §7.2.3).

use super::children::{self, Children, attribute};
use crate::error::WriteError;
use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use xlsx_graft_package::override_key;
use xlsx_graft_xml::XmlLimits;
use xlsx_graft_xml::events::BytesStart;

/// The `Override` children of the content types stream's root, in its
/// namespace (§7.2.3.1); a `Default` is copied as it is.
const CHILDREN: Children<'static> = Children {
    namespace: "http://schemas.openxmlformats.org/package/2006/content-types",
    name: "Override",
};

/// Copies `original` to `out`, leaving out each `Override` whose part name
/// key is in `drop`, and writing an `Override` for each of `add` (a
/// part name and its media type) after the others.
///
/// An added part gets an `Override` even when a `Default` for its extension
/// gives the same type: an `Override` is always right (§7.2.3.5).
///
/// # Errors
///
/// As [`children::rewrite`].
pub(super) fn rewrite(
    original: &mut dyn BufRead,
    out: &mut dyn Write,
    limits: &XmlLimits,
    drop: &BTreeSet<String>,
    add: &[(String, String)],
) -> Result<(), WriteError> {
    let attributes: Vec<Vec<(&str, &str)>> = (add.iter())
        .map(|(name, content_type)| {
            vec![("PartName", name.as_str()), ("ContentType", content_type)]
        })
        .collect();
    children::rewrite(
        original,
        out,
        limits,
        &CHILDREN,
        |start| names(start, drop),
        &attributes,
    )
}

/// Whether the `Override` `start` is for a name in `drop`: the key of its
/// `PartName` as the package reads it ([`override_key`]), so an `Override`
/// whose value is no part name is left out too when a name that is the same
/// as it is occupied. An `Override` with no `PartName` is not.
fn names(start: &BytesStart<'_>, drop: &BTreeSet<String>) -> bool {
    let value = attribute(start, "PartName");
    value.is_some_and(|value| drop.contains(&override_key(&value)))
}

//! The packages the model tests read: a minimal workbook and the items to
//! build one from, and how a test reads one and sees what was found.

use super::Reader;
use super::builder::{Item, Zip};
use crate::{Limits, Package};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};

/// The content types namespace.
pub(super) const TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
/// The relationships namespace.
pub(super) const RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
/// The Relationships media type.
pub(super) const RELS_TYPE: &str = "application/vnd.openxmlformats-package.relationships+xml";
/// The `officeDocument` relationship type.
pub(super) const OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// The `Default`s every test package declares: `rels` and `xml`.
pub(super) const DEFAULTS: &str = concat!(
    r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
    r#"<Default Extension="xml" ContentType="application/xml"/>"#,
);

/// A package read by the tests.
pub(super) type Model = Package<Reader>;

/// What reading a package found: each diagnostic, with where it points.
pub(super) type Seen = Vec<(Diagnostic, String)>;

/// The content types stream, `body` inside its `Types`.
pub(super) fn types(body: &str) -> Item {
    let xml = format!(r#"<?xml version="1.0"?><Types xmlns="{TYPES_NS}">{body}</Types>"#);
    Item::stored("[Content_Types].xml", xml.as_bytes())
}

/// The Relationships part `name`, `body` inside its `Relationships`.
pub(super) fn rels(name: &str, body: &str) -> Item {
    let xml =
        format!(r#"<!-- relationships --><Relationships xmlns="{RELS_NS}">{body}</Relationships>"#);
    Item::stored(name, xml.as_bytes())
}

/// An internal relationship `id` of a made-up type to `target`.
pub(super) fn rel(id: &str, target: &str) -> String {
    format!(r#"<Relationship Id="{id}" Type="urn:t" Target="{target}"/>"#)
}

/// An XML part `name` holding an empty element.
pub(super) fn part(name: &str) -> Item {
    Item::stored(name, b"<a/>")
}

/// The package relationships of a workbook at `xl/workbook.xml`, then
/// `more`.
pub(super) fn package_rels(more: &str) -> Item {
    let main =
        format!(r#"<Relationship Id="rId1" Type="{OFFICE_DOCUMENT}" Target="xl/workbook.xml"/>"#);
    rels("_rels/.rels", &format!("{main}{more}"))
}

/// The smallest package that reads without a diagnostic: the content types
/// stream, the package relationships and the workbook.
pub(super) fn minimal() -> Vec<Item> {
    vec![types(DEFAULTS), package_rels(""), part("xl/workbook.xml")]
}

/// `minimal` with `more` items after it.
pub(super) fn with(more: Vec<Item>) -> Vec<Item> {
    let mut items = minimal();
    items.extend(more);
    items
}

/// Reads a package of `items` under `limits`.
pub(super) fn read_with(items: Vec<Item>, limits: &Limits) -> (Option<Model>, Seen) {
    let mut diagnostics = Diagnostics::new();
    let reader = Reader::new(Zip::of(items).build());
    let package = Package::open(reader, limits, &mut diagnostics).unwrap();
    (package, seen(diagnostics))
}

/// [`read_with`] under the default limits.
pub(super) fn read(items: Vec<Item>) -> (Option<Model>, Seen) {
    read_with(items, &Limits::default())
}

/// What reading `items` found, the package asserted read.
pub(super) fn found(items: Vec<Item>) -> Seen {
    let (package, seen) = read(items);
    assert!(package.is_some(), "{seen:?}");
    seen
}

/// The package read from `items`, which must read without a diagnostic.
pub(super) fn clean(items: Vec<Item>) -> Model {
    let (package, seen) = read(items);
    assert_eq!(seen, vec![]);
    package.unwrap()
}

/// The diagnostics in `diagnostics`, each with where it points: its entry,
/// part, or relationships part and `#` its relationship's `Id`.
pub(super) fn seen(diagnostics: Diagnostics) -> Seen {
    let place = |location: Location| {
        let id = location.relationship_id.map(|id| id.as_str().to_owned());
        match (location.entry, location.part, location.rels_part) {
            (Some(entry), _, _) => entry.as_str().to_owned(),
            (None, Some(part), _) => part.as_str().to_owned(),
            (None, None, Some(rels)) => format!("{}#{}", rels.as_str(), id.unwrap_or_default()),
            (None, None, None) => String::new(),
        }
    };
    (diagnostics.finish().into_iter())
        .map(|record| (record.diagnostic, place(record.location)))
        .collect()
}

/// `(diagnostic, place)`, as [`seen`] lists one.
pub(super) fn at(diagnostic: Diagnostic, place: &str) -> (Diagnostic, String) {
    (diagnostic, place.to_owned())
}

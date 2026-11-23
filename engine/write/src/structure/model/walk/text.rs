//! The text a reference stands for, inside a `definedName`.

use xlsx_graft_xml::events::BytesRef;

/// The predefined entities (XML 1.0 §4.6), the only ones the XML reader
/// admits.
const ENTITIES: [(&str, char); 5] = [
    ("lt", '<'),
    ("gt", '>'),
    ("amp", '&'),
    ("apos", '\''),
    ("quot", '"'),
];

/// The text a reference the XML reader admitted stands for.
pub(super) fn resolve(reference: &BytesRef<'_>) -> String {
    let named = ENTITIES.iter().find(|(name, _)| *name == &**reference);
    let c = reference.resolve_char_ref().ok().flatten();
    (c.or(named.map(|&(_, c)| c)))
        .map(String::from)
        .unwrap_or_default()
}

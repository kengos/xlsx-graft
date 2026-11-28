//! Reading and writing the start tags `SheetEdit` changes or adds: an
//! attribute's value, a start tag with some attributes replaced, added or
//! removed, and a new element written as raw bytes.

use super::apply::Rw;
use crate::error::WriteError;
use crate::sheet::write_attributes;
use std::io::Write;
use xlsx_graft_cell::xml_attribute;
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::BytesStart;

/// The value of the attribute `key` (without a prefix, as written) of
/// `start`, as the XML reader normalizes it.
pub(super) fn attribute(start: &BytesStart<'_>, key: &str) -> Option<String> {
    let found = start.attributes().flatten();
    let found = found.into_iter().find(|a| a.key.as_ref() == key);
    found.map(|a| {
        let value = a.normalized_value(XmlVersion::Implicit1_0);
        value.unwrap_or_default().into_owned()
    })
}

/// The name of `start`, with its prefix.
pub(super) fn name_of(start: &BytesStart<'_>) -> String {
    start.name().as_ref().to_owned()
}

/// Whether `start` has an attribute whose name is not one of `keys`.
pub(super) fn has_besides(start: &BytesStart<'_>, keys: &[&str]) -> bool {
    (start.attributes().flatten()).any(|a| !keys.contains(&a.key.as_ref()))
}

/// Appends ` key="value"` to `text`, the value written as an XML attribute.
fn push(text: &mut String, key: &str, value: &str) {
    text.push(' ');
    text.push_str(key);
    text.push_str("=\"");
    // Writing to a `String` does not fail.
    xml_attribute(value, |piece| {
        text.push_str(piece);
        Ok::<(), std::io::Error>(())
    })
    .unwrap_or_default();
    text.push('"');
}

/// `start` with the attributes of `changes` set (`Some`) or removed
/// (`None`): an attribute already there keeps its place and an absent one
/// is added at the end. The other attributes, namespace declarations
/// included, keep their values as written, single quotes aside.
pub(super) fn retag(
    start: &BytesStart<'_>,
    changes: &[(&str, Option<String>)],
) -> BytesStart<'static> {
    let name = name_of(start);
    let mut text = name.clone();
    let mut present = Vec::new();
    for attribute in start.attributes().flatten() {
        let key = attribute.key.as_ref();
        match changes.iter().find(|(changed, _)| *changed == key) {
            Some((_, value)) => {
                present.push(key.to_owned());
                if let Some(value) = value {
                    push(&mut text, key, value);
                }
            }
            None => {
                let raw = attribute.value.replace('"', "&quot;");
                text.push_str(&format!(" {key}=\"{raw}\""));
            }
        }
    }
    for (key, value) in changes {
        if let Some(value) = value.as_ref().filter(|_| !present.iter().any(|p| p == key)) {
            push(&mut text, key, value);
        }
    }
    BytesStart::from_content(text, name.len())
}

/// Writes `<name a="1" …/>` as raw bytes.
pub(super) fn write_empty(
    rw: &mut Rw<'_>,
    name: &str,
    attributes: &[(&str, String)],
) -> Result<(), WriteError> {
    write_open(rw, name, attributes, "/>")
}

/// Writes `<name a="1" …>` as raw bytes.
pub(super) fn write_start(
    rw: &mut Rw<'_>,
    name: &str,
    attributes: &[(&str, String)],
) -> Result<(), WriteError> {
    write_open(rw, name, attributes, ">")
}

/// Writes `</name>` as raw bytes.
pub(super) fn write_end(rw: &mut Rw<'_>, name: &str) -> Result<(), WriteError> {
    write!(rw.output_mut(), "</{name}>")?;
    Ok(())
}

/// Writes `<name`, the attributes and `close`.
fn write_open(
    rw: &mut Rw<'_>,
    name: &str,
    attributes: &[(&str, String)],
    close: &str,
) -> Result<(), WriteError> {
    let out = rw.output_mut();
    write!(out, "<{name}")?;
    write_attributes(out, attributes)?;
    out.write_all(close.as_bytes())?;
    Ok(())
}

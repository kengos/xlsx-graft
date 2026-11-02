//! The attributes of an element: the formulas some keep, and the value of
//! one as the collector reads it.
//!
//! An attribute is read only unprefixed, as SpreadsheetML writes it: an
//! attribute of another namespace with the same local name (`z:val`) is
//! no decoy for the real one.

use super::{Collector, Open};
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::BytesStart;

/// The most bytes of an attribute that places a formula or names a defined
/// name kept: Excel keeps a defined name to 255 characters.
pub(super) const MAX_ATTRIBUTE_BYTES: usize = 1024;

impl Collector<'_> {
    /// Reads the formulas the attributes of `start`, of local name `local`,
    /// keep.
    pub(super) fn attributes(&mut self, local: &str, start: &BytesStart<'_>) {
        match local {
            "sp" => {
                self.formula_in(start, "textlink", "sp@textlink");
                self.formula_in(start, "macro", "sp@macro");
            }
            "cxnSp" => self.formula_in(start, "macro", "cxnSp@macro"),
            "pic" => self.formula_in(start, "macro", "pic@macro"),
            "graphicFrame" => self.formula_in(start, "macro", "graphicFrame@macro"),
            "objectPr" => self.formula_in(start, "macro", "objectPr@macro"),
            "controlPr" => {
                self.formula_in(start, "macro", "controlPr@macro");
                self.formula_in(start, "linkedCell", "controlPr@linkedCell");
                self.formula_in(start, "listFillRange", "controlPr@listFillRange");
            }
            "cameraTool" => self.formula_in(start, "cellRange", "cameraTool@cellRange"),
            "cfvo" => self.formula_in(start, "val", "cfvo@val"),
            "cacheField" => self.formula_in(start, "formula", "cacheField@formula"),
            "calculatedItem" => self.formula_in(start, "formula", "calculatedItem@formula"),
            "oleObject" => self.formula_in(start, "link", "oleObject@link"),
            "definedName" => self.formula_in(start, "refersTo", "definedName@refersTo"),
            _ => {}
        }
    }

    /// Reads the formula in the attribute `name` of `start`, if it has it,
    /// at the place `what`.
    fn formula_in(&mut self, start: &BytesStart<'_>, name: &str, what: &str) {
        if let Some(text) = attribute(start, name) {
            self.hand(Open {
                element: String::new(),
                at: self.place(what),
                defines: None,
                size: bytes(&text),
                markup: false,
                blank: text.chars().all(char::is_whitespace),
                text,
            });
        }
    }
}

/// The value of the unprefixed attribute `name` of `start`, if it has one
/// that reads, up to [`MAX_ATTRIBUTE_BYTES`].
pub(super) fn value(start: &BytesStart<'_>, name: &str) -> Option<String> {
    let value = attribute(start, name)?;
    let mut end = value.len().min(MAX_ATTRIBUTE_BYTES);
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    Some(value.get(..end).unwrap_or_default().to_owned())
}

/// The value of the unprefixed attribute `name` of `start`, whole, if it
/// has one that reads.
fn attribute(start: &BytesStart<'_>, name: &str) -> Option<String> {
    (start.attributes().flatten())
        .find(|attribute| attribute.key.as_ref() == name)
        .and_then(|attribute| {
            let value = attribute.normalized_value(XmlVersion::Implicit1_0);
            value.ok().map(|value| value.into_owned())
        })
}

/// The bytes `text` holds as UTF-8.
pub(super) fn bytes(text: &str) -> u64 {
    u64::try_from(text.len()).unwrap_or(u64::MAX)
}

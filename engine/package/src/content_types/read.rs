//! [`read`]: the content types stream's XML (opc-contentTypes.xsd), as
//! [`ContentTypes`].

use super::{ContentType, ContentTypes};
use crate::PartName;
use crate::part_name::{decode_non_ascii, shadow_key};
use crate::schema::{self, ChildShape, Element, Shape};
use std::collections::BTreeMap;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};
use xlsx_graft_xml::XmlLimits;

/// The stream's shape: `Types` holding `Default` and `Override`.
const SHAPE: Shape = Shape {
    namespace: "http://schemas.openxmlformats.org/package/2006/content-types",
    root: "Types",
    children: &[
        ChildShape {
            name: "Default",
            attributes: &["Extension", "ContentType"],
            text: false,
        },
        ChildShape {
            name: "Override",
            attributes: &["PartName", "ContentType"],
            text: false,
        },
    ],
};

/// The index of `Default` in the shape's children.
const DEFAULT: usize = 0;

/// Reads the stream's `bytes`, pushing what is wrong with it at `location`.
/// `None` when it cannot be read to its end.
pub(crate) fn read(
    bytes: &[u8],
    limits: &XmlLimits,
    location: &Location,
    diagnostics: &mut Diagnostics,
) -> Option<ContentTypes> {
    let mut types = ContentTypes::default();
    let read = schema::read(
        bytes,
        limits,
        &SHAPE,
        location,
        diagnostics,
        |element, diagnostics| {
            let mut push = |diagnostic| diagnostics.push(diagnostic, location.clone());
            types.add(element, &mut push);
        },
    );
    // A child the walk refused may have typed any part.
    types.wild |= !read?;
    Some(types)
}

impl ContentTypes {
    /// Adds a `Default` or `Override`, pushing what is wrong with it. One
    /// that is refused still maps what it names, as refused; one that does
    /// not say what it names makes every lookup refused.
    fn add(&mut self, element: Element, push: &mut impl FnMut(Diagnostic)) {
        let (name, value) = match element.values.as_slice() {
            [Some(name), value] => (name.clone(), value.clone()),
            values => {
                push(schema::missing_attribute(&SHAPE, &element, values));
                self.wild = true;
                return;
            }
        };
        let content_type = match &value {
            Some(value) => ContentType::parse(value),
            None => None,
        };
        match (&value, &content_type) {
            (None, _) => push(schema::missing_attribute(&SHAPE, &element, &element.values)),
            (Some(value), None) => push(Diagnostic::ContentTypeInvalid {
                content_type: Echo::new(value),
            }),
            (Some(_), Some(_)) => {}
        }
        if element.child == DEFAULT {
            // Keyed as a part name's extension is: decoded, then lowered.
            let key = decode_non_ascii(&name).to_ascii_lowercase();
            if !is_extension(&name) {
                push(schema::invalid_value(element.offset, "Extension"));
                self.default_holes.insert(key);
            } else if insert(&mut self.defaults, key, content_type) {
                push(Diagnostic::ContentTypesDuplicateDefault {
                    extension: Echo::new(&name),
                });
            }
            return;
        }
        let name = schema::collapse(&name);
        match PartName::parse(&name) {
            Ok(part) => {
                if insert(&mut self.overrides, part.key().to_owned(), content_type) {
                    push(Diagnostic::ContentTypesDuplicateOverride {
                        part_name: Echo::new(&name),
                    });
                }
            }
            Err(reason) => {
                push(Diagnostic::ContentTypesPartNameInvalid {
                    part_name: Echo::new(&name),
                    reason,
                });
                self.override_holes.insert(shadow_key(name.as_bytes()));
            }
        }
    }
}

/// Maps `key` to `content_type`, or refuses it when it is mapped already.
/// Returns whether it was.
fn insert(
    map: &mut BTreeMap<String, Option<ContentType>>,
    key: String,
    content_type: Option<ContentType>,
) -> bool {
    match map.get_mut(&key) {
        Some(mapped) => {
            *mapped = None;
            true
        }
        None => {
            map.insert(key, content_type);
            false
        }
    }
}

/// Whether `extension` is an `ST_Extension`: one or more of the characters
/// a part name may hold but `/` and `.`, percent-encodings among them.
fn is_extension(extension: &str) -> bool {
    let mut rest = extension.as_bytes();
    while let Some((&byte, after)) = rest.split_first() {
        rest = match (byte, after) {
            (b'%', [high, low, after @ ..])
                if high.is_ascii_hexdigit() && low.is_ascii_hexdigit() =>
            {
                after
            }
            (byte, after) if byte.is_ascii_alphanumeric() || b"!$&'()*+,:=@-_~".contains(&byte) => {
                after
            }
            _ => return false,
        };
    }
    !extension.is_empty()
}

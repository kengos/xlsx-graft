//! The `ref` of each table of a sheet: the root `table` of each table part
//! the sheet's Relationships part targets (ECMA-376-1:2016 §18.5.1.2), so
//! `merge` can refuse a range that overlaps one.

use super::name::is_main;
use super::tag::attribute;
use crate::error::WriteError;
use crate::sheet::CellRange;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_package::{Package, Target};
use xlsx_graft_xml::events::Event;
use xlsx_graft_xml::{XmlLimits, XmlReader};

/// The relationship types of a table part, Transitional and Strict.
const TABLE_TYPES: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/table",
    "http://purl.oclc.org/ooxml/officeDocument/relationships/table",
];

/// The `ref`s of the tables of the part at `index` of the package's parts,
/// in the order of its relationships; a `ref` that is not a range is left
/// out. `None` when a table part cannot be read: it is too large or its
/// zip entry is damaged (the package pushed the diagnostic) or its XML is not
/// well formed (pushed by `read`, located at the table part).
///
/// # Errors
///
/// [`WriteError::Io`] when reading the package fails.
pub(super) fn read<R: Read + Seek>(
    package: &mut Package<R>,
    index: usize,
    limits: &XmlLimits,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Vec<CellRange>>, WriteError> {
    let relationships =
        (package.parts().get(index).into_iter()).flat_map(|part| part.relationships());
    let targets: Vec<usize> = relationships
        .filter(|relationship| TABLE_TYPES.contains(&relationship.rel_type()))
        .filter_map(|relationship| match relationship.target() {
            Target::Internal(target) => Some(*target),
            Target::External(_) => None,
        })
        .collect();
    let mut refs = Vec::new();
    for target in targets {
        let Some(bytes) = package.read_xml_part(target, limits.max_bytes, diagnostics)? else {
            return Ok(None);
        };
        match table_ref(&bytes, limits) {
            Ok(found) => refs.extend(found),
            Err(diagnostic) => {
                let name = (package.parts().get(target)).map(|part| part.name().as_str());
                let location = Location::default().with_part(name.unwrap_or_default());
                diagnostics.push(diagnostic, location);
                return Ok(None);
            }
        }
    }
    Ok(Some(refs))
}

/// The `ref` of the root of the table part `bytes`, read to its end.
fn table_ref(bytes: &[u8], limits: &XmlLimits) -> Result<Option<CellRange>, Diagnostic> {
    let mut reader = XmlReader::open(bytes, limits)?;
    let mut buf = Vec::new();
    let mut root = None;
    let mut first = true;
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(start) | Event::Empty(start) => {
                if first && is_main(&reader.resolve_element(start.name()), "table") {
                    root = attribute(&start, "ref")
                        .and_then(|text| CellRange::parse(text.trim()).ok());
                }
                first = false;
            }
            Event::Eof => return Ok(root),
            _ => {}
        }
    }
}

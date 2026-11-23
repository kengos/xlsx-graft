//! [`read`]: the names of the tables of the table parts
//! (ECMA-376-1:2016 §18.5.1.2).
//!
//! A part is a table part when its media type is the table's. Its tables are
//! the elements whose local name is `table`, in any namespace and at any
//! depth, each with its `name` and `displayName` as written, the way
//! `xlsx-graft-verify` takes the tables it checks names against: a part
//! whose root is another element, or whose root holds more than one `table`,
//! gives a [`Table`] for each. The XML is read to its end, so a part that is
//! not well formed is refused.

use super::Table;
use crate::error::WriteError;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_package::Package;
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::{XmlLimits, XmlReader, XmlVersion};

/// The media type of a table part.
const TABLE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";

/// The tables of `package`, in the order of its parts, under the XML
/// limits `limits`. `None` when a table part cannot be read: it is too
/// large or damaged (pushed by the package) or its XML is not well formed,
/// anywhere in the part (pushed here, at the part).
///
/// # Errors
///
/// [`WriteError::Io`] when reading the package fails.
pub(super) fn read<R: Read + Seek>(
    package: &mut Package<R>,
    limits: &XmlLimits,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Vec<Table>>, WriteError> {
    let parts = (package.parts().iter().enumerate())
        .filter(|(_, part)| {
            part.content_type()
                .is_some_and(|kind| kind.essence() == TABLE)
        })
        .map(|(index, part)| (index, Location::default().with_part(part.name().as_str())))
        .collect::<Vec<_>>();
    let mut tables = Vec::new();
    for (index, location) in parts {
        let Some(bytes) = package.read_xml_part(index, limits.max_bytes, diagnostics)? else {
            return Ok(None);
        };
        match self::tables(&bytes, index, limits) {
            Ok(found) => tables.extend(found),
            Err(diagnostic) => {
                diagnostics.push(diagnostic, location);
                return Ok(None);
            }
        }
    }
    Ok(Some(tables))
}

/// The tables of the part at `part` whose bytes are `bytes`: one for each
/// element whose local name is `table`. The XML is read to its end.
fn tables(bytes: &[u8], part: usize, limits: &XmlLimits) -> Result<Vec<Table>, Diagnostic> {
    let mut reader = XmlReader::open(bytes, limits)?;
    let mut buf = Vec::new();
    let mut found = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(start) | Event::Empty(start) => {
                let (_, local) = reader.resolve_element(start.name());
                if local.as_ref() == "table" {
                    found.push(names(&start, part));
                }
            }
            Event::Eof => return Ok(found),
            _ => {}
        }
    }
}

/// The table of the element `start`, in the part at index `part`, with its
/// `name` and `displayName` as written.
fn names(start: &BytesStart<'_>, part: usize) -> Table {
    let (mut name, mut display_name) = (None, None);
    for attribute in start.attributes().flatten() {
        let value = attribute.normalized_value(XmlVersion::Implicit1_0);
        let value = value.unwrap_or_default().into_owned();
        match attribute.key.as_ref() {
            "name" => name = Some(value),
            "displayName" => display_name = Some(value),
            _ => {}
        }
    }
    Table {
        part,
        name,
        display_name,
    }
}

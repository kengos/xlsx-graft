//! [`Package::assign_types`]: each part's media type, from the content types
//! stream (ECMA-376-2:2021 §7.2.3).
//!
//! A part with none is `part_content_type_missing`, but a Relationships
//! part, which §7.2.3.2.1 leaves out. A part whose name is a Relationships
//! part's and whose type is not the Relationships media type, or the other
//! way round, is `part_content_type_mismatch` (§6.5.2.1; 4th edition M1.30).
//! When the stream is missing, that is reported once; when it is refused or
//! cannot be read, its own diagnostic stands for every part.

use super::names::ContentTypesEntry;
use super::{Package, xml_part};
use crate::content_types::{self, Lookup, RELATIONSHIPS};
use crate::{Limits, PackageError};
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};

impl<R: Read + Seek> Package<R> {
    /// Reads the content types stream from `entry` and gives each part its
    /// media type.
    pub(super) fn assign_types(
        &mut self,
        entry: ContentTypesEntry,
        limits: &Limits,
        diagnostics: &mut Diagnostics,
    ) -> Result<(), PackageError> {
        // Until the stream is read, no part's type is known.
        for part in &mut self.parts {
            part.content_type_unknown = true;
        }
        let index = match entry {
            ContentTypesEntry::At(index) => index,
            ContentTypesEntry::Missing => {
                diagnostics.push(Diagnostic::ContentTypesMissing {}, Location::default());
                return Ok(());
            }
            ContentTypesEntry::Refused => return Ok(()),
        };
        let name = self
            .archive
            .entries()
            .get(index)
            .map_or(&[][..], |e| e.name());
        let location = Location::default().with_entry_bytes(name);
        let max_bytes = limits.xml.max_bytes;
        let bytes = xml_part::read(&mut self.archive, index, max_bytes, &location, diagnostics)?;
        let Some(types) = bytes
            .and_then(|bytes| content_types::read(&bytes, &limits.xml, &location, diagnostics))
        else {
            return Ok(());
        };
        for part in &mut self.parts {
            let is_relationships = part.name.is_relationships();
            let lookup = types.lookup(&part.name);
            part.content_type_unknown = lookup == Lookup::Refused;
            match lookup {
                Lookup::Found(content_type) => {
                    if is_relationships != (content_type.essence() == RELATIONSHIPS)
                        && part.readable
                    {
                        let diagnostic = Diagnostic::PartContentTypeMismatch {
                            content_type: Echo::new(content_type.as_str()),
                        };
                        diagnostics.push(diagnostic, part.location());
                    }
                    part.content_type = Some(content_type.clone());
                }
                Lookup::Missing if !is_relationships && part.readable => {
                    diagnostics.push(Diagnostic::PartContentTypeMissing {}, part.location());
                }
                Lookup::Missing | Lookup::Refused => {}
            }
        }
        Ok(())
    }
}

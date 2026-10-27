//! [`Package::read_graph`]: every Relationships part read into the graph of
//! relationships between the parts (ECMA-376-2:2021 §6.5).
//!
//! A Relationships part is read when its source is the package or a part
//! of it; one whose source is not there is `relationships_source_missing`.
//! An internal target is resolved against the source part's name; a target
//! that names no part is `relationship_target_missing`, and a relationship
//! from or to a Relationships part is `relationship_rels_part`. Neither is
//! kept, so every internal relationship in the graph targets a part. Once
//! the package declares more relationships than `maxTotalRelationships`
//! (`relationships_total_too_many`), no later Relationships part is read.
//!
//! The package's own relationships must include an internal
//! `officeDocument` one (`main_document_missing`). When every Relationships
//! part was read, a part no relationship targets is `part_unreferenced`,
//! a notice. When one was not, or one may be there but is not a part
//! ([`Presence::Unknown`]), or one left out a relationship it declares for a
//! reason already reported, which parts it would have reached is not known,
//! and none is reported; the same goes for the main document when the
//! package's own relationships are the ones. A source or a target that is
//! [`Presence::Unknown`] is not reported missing.

mod link;

use super::{Package, Presence, RelationshipSource, xml_part};
use crate::part_name::Source;
use crate::relationships::{self, OFFICE_DOCUMENT};
use crate::{Limits, PackageError, PartName};
use link::Context;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};

/// The key of the package's own Relationships part.
const PACKAGE_RELATIONSHIPS: &str = "/_rels/.rels";

/// What reading one Relationships part found.
#[derive(Debug, Clone, Copy)]
struct Outcome {
    /// Whether it declares an internal `officeDocument` relationship.
    main: bool,
    /// Whether nothing in it was left out for a reason already reported,
    /// so every relationship it declares that targets a part is in the
    /// graph.
    whole: bool,
}

/// A Relationships part to read.
struct RelsPart {
    source: Source,
    name: String,
    entry: usize,
    location: Location,
    readable: bool,
}

impl<R: Read + Seek> Package<R> {
    /// Reads every Relationships part. Returns whether all of them were.
    pub(super) fn read_graph(
        &mut self,
        limits: &Limits,
        diagnostics: &mut Diagnostics,
    ) -> Result<bool, PackageError> {
        // A Relationships part that may be there but is not a part is not
        // read either.
        let mut complete = !(self.unknown.iter())
            .any(|key| PartName::parse(key).is_ok_and(|name| name.is_relationships()));
        let mut main = None;
        for key in self.unknown.clone() {
            // The relationships of a part whose Relationships part may be
            // there are not known.
            if let Some(source) = PartName::parse(&key)
                .ok()
                .and_then(|n| n.relationships_source())
            {
                self.set_known(&source, false);
            }
        }
        for index in 0..self.parts.len() {
            let rels = self.parts.get(index).and_then(|part| {
                Some(RelsPart {
                    source: part.name.relationships_source()?,
                    name: part.name.to_string(),
                    entry: part.entry,
                    location: part.location(),
                    readable: part.readable,
                })
            });
            let Some(rels) = rels else {
                continue;
            };
            let is_package = rels.source == Source::Package;
            self.set_known(&rels.source, false);
            let read = self.read_part(rels, limits, diagnostics)?;
            complete &= read.is_some_and(|read| read.whole);
            if is_package {
                // A relationship left out may have been the main document's.
                main = read.and_then(|read| (read.main || read.whole).then_some(read.main));
            }
        }
        let missing = match self.find_key(PACKAGE_RELATIONSHIPS) {
            Presence::Absent => true,
            Presence::Part(_) => main == Some(false),
            Presence::Unknown => false,
        };
        if missing {
            diagnostics.push(Diagnostic::MainDocumentMissing {}, Location::default());
        }
        Ok(complete)
    }

    /// Reads the Relationships part `rels`, or returns `None` when it is not
    /// read.
    fn read_part(
        &mut self,
        rels: RelsPart,
        limits: &Limits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Outcome>, PackageError> {
        let RelsPart {
            source,
            name: rels,
            entry,
            location,
            readable,
        } = rels;
        if !readable {
            return Ok(None);
        }
        let (base, from) = match source {
            Source::Package => (String::from("/"), RelationshipSource::Package),
            Source::Part(name) => match self.find_key(&name.to_ascii_lowercase()) {
                Presence::Part(source) => (name, RelationshipSource::Part(source)),
                Presence::Absent => {
                    diagnostics.push(Diagnostic::RelationshipsSourceMissing {}, location);
                    return Ok(None);
                }
                Presence::Unknown => return Ok(None),
            },
        };
        // Past the package's limit, no Relationships part is read.
        if self.total_relationships > limits.max_total_relationships.value {
            return Ok(None);
        }
        let max_bytes = limits.xml.max_bytes;
        let bytes = xml_part::read(&mut self.archive, entry, max_bytes, &location, diagnostics)?;
        let Some((raws, mut whole)) = bytes
            .and_then(|bytes| relationships::read(&bytes, limits, &location, &rels, diagnostics))
        else {
            return Ok(None);
        };
        let main = raws
            .iter()
            .any(|raw| !raw.external && OFFICE_DOCUMENT.contains(&raw.rel_type.as_str()));
        if !self.count_relationships(raws.len(), limits, &location, diagnostics) {
            return Ok(None);
        }
        let from_rels = match from {
            RelationshipSource::Part(source) => self
                .parts
                .get(source)
                .is_some_and(|p| p.name.is_relationships()),
            RelationshipSource::Package => false,
        };
        if from_rels {
            // One problem: the part should not be there (§6.5.2.1).
            let location = Location::default().with_rels_part(&rels);
            diagnostics.push(Diagnostic::RelationshipRelsPart {}, location);
            return Ok(Some(Outcome { main, whole: false }));
        }
        let context = Context { rels, base, from };
        let (declared, mut kept) = (raws.len(), Vec::new());
        for raw in raws {
            let linked = self.link(raw, &context, kept.len(), diagnostics);
            match linked {
                Ok(Some(relationship)) => kept.push(relationship),
                Ok(None) => {}
                Err(()) => whole = false,
            }
        }
        let relationships = match from {
            RelationshipSource::Part(source) => {
                self.parts.get_mut(source).map(|p| &mut p.relationships)
            }
            RelationshipSource::Package => Some(&mut self.relationships),
        };
        // Known when none was left out, nor dropped for its target.
        let known = whole && kept.len() == declared;
        if let Some(relationships) = relationships {
            *relationships = kept;
        }
        match from {
            RelationshipSource::Part(source) => {
                if let Some(part) = self.parts.get_mut(source) {
                    part.relationships_known = known;
                }
            }
            RelationshipSource::Package => self.relationships_known = known,
        }
        Ok(Some(Outcome { main, whole }))
    }

    /// Counts `declared` more relationships, those a part keeps once it is
    /// read, against the package's limit.
    /// Returns false when they are past it, pushing
    /// `relationships_total_too_many` at `location` when they are the first.
    fn count_relationships(
        &mut self,
        declared: usize,
        limits: &Limits,
        location: &Location,
        diagnostics: &mut Diagnostics,
    ) -> bool {
        let limit = limits.max_total_relationships;
        let before = self.total_relationships;
        let count = before.saturating_add(u64::try_from(declared).unwrap_or(u64::MAX));
        self.total_relationships = count;
        if count <= limit.value {
            return true;
        }
        if before <= limit.value {
            let diagnostic = Diagnostic::RelationshipsTotalTooMany { count, limit };
            diagnostics.push(diagnostic, location.clone());
        }
        false
    }

    /// Sets whether the relationships of `source` are known.
    fn set_known(&mut self, source: &Source, known: bool) {
        match source {
            Source::Package => self.relationships_known = known,
            Source::Part(name) => {
                if let Presence::Part(index) = self.find_key(&name.to_ascii_lowercase())
                    && let Some(part) = self.parts.get_mut(index)
                {
                    part.relationships_known = known;
                }
            }
        }
    }

    /// Pushes `part_unreferenced` at each part no relationship targets, but
    /// the Relationships parts.
    pub(super) fn unreferenced(&self, diagnostics: &mut Diagnostics) {
        for part in &self.parts {
            if part.readable && part.incoming.is_empty() && !part.name.is_relationships() {
                diagnostics.push(Diagnostic::PartUnreferenced {}, part.location());
            }
        }
    }
}

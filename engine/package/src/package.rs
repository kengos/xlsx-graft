//! [`Package`]: a zip read as an OPC package: its parts, their media types
//! and the relationships between them.
//!
//! [`Package::open`] reads the zip (`zip.rs`), maps its entries to parts
//! (`names.rs`), gives each part its media type (`types.rs`), then reads
//! every Relationships part into the graph (`graph.rs`). The XML streams are
//! read whole into memory, each under the XML size limit (`xml_part.rs`).

mod graph;
mod names;
mod types;
mod xml_part;

use crate::{ContentType, EntryReader, Limits, PackageError, PartName, Relationship, ZipArchive};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostics, Location};

/// Where a relationship is declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipSource {
    /// In the package's own Relationships part, `/_rels/.rels`.
    Package,
    /// In the Relationships part of the part at this index of
    /// [`Package::parts`].
    Part(usize),
}

/// A relationship that targets a part: where it is declared, and its index
/// among that source's relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Incoming {
    /// Where the relationship is declared.
    pub source: RelationshipSource,
    /// Its index in [`Package::relationships`] or [`Part::relationships`].
    pub index: usize,
}

/// Whether the package holds a part of some name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// The part at this index of [`Package::parts`].
    Part(usize),
    /// No part, and no entry that could have been one.
    Absent,
    /// No part, but an entry that could have been one was refused, for a
    /// reason already reported: whether the part is there is not known, so
    /// its absence is not a problem of its own.
    Unknown,
}

/// One part of the package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    name: PartName,
    entry: usize,
    readable: bool,
    content_type: Option<ContentType>,
    /// Whether the content types stream could not say its type.
    content_type_unknown: bool,
    relationships: Vec<Relationship>,
    /// Whether `relationships` are all its Relationships part declares.
    relationships_known: bool,
    incoming: Vec<Incoming>,
}

impl Part {
    /// Its name.
    #[must_use]
    pub fn name(&self) -> &PartName {
        &self.name
    }

    /// The index of its zip entry in [`ZipArchive::entries`].
    #[must_use]
    pub fn entry(&self) -> usize {
        self.entry
    }

    /// Its media type, when the content types stream gives it one.
    #[must_use]
    pub fn content_type(&self) -> Option<&ContentType> {
        self.content_type.as_ref()
    }

    /// Whether its media type is not known: the content types stream is
    /// missing or was not read, or the mapping that would give its type was
    /// refused, all reported already. A part with no
    /// [`content_type`](Self::content_type) whose type is known has none.
    #[must_use]
    pub fn content_type_unknown(&self) -> bool {
        self.content_type_unknown
    }

    /// The relationships its Relationships part declares, in their order.
    #[must_use]
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    /// Whether [`relationships`](Self::relationships) are all it has: its
    /// Relationships part, if there is one, was read and none of its
    /// relationships was left out, for whatever reason already reported.
    /// When not, a relationship looked up by `Id` and not found may be one
    /// that was left out, and is not a problem of its own.
    #[must_use]
    pub fn relationships_known(&self) -> bool {
        self.relationships_known
    }

    /// The relationships that target it, in the order they were read.
    #[must_use]
    pub fn incoming(&self) -> &[Incoming] {
        &self.incoming
    }

    /// Where a diagnostic about it points.
    fn location(&self) -> Location {
        Location::default().with_part(self.name.as_str())
    }
}

/// A package read from `R`.
#[derive(Debug)]
pub struct Package<R> {
    archive: ZipArchive<R>,
    parts: Vec<Part>,
    /// Each part's index by its name's key.
    keys: BTreeMap<String, usize>,
    relationships: Vec<Relationship>,
    /// Whether `relationships` are all `/_rels/.rels` declares, as
    /// [`Part::relationships_known`] has it.
    relationships_known: bool,
    /// The keys whose parts are [`Presence::Unknown`].
    unknown: BTreeSet<String>,
    /// How many relationships the Relationships parts read so far declare.
    total_relationships: u64,
}

impl<R: Read + Seek> Package<R> {
    /// Reads the package, pushing a diagnostic for each problem found.
    ///
    /// `Ok(None)` when the zip's central directory cannot be read, which the
    /// diagnostics say why.
    ///
    /// # Errors
    ///
    /// [`PackageError::Io`] when reading or seeking `reader` fails.
    pub fn open(
        reader: R,
        limits: &Limits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Self>, PackageError> {
        let Some(archive) = ZipArchive::open(reader, &limits.zip, diagnostics)? else {
            return Ok(None);
        };
        let names = names::map(archive.entries(), diagnostics);
        let parts: Vec<Part> = (names.parts.into_iter())
            .map(|(name, entry)| Part {
                readable: archive
                    .entries()
                    .get(entry)
                    .is_some_and(|e| e.is_readable()),
                name,
                entry,
                content_type: None,
                content_type_unknown: false,
                relationships: Vec::new(),
                relationships_known: true,
                incoming: Vec::new(),
            })
            .collect();
        let keys = (parts.iter().enumerate())
            .map(|(index, part)| (part.name.key().to_owned(), index))
            .collect();
        let mut package = Self {
            archive,
            parts,
            keys,
            relationships: Vec::new(),
            relationships_known: true,
            unknown: names.unknown,
            total_relationships: 0,
        };
        package.assign_types(names.content_types, limits, diagnostics)?;
        let complete = package.read_graph(limits, diagnostics)?;
        if complete {
            package.unreferenced(diagnostics);
        }
        Ok(Some(package))
    }

    /// Every part, in the order of their zip entries.
    #[must_use]
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }

    /// The part named `name`, ASCII letter case aside; `None` when there is
    /// none, or `name` is not a part name.
    #[must_use]
    pub fn part(&self, name: &str) -> Option<&Part> {
        self.part_named(&PartName::parse(name).ok()?)
    }

    /// The part named `name`, such as an internal relationship's target.
    #[must_use]
    pub fn part_named(&self, name: &PartName) -> Option<&Part> {
        self.parts.get(*self.keys.get(name.key())?)
    }

    /// Whether the package holds a part named `name`, or may: a reader
    /// that looks a part up reports it missing only when it is
    /// [`Presence::Absent`].
    #[must_use]
    pub fn find(&self, name: &PartName) -> Presence {
        self.find_key(name.key())
    }

    /// [`find`](Self::find) by a name's key.
    fn find_key(&self, key: &str) -> Presence {
        match self.keys.get(key) {
            Some(&index) => Presence::Part(index),
            None if self.unknown.contains(key) => Presence::Unknown,
            None => Presence::Absent,
        }
    }

    /// The package relationships, from `/_rels/.rels`.
    #[must_use]
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    /// Whether [`relationships`](Self::relationships) are all the package
    /// has, as [`Part::relationships_known`] has it.
    #[must_use]
    pub fn relationships_known(&self) -> bool {
        self.relationships_known
    }

    /// The zip the package is read from.
    #[must_use]
    pub fn archive(&self) -> &ZipArchive<R> {
        &self.archive
    }

    /// A reader of the part at `index` of [`parts`](Self::parts), as
    /// [`ZipArchive::open_entry`] gives one.
    ///
    /// # Errors
    ///
    /// [`PackageError::Io`] when seeking to the part's data fails.
    pub fn open_part<'a>(
        &'a mut self,
        index: usize,
        diagnostics: &'a mut Diagnostics,
    ) -> Result<Option<EntryReader<'a, R>>, PackageError> {
        let entry = self.parts.get(index).map_or(usize::MAX, |part| part.entry);
        self.archive.open_entry(entry, diagnostics)
    }
}

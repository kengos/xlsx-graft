//! [`Location`]: where in the package a diagnostic points.

use crate::Echo;
use serde::Serialize;

/// Where in the package a diagnostic points: the zip entry, the part, the
/// relationship and the relationships part it was found in, the place in
/// the part a formula stands and the defined name it reached the problem
/// through, each only when the diagnostic has one.
///
/// Serialized with camelCase keys, leaving out the ones it does not have; a
/// diagnostic about the package as a whole serializes as `{}`. Every value is
/// an [`Echo`], since each is a name the workbook chose.
///
/// The values are for display: stripping and clipping mean one need not
/// equal the name in the package, so nothing looks a part up by it. A
/// consumer that needs the exact name gets it from a field of its own, not
/// from here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    /// The zip entry name, as written in the central directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<Echo>,
    /// The part name, as spelled in the package.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<Echo>,
    /// The `Id` of the relationship.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_id: Option<Echo>,
    /// The relationships part the relationship is declared in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rels_part: Option<Echo>,
    /// Where in the part the formula stands, as the part says it: a cell
    /// (`B2`), or what holds the formula (`definedName MyFn`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<Echo>,
    /// The defined name whose formula holds the problem, reached from the
    /// formula at [`Location::place`] through the names it names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defined_name: Option<Echo>,
}

impl Location {
    /// This location, in the zip entry `name`.
    #[must_use]
    pub fn with_entry(mut self, name: &str) -> Self {
        self.entry = Some(Echo::new(name));
        self
    }

    /// This location, in the zip entry `name`, given as the bytes the
    /// central directory holds, which need not be UTF-8.
    #[must_use]
    pub fn with_entry_bytes(mut self, name: &[u8]) -> Self {
        self.entry = Some(Echo::from_utf8_lossy(name));
        self
    }

    /// This location, in the part `name`.
    #[must_use]
    pub fn with_part(mut self, name: &str) -> Self {
        self.part = Some(Echo::new(name));
        self
    }

    /// This location, at the relationship `id`.
    #[must_use]
    pub fn with_relationship_id(mut self, id: &str) -> Self {
        self.relationship_id = Some(Echo::new(id));
        self
    }

    /// This location, in the relationships part `name`.
    #[must_use]
    pub fn with_rels_part(mut self, name: &str) -> Self {
        self.rels_part = Some(Echo::new(name));
        self
    }

    /// This location, at the place `place` in the part.
    #[must_use]
    pub fn with_place(mut self, place: &str) -> Self {
        self.place = Some(Echo::new(place));
        self
    }

    /// This location, through the defined name `name`.
    #[must_use]
    pub fn with_defined_name(mut self, name: &str) -> Self {
        self.defined_name = Some(Echo::new(name));
        self
    }
}

#[cfg(test)]
mod tests;

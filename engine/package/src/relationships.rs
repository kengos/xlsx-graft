//! [`Relationship`]: one relationship of a Relationships part
//! (ECMA-376-2:2021 §6.5), its target resolved to a part name when it is
//! internal.

mod read;
mod resolve;

pub(crate) use read::{Raw, read};
pub(crate) use resolve::resolve;

/// The `officeDocument` relationship types, Transitional and Strict, of the
/// package relationship to a workbook's main part.
pub(crate) const OFFICE_DOCUMENT: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
    "http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument",
];

/// What a relationship points to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A part of the package (`TargetMode` `Internal`, the default): the
    /// index in [`Package::parts`](crate::Package::parts) of the part its
    /// `Target` resolves to. A relationship whose target names no part is
    /// reported and not kept.
    Internal(usize),
    /// A resource outside the package (`TargetMode` `External`): its
    /// `Target`, unresolved and unchecked.
    External(String),
}

/// One relationship that was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    pub(crate) id: String,
    pub(crate) rel_type: String,
    pub(crate) target: Target,
}

impl Relationship {
    /// Its `Id`, unique in its Relationships part.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Its `Type`, compared as written: letter case counts and no
    /// percent-encoding is undone (§6.5.3.4).
    #[must_use]
    pub fn rel_type(&self) -> &str {
        &self.rel_type
    }

    /// What it points to.
    #[must_use]
    pub fn target(&self) -> &Target {
        &self.target
    }
}

#[cfg(test)]
mod tests;

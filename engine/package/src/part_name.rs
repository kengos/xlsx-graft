//! [`PartName`]: a part's name, checked against the part name grammar
//! (ECMA-376-2:2021 §6.2.2.2) and keyed for the equivalence of §6.2.2.3.
//!
//! A name is held in its IRI form (§7.3.5, and 4th edition §9.1.1.3): a
//! percent-encoded run of bytes that decodes to non-ASCII UTF-8 is decoded,
//! and every other percent-encoding stays as written. Two names are the same
//! part when they are equal but for ASCII letter case, the case of their
//! hexadecimal digits included; non-ASCII letters keep their case. Names that
//! are the same only once normalized to NFC or NFD are told apart, which
//! §6.2.2.3 only advises against.

mod grammar;
mod shadow;

pub(crate) use grammar::{decode_non_ascii, decode_unreserved};
pub(crate) use shadow::shadow_key;

use std::fmt;
use xlsx_graft_diagnostics::PartNameReason;

/// The Relationships part of the package itself (§6.5.2.2).
const PACKAGE_RELATIONSHIPS: &str = "/_rels/.rels";

/// A part name that follows the grammar, as spelled and as compared.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PartName {
    /// The name in IRI form, starting with `/`.
    name: String,
    /// The name with its ASCII letters lowered: equal keys, equivalent names.
    key: String,
}

/// What a Relationships part holds the relationships of (§6.5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source {
    /// The package itself: the part is `/_rels/.rels`.
    Package,
    /// The part of this name, which need not be a valid one.
    Part(String),
}

impl PartName {
    /// The part name `name`, which must start with `/`.
    ///
    /// # Errors
    ///
    /// Why `name` is not a part name.
    pub fn parse(name: &str) -> Result<Self, PartNameReason> {
        if !name.starts_with('/') {
            return Err(PartNameReason::MissingLeadingSlash);
        }
        let name = grammar::decode_non_ascii(name);
        grammar::check(&name)?;
        let key = name.to_ascii_lowercase();
        Ok(Self { name, key })
    }

    /// The part a zip entry `name` maps to (§7.3.5): its non-ASCII
    /// characters decoded, after a `/`.
    pub(crate) fn from_entry(name: &str) -> Result<Self, PartNameReason> {
        Self::parse(&format!("/{name}"))
    }

    /// The name, in IRI form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// The key two equivalent names share.
    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    /// The extension a `Default` matches (§7.2.3.5): what follows the last
    /// `.` of the last segment, lowered; `None` without one.
    pub(crate) fn extension(&self) -> Option<&str> {
        let last = self.key.rsplit('/').next().unwrap_or_default();
        last.rsplit_once('.').map(|(_, extension)| extension)
    }

    /// The source of the relationships this part holds, when its name is a
    /// Relationships part's (§6.2.2.2): `/_rels/.rels`, or a final segment
    /// ending in `.rels` after a `_rels` segment, letter case aside.
    pub(crate) fn relationships_source(&self) -> Option<Source> {
        if self.key == PACKAGE_RELATIONSHIPS {
            return Some(Source::Package);
        }
        let (folder, last) = self.name.rsplit_once('/')?;
        let (parent, rels) = folder.rsplit_once('/')?;
        let stem = last.len().checked_sub(".rels".len())?;
        let (source, suffix) = (last.get(..stem)?, last.get(stem..)?);
        let is_rels = rels.eq_ignore_ascii_case("_rels") && suffix.eq_ignore_ascii_case(".rels");
        is_rels.then(|| Source::Part(format!("{parent}/{source}")))
    }

    /// Whether this is a Relationships part's name.
    pub(crate) fn is_relationships(&self) -> bool {
        self.relationships_source().is_some()
    }
}

impl fmt::Display for PartName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[cfg(test)]
mod tests;

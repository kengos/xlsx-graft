//! [`Link`] and [`LinkSource`]: the relationship that is added with a part,
//! its `Id` and `Target`, and the name of the Relationships part that
//! holds it.

use super::{PackageEdit, reserved};
use crate::error::WriteError;
use std::io::{Read, Seek};
use xlsx_graft_package::PartName;
use xlsx_graft_xml::is_xml_char;

/// Where a relationship is declared: in the package's own relationships
/// or in those of a part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkSource {
    /// The package itself (`/_rels/.rels`).
    Package,
    /// The part of this name, which the package must have.
    Part(PartName),
}

/// A relationship to a part that is added, as the caller says it: where it
/// is declared and its `Type`. The `Id` and `Target` are made by the edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Where the relationship is declared.
    pub source: LinkSource,
    /// The relationship's `Type`, written as it is given.
    pub rel_type: String,
}

/// A relationship to write, with its `Id` and `Target` settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Pending {
    pub(super) source: LinkSource,
    pub(super) id: String,
    pub(super) rel_type: String,
    /// The target, a reference relative to the source's folder.
    pub(super) target: String,
}

impl LinkSource {
    /// The name of the Relationships part that holds the relationships of
    /// this source (§6.5.2.2).
    pub(super) fn rels_part(&self) -> PartName {
        match self {
            Self::Package => PartName::package_relationships(),
            Self::Part(name) => name.relationships_part(),
        }
    }

    /// Whether this is the same source as `other`: the same part, by the key
    /// of its name.
    pub(super) fn is(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Package, Self::Package) => true,
            (Self::Part(a), Self::Part(b)) => a.key() == b.key(),
            _ => false,
        }
    }

    /// The folders the source's relative references start from: the part
    /// name's segments but the last; none for the package.
    fn folders(&self) -> Vec<&str> {
        match self {
            Self::Package => Vec::new(),
            Self::Part(name) => segments(split(name.as_str()).0),
        }
    }
}

impl<R: Read + Seek> PackageEdit<'_, R> {
    /// The relationship `link` to the part `target`, with the `Id` and
    /// `Target` it gets, once its source is known to be in the package with
    /// all its relationships known.
    pub(super) fn pending(&self, link: Link, target: &PartName) -> Result<Pending, WriteError> {
        let Link { source, rel_type } = link;
        // `XmlRewriter::write` writes the type into XML and does not check its
        // characters.
        if !rel_type.chars().all(is_xml_char) {
            return Err(WriteError::RelationshipTypeInvalid { rel_type });
        }
        // The source is kept as the package spells the part's name, so one
        // part is one source whatever the letter case the caller wrote.
        let (source, known, existing) = match source {
            LinkSource::Package => (
                LinkSource::Package,
                self.package.relationships_known(),
                self.package.relationships(),
            ),
            LinkSource::Part(name) => {
                // A Relationships part has no relationships (§6.5.2.1).
                reserved(&name)?;
                let part = self.live_part(&name)?;
                let source = LinkSource::Part(part.name().clone());
                (source, part.relationships_known(), part.relationships())
            }
        };
        if !known {
            let name = match &source {
                LinkSource::Package => PartName::package_relationships().to_string(),
                LinkSource::Part(name) => name.to_string(),
            };
            return Err(WriteError::RelationshipsUnknown { name });
        }
        // The `Id`s taken number at most the relationships of the source plus
        // the links added so far. The search runs through one number more than
        // that, so it always finds a number whose `Id` is free.
        let numbers = (existing.len().saturating_add(self.links.len()) as u64).saturating_add(1);
        let id = (1..=numbers)
            .map(|number| format!("rId{number}"))
            .find(|id| {
                let earlier = self
                    .links
                    .iter()
                    .any(|l| l.source.is(&source) && l.id == *id);
                !earlier && !existing.iter().any(|relationship| relationship.id() == id)
            })
            .unwrap_or_default();
        let target = relative(&source, target);
        Ok(Pending {
            source,
            id,
            rel_type,
            target,
        })
    }
}

/// The folder of a part name (`/xl` for `/xl/workbook.xml`, empty for a
/// part at the root) and its last segment.
fn split(name: &str) -> (&str, &str) {
    name.rsplit_once('/').unwrap_or(("", name))
}

/// The segments of a path that starts with `/` or is empty.
fn segments(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// `target` as a relative reference from the folder of `source`
/// (RFC 3986 §4.2): the part name without the folders they share, after one
/// `..` for each folder of the source not shared. The name stays in its IRI
/// form. A first segment with a `:` would read as a URI scheme, so it is
/// written after `./`.
pub(super) fn relative(source: &LinkSource, target: &PartName) -> String {
    let from = source.folders();
    let (folder, _) = split(target.as_str());
    let to = segments(folder);
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let all = segments(target.as_str());
    let mut parts: Vec<&str> = vec![".."; from.len().saturating_sub(shared)];
    parts.extend(all.get(shared..).unwrap_or_default());
    let reference = parts.join("/");
    let scheme = parts.first().is_some_and(|first| first.contains(':'));
    if scheme {
        return format!("./{reference}");
    }
    reference
}

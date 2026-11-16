//! [`Tree`]: the parts of a workbook and the relationships between them,
//! what the `tree` view shows, and the JSON every host prints it as.
//!
//! ```json
//! {
//!   "ok": true, "error": null, "diagnostics": [],
//!   "complete": true,
//!   "relationshipsKnown": true,
//!   "relationships": [
//!     { "id": "rId1", "type": "http://.../officeDocument",
//!       "target": { "part": "/xl/workbook.xml" } }
//!   ],
//!   "parts": [
//!     { "name": "/xl/workbook.xml", "contentType": "application/vnd...",
//!       "contentTypeKnown": true, "size": 1234, "compressedSize": 400,
//!       "readable": true, "referenced": true, "relationshipsKnown": true,
//!       "relationships": [] }
//!   ]
//! }
//! ```
//!
//! After `ok`, `error` and `diagnostics` ([`View`]), the keys and their
//! types are the contract; a key may be added, never renamed or retyped.
//! `parts` are in the order of their zip entries and `relationships` in the
//! order of their Relationships part. A name, an `id`, a `type`, a content
//! type and an external target come from the workbook, so each is an
//! [`Echo`]: a value for display, with control characters and bidirectional
//! formatting characters removed and cut to 200 characters (a cut value ends
//! in `…`), so two different names may be written the same.
//!
//! `complete` is the value of [`Package::graph_complete`]: false when a
//! Relationships part was not read, or a relationship was not kept, that may
//! have targeted a part. `relationshipsKnown` is the value of
//! [`Part::relationships_known`](xlsx_graft_package::Part::relationships_known)
//! (for a part) or [`Package::relationships_known`] (at the top, for the
//! package's own relationships): false when the relationships may include one
//! that is not listed. `referenced` is `true` for a part a listed
//! relationship targets; for any other part it is `false` when `complete` is
//! true, and `null` when it is false, since a relationship that is not listed
//! may target the part. A relationship whose target is no part targets none,
//! so it does not make `complete` false (it makes `relationshipsKnown`
//! false).
//!
//! Relationships parts are not listed: their relationships are listed on
//! their source part (`relationships` of the package, or of a part). Only
//! the relationships the package kept are listed; a diagnostic says what it
//! did not keep.

mod build;
mod text;

use crate::open::open_regular;
use crate::policy::limits::HostLimits;
use crate::report::Failure;
use crate::view::View;
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use xlsx_graft_diagnostics::{Diagnostics, Echo};
use xlsx_graft_package::{Package, PackageError};

pub use text::TreeText;

/// What the `tree` view shows of a workbook; its content is read through
/// [`View::to_json`] and [`TreeText`].
#[derive(Debug, Default, Serialize)]
pub struct Tree {
    /// The value of [`Package::graph_complete`]: false when a Relationships
    /// part was not read, or a relationship was not kept, that may have
    /// targeted a part. False when the package could not be opened.
    pub(crate) complete: bool,
    /// The value of [`Package::relationships_known`]: false when the
    /// package's own relationships may include one that is not listed. False
    /// when the package could not be opened.
    #[serde(rename = "relationshipsKnown")]
    pub(crate) relationships_known: bool,
    /// The package's own relationships, from `/_rels/.rels`, that the package
    /// kept.
    pub(crate) relationships: Vec<TreeRelationship>,
    /// Every part, in the order of their zip entries.
    pub(crate) parts: Vec<TreePart>,
}

/// One part of the package.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreePart {
    /// The part's name, such as `/xl/workbook.xml`, as an [`Echo`] (a value
    /// for display: it may be cut, and two cut names may be the same).
    pub(crate) name: Echo,
    /// The part's media type, when the content types stream gives it one.
    pub(crate) content_type: Option<Echo>,
    /// Whether the content types stream gave the part's media type, or said
    /// it has none. When false, a null `contentType` does not mean the part
    /// has none.
    pub(crate) content_type_known: bool,
    /// The size, in bytes, that the zip's central directory declares for
    /// the part's entry once inflated. For an entry that was not read
    /// (`readable` is false) it may be more than 2^53 − 1, which a language
    /// that reads a JSON number as a double cannot read exactly.
    pub(crate) size: u64,
    /// The size, in bytes, that the zip's central directory declares for
    /// the part's entry in the zip; as `size`, it may be more than 2^53 − 1
    /// when `readable` is false.
    pub(crate) compressed_size: u64,
    /// Whether the package reader accepted the part's zip entry: within the
    /// limits, with a supported compression method, and so on.
    pub(crate) readable: bool,
    /// Whether a listed relationship targets the part; when none does, `false`
    /// if [`Tree::complete`] is true and `null` if it is not.
    pub(crate) referenced: Option<bool>,
    /// The value of
    /// [`Part::relationships_known`](xlsx_graft_package::Part::relationships_known):
    /// false when the part's relationships may include one that is not listed.
    pub(crate) relationships_known: bool,
    /// The relationships of the part's Relationships part that the package
    /// kept; all of them when `relationshipsKnown` is true.
    pub(crate) relationships: Vec<TreeRelationship>,
}

/// One relationship of a Relationships part.
#[derive(Debug, Serialize)]
pub struct TreeRelationship {
    /// The relationship's `Id`.
    pub(crate) id: Echo,
    /// The relationship's `Type`, a URI, as written.
    #[serde(rename = "type")]
    pub(crate) rel_type: Echo,
    /// What the relationship points to.
    pub(crate) target: TreeTarget,
}

/// What a relationship points to; written as `{"part": name}` or
/// `{"external": target}`. A part's name is an [`Echo`]: a value for
/// display, which may be cut, so two cut names may be the same.
#[derive(Debug)]
pub struct TreeTarget(pub(crate) Linked);

/// What a [`TreeTarget`] is.
#[derive(Debug)]
pub(crate) enum Linked {
    /// A part of the package.
    Part {
        /// The part's index in [`Tree::parts`]. Not written; `TreeText` uses
        /// it to find the target among [`Tree::parts`].
        index: usize,
        /// The part's name.
        name: Echo,
    },
    /// A resource outside the package: its `Target`, as written.
    External(Echo),
}

impl Serialize for TreeTarget {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        match &self.0 {
            Linked::Part { name, .. } => map.serialize_entry("part", name)?,
            Linked::External(text) => map.serialize_entry("external", text)?,
        }
        map.end()
    }
}

/// The tree of the workbook `reader` holds, under `limits`.
///
/// Bytes that [`Package::open`] does not take as a package (not a zip, and so
/// on) give diagnostics and an empty tree; an I/O error of `reader` is the
/// view's `error` (`io`).
pub fn tree<R: Read + Seek>(reader: R, limits: &HostLimits) -> View<Tree> {
    let mut diagnostics = Diagnostics::with_limit(limits.max_diagnostics);
    let opened = Package::open(reader, &limits.verify.package, &mut diagnostics);
    viewed(
        opened.map(|found| found.map(|p| Tree::from_package(&p))),
        diagnostics,
    )
}

/// The view of what reading a package gave: the tree when the package was
/// opened, an empty tree when it was not (`Ok(None)`, the diagnostics say
/// why), and an empty tree with an `io` `error` when reading failed.
fn viewed(read: Result<Option<Tree>, PackageError>, diagnostics: Diagnostics) -> View<Tree> {
    match read {
        Ok(Some(tree)) => View::finished(tree, diagnostics, None),
        Ok(None) => View::finished(Tree::default(), diagnostics, None),
        Err(PackageError::Io(error)) => {
            let failure = Failure::io(format!("the package could not be read: {error}"));
            View::finished(Tree::default(), diagnostics, Some(failure))
        }
    }
}

/// The tree of the workbook at `path`, as [`tree`] gives it: a file that
/// [`Package::open`] does not take as a package gives diagnostics and an
/// empty tree. A file that cannot be opened, and a path that is not a regular
/// file, are the view's `error` (`io`), as for
/// [`verify_path`](crate::verify_path).
pub fn tree_path(path: &Path, limits: &HostLimits) -> View<Tree> {
    match open_regular(path) {
        Ok(reader) => tree(reader, limits),
        Err(failure) => View::finished(Tree::default(), Diagnostics::new(), Some(failure)),
    }
}

/// The tree of the workbook `bytes` holds, without copying it: bytes that
/// [`Package::open`] does not take as a package give diagnostics and an empty
/// tree.
pub fn tree_bytes(bytes: &[u8], limits: &HostLimits) -> View<Tree> {
    tree(Cursor::new(bytes), limits)
}

#[cfg(test)]
mod tests;

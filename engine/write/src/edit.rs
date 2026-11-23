//! [`PackageEdit`]: the changes to make to a package, collected by
//! [`replace`](PackageEdit::replace), [`add`](PackageEdit::add) and
//! [`remove`](PackageEdit::remove), and written as a new zip by
//! [`finish`](PackageEdit::finish).
//!
//! The calls check what the package already tells (a part is there, a name
//! is free, a relationship's source is known) and keep the changes; nothing
//! is read from a part or written until `finish` (`edit/finish.rs`), which
//! plans the output from them (`edit/plan.rs`). A new relationship gets its
//! `Id` and `Target` when `add` is called (`edit/link.rs`). `finish` rewrites the
//! content types stream (`edit/content_types.rs`) and each Relationships
//! part the edit changes (`edit/relationships.rs`), both with the same walk
//! over the children of the root (`edit/children.rs`).

mod children;
mod content_types;
mod finish;
mod link;
mod names;
mod plan;
mod relationships;

pub use link::{Link, LinkSource};

use crate::error::WriteError;
use crate::part::{PartRewriter, PartWriter};
use link::Pending;
use names::{Names, Origin};
use std::collections::BTreeMap;
use std::io::{Read, Seek};
use xlsx_graft_package::{ContentType, Package, Part, PartName, Presence};

/// A part to add to the package.
struct Added<'r> {
    name: PartName,
    content_type: ContentType,
    writer: Box<dyn PartWriter + 'r>,
}

/// The changes to make to a package, written as a new zip by
/// [`finish`](Self::finish).
///
/// What is written, in the order of the zip it is written to (the
/// crate root says what the zip holds):
///
/// - The entries of the original package, in their order, each copied as it
///   is, unless the edit changes it: a part replaced is written from its
///   rewriter, a part removed and its Relationships part are left out, and
///   so are the relationships that target a part removed. The content types
///   stream and a Relationships part that the edit changes are read and
///   written again with only the change (the bytes outside it as they
///   were).
/// - Then the parts added, in the order they were added, each with an
///   `Override` in the content types stream, then the Relationships parts
///   made for a source that had none, in the order of the first link to it.
///
/// The lifetime `'r` is how long the rewriters and writers the edit holds
/// live, so they may borrow values of the caller's.
pub struct PackageEdit<'r, R> {
    package: Package<R>,
    /// The rewriters, by the index of the zip entry of the part they
    /// rewrite.
    replaced: BTreeMap<usize, Box<dyn PartRewriter + 'r>>,
    added: Vec<Added<'r>>,
    /// The relationships to add, in the order of the calls.
    links: Vec<Pending>,
    /// The part names the written package will hold, and which changed.
    names: Names,
}

impl<'r, R: Read + Seek> PackageEdit<'r, R> {
    /// An edit of `package` that changes nothing yet.
    #[must_use]
    pub fn new(package: Package<R>) -> Self {
        let names = Names::of(&package);
        Self {
            package,
            names,
            replaced: BTreeMap::new(),
            added: Vec::new(),
            links: Vec::new(),
        }
    }

    /// Writes the part `name` from `rewriter`, which gets the part's
    /// original content. A second `replace` of the same part takes the
    /// place of the first: only the last rewriter is called.
    ///
    /// # Errors
    ///
    /// [`WriteError::PartReserved`] for a Relationships part;
    /// [`WriteError::PartMissing`] when the package has no part of that
    /// name or it was removed.
    pub fn replace(
        &mut self,
        name: &PartName,
        rewriter: Box<dyn PartRewriter + 'r>,
    ) -> Result<(), WriteError> {
        reserved(name)?;
        let entry = self.live_part(name)?.entry();
        self.replaced.insert(entry, rewriter);
        Ok(())
    }

    /// Adds a part `name` of `content_type`, written by `writer`, and
    /// with `link` a relationship to it from the link's source.
    ///
    /// Returns the `Id` of the relationship added, `rId` and the least
    /// number that no relationship of its source has, and none added by an
    /// earlier call. The `Target` is the part's name relative to the source.
    /// The content type is written as an `Override`, whether or not a
    /// `Default` gives the part the same type, and any `Override` of the
    /// stream for the name is replaced by it.
    ///
    /// A name of a part removed in this edit cannot be added again: use
    /// [`replace`](Self::replace) to give the part new content.
    ///
    /// Nothing changes when the call returns an error.
    ///
    /// # Errors
    ///
    /// [`WriteError::PartReserved`] for a Relationships part's name.
    ///
    /// [`WriteError::PartExists`] when a name the call would take is not free.
    /// The names are the part's and, for a `link` whose source has no
    /// Relationships part, the name of the one that would be made; `name` of
    /// the error is the first of them that is refused. One is refused when it
    /// is equivalent to a name the written package would hold, ASCII letter
    /// case aside, or is a folder of one or has one as its folder
    /// (`/a/b.xml/c.xml` and `/a/b.xml`; ECMA-376-2:2021 §6.2.2.3): the names
    /// of the package's parts, of the entries it refused for their names, of
    /// the parts added and of the Relationships parts to be made. The name of
    /// a part removed is refused too, and so is a part whose own
    /// Relationships part's name (`/<folder>/_rels/<name>.rels`) is a name of
    /// the package already, since that part would be its relationships;
    /// `name` of the error is then that Relationships part's name.
    ///
    /// For `link`: [`WriteError::PartMissing`] when its source part is not
    /// in the package or was removed, [`WriteError::PartReserved`] when its
    /// source is a Relationships part,
    /// [`WriteError::RelationshipTypeInvalid`] when its `rel_type` has a
    /// character XML does not allow, and
    /// [`WriteError::RelationshipsUnknown`] when the relationships of its
    /// source are not all known.
    pub fn add(
        &mut self,
        name: PartName,
        content_type: ContentType,
        writer: Box<dyn PartWriter + 'r>,
        link: Option<Link>,
    ) -> Result<Option<String>, WriteError> {
        reserved(&name)?;
        // A part of the package, whether or not this edit removed it: the
        // table of names no longer holds the name of a part removed, and a
        // removed part's name cannot be added again. A name the package
        // refused an entry for is an `Occupied` row of the table, which
        // `occupy` refuses.
        if matches!(self.package.find(&name), Presence::Part(_)) {
            return Err(WriteError::PartExists {
                name: name.to_string(),
            });
        }
        // Refuses the part when the table holds the name of its own
        // Relationships part (`/<folder>/_rels/<name>.rels`): the package
        // written would read that part as the relationships of this part,
        // which the caller did not ask for.
        let rels = name.relationships_part();
        if self.names.get(&rels).is_some() {
            return Err(WriteError::PartExists {
                name: rels.to_string(),
            });
        }
        let pending = link.map(|link| self.pending(link, &name)).transpose()?;
        let mut taking = vec![(name.clone(), Origin::Added(self.added.len()))];
        if let Some(pending) = &pending {
            // A link writes to the Relationships part of its source when the
            // table holds one that is a part of the package or was made by an
            // earlier link. For any other row under that name (an entry the
            // package refused) `add` takes the name again, so `occupy` refuses
            // it with `PartExists`. The refusal by `occupy` is not reached
            // with the package reader as it is: an `Occupied` Relationships
            // part makes `relationships_known` false, so `pending` returns
            // `RelationshipsUnknown` first.
            let rels = pending.source.rels_part();
            let held = matches!(
                self.names.get(&rels),
                Some(Origin::Original(_) | Origin::NewRelationships(_))
            );
            if !held {
                let made = Origin::NewRelationships(pending.source.clone());
                taking.push((rels, made));
            }
        }
        self.names.occupy(taking)?;
        let id = pending.as_ref().map(|pending| pending.id.clone());
        self.links.extend(pending);
        self.added.push(Added {
            name,
            content_type,
            writer,
        });
        Ok(id)
    }

    /// Leaves out the part `name`, its Relationships part, their
    /// `Override`s in the content types stream, and each relationship that
    /// targets the part. A part it targets is kept, whatever else it was
    /// for. A rewriter given to [`replace`](Self::replace) for the part is
    /// dropped, and so is a link to a part added, from the part removed (the
    /// part added stays, with no relationship from it).
    ///
    /// # Errors
    ///
    /// [`WriteError::PartReserved`] for a Relationships part;
    /// [`WriteError::PartMissing`] when the package has no part of that
    /// name or it was removed; [`WriteError::GraphIncomplete`] when a
    /// Relationships part of the package was not read whole, so
    /// the relationships that target the part are not all known.
    pub fn remove(&mut self, name: &PartName) -> Result<(), WriteError> {
        reserved(name)?;
        let part = self.live_part(name)?;
        let (entry, name) = (part.entry(), part.name().clone());
        if !self.package.graph_complete() {
            return Err(WriteError::GraphIncomplete);
        }
        // The part and its Relationships part are not in the package written.
        self.names.vacate(&name);
        self.names.vacate(&name.relationships_part());
        self.replaced.remove(&entry);
        Ok(())
    }

    /// The part `name` of the package, unless it was removed.
    fn live_part(&self, name: &PartName) -> Result<&Part, WriteError> {
        let live =
            (self.package.part_named(name)).filter(|p| !self.names.vacated().contains(&p.entry()));
        live.ok_or_else(|| WriteError::PartMissing {
            name: name.to_string(),
        })
    }
}

/// `Err` for a Relationships part's name, which only the edit writes.
fn reserved(name: &PartName) -> Result<(), WriteError> {
    if name.is_relationships() {
        return Err(WriteError::PartReserved {
            name: name.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;

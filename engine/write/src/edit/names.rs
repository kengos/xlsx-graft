//! [`Names`]: the one table of the part names the written package will hold.
//!
//! Every name in the output, whether a part of the package, a name a
//! refused entry takes, a part added or a Relationships part to be made, is a
//! row of the table, keyed by [`PartName::key`]. The three things the output
//! is made of are worked out from this table alone (`edit/plan.rs`): the zip
//! entries, the `Override`s of the content types stream and the Relationships
//! parts. The table has two mutators, [`occupy`](Names::occupy) and
//! [`vacate`](Names::vacate), and each records the keys it touched, so no
//! name can change without its `Override` being rewritten.
//!
//! Names must not collide (ECMA-376-2:2021 §6.2.2.3): a part name is not
//! equivalent to another's, and not a folder of another's (`/a/b.xml` and
//! `/a/b.xml/c.xml`). [`occupy`](Names::occupy) refuses a name that does,
//! against every row, so no call can make a package the reader refuses for names that overlap.

use super::link::LinkSource;
use crate::error::WriteError;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek};
use xlsx_graft_package::{Package, PartName};

/// Where a name of the table comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Origin {
    /// A part of the package, in this zip entry.
    Original(usize),
    /// A name no part has but the package refused an entry for (the name is
    /// the same part as another's, or derived from another's, or is no part
    /// name): taken, with nothing to write.
    Occupied,
    /// The part added by the call of this index (in the order of the calls).
    Added(usize),
    /// A Relationships part to make, for the relationships a link adds to
    /// this source, which has none.
    NewRelationships(LinkSource),
}

/// The names the written package will hold, and what changed.
#[derive(Debug)]
pub(super) struct Names {
    table: BTreeMap<String, Origin>,
    /// The keys of every name occupied or vacated since the table was made:
    /// the `Override`s of these are rewritten.
    touched: BTreeSet<String>,
    /// The zip entries of the parts of the package that were vacated.
    vacated: BTreeSet<usize>,
}

impl Names {
    /// The table of `package`: its parts, and the keys of the names its
    /// refused entries take.
    pub(super) fn of<R: Read + Seek>(package: &Package<R>) -> Self {
        let mut table = BTreeMap::new();
        for key in package.unknown_keys() {
            table.insert(key.to_owned(), Origin::Occupied);
        }
        for part in package.parts() {
            let key = part.name().key().to_owned();
            table.insert(key, Origin::Original(part.entry()));
        }
        Self {
            table,
            touched: BTreeSet::new(),
            vacated: BTreeSet::new(),
        }
    }

    /// Where `name` comes from, if the table holds it.
    pub(super) fn get(&self, name: &PartName) -> Option<&Origin> {
        self.table.get(name.key())
    }

    /// The keys of every name occupied or vacated.
    pub(super) fn touched(&self) -> &BTreeSet<String> {
        &self.touched
    }

    /// The zip entries of the parts of the package vacated.
    pub(super) fn vacated(&self) -> &BTreeSet<usize> {
        &self.vacated
    }

    /// Takes all of `names`, or none. A name is refused when it is equivalent
    /// to a name of the table, or to another of `names`, or is a folder of
    /// one or has one as a folder.
    ///
    /// # Errors
    ///
    /// [`WriteError::PartExists`] naming the first of `names` that is
    /// refused; the table is as it was.
    pub(super) fn occupy(&mut self, names: Vec<(PartName, Origin)>) -> Result<(), WriteError> {
        for (at, (name, _)) in names.iter().enumerate() {
            let key = name.key();
            let before = (names.get(..at).unwrap_or_default().iter()).map(|(n, _)| n.key());
            let taken = self.table.keys().map(String::as_str).chain(before);
            if taken.into_iter().any(|other| related(key, other)) {
                return Err(WriteError::PartExists {
                    name: name.to_string(),
                });
            }
        }
        for (name, origin) in names {
            self.touched.insert(name.key().to_owned());
            self.table.insert(name.key().to_owned(), origin);
        }
        Ok(())
    }

    /// Takes `name` out of the table, if it is a part of the package or a
    /// part or Relationships part of the edit: the written package does not
    /// hold it, and the zip entry of a part of the package is left out. A name
    /// taken by an entry that is no part is left taken.
    pub(super) fn vacate(&mut self, name: &PartName) {
        // A name that an entry took with no part stays taken, since the entry
        // stays in the zip. Any other row goes, and a part of the package is
        // vacated with its entry.
        if self.table.get(name.key()) != Some(&Origin::Occupied)
            && let Some(Origin::Original(entry)) = self.table.remove(name.key())
        {
            self.vacated.insert(entry);
        }
        self.touched.insert(name.key().to_owned());
    }
}

/// Whether the part names with keys `a` and `b` cannot both be in a package:
/// they are equivalent, or one is the other and `/` and one or more segments
/// (§6.2.2.3).
fn related(a: &str, b: &str) -> bool {
    let folder = |long: &str, short: &str| {
        (long.strip_prefix(short)).is_some_and(|rest| rest.starts_with('/'))
    };
    a == b || folder(a, b) || folder(b, a)
}

//! [`ChildPlacer`]: before which child of the root each kind a caller
//! writes is placed, as the caller reads the root's children in document
//! order.

use super::WorksheetChild;
use std::collections::BTreeMap;

/// What the placer knows of a wanted kind.
#[derive(Debug, Clone, Copy)]
struct Seen {
    /// An element of the kind started.
    found: bool,
    /// `start` or `finish` has returned the kind in a [`Due`]; this happens
    /// at most once for each kind.
    due: bool,
}

/// Says where the kinds of child a caller writes go among a worksheet's
/// root children, in the order of [`WorksheetChild`].
///
/// The caller names the kinds first, then asks at each child of the root
/// ([`start`](Self::start)) and at the root's end ([`finish`](Self::finish)).
/// To replace a kind, do not write the child that is [`Slot::Wanted`] (skip
/// it in the output) and write the new one at the [`Due`] with
/// `found: true`. To edit inside a kind, copy the `Wanted` child through,
/// changing it, and do nothing at the `Due` that has `found: true`; at one
/// with `found: false`, write the kind new. A kind whose child was
/// [`Slot::Wrapped`] is written unchanged inside its wrapper; write nothing
/// at its `Due`. To write a new sheet, call `new`, write the `due` of
/// `start(RootChild::Table(SheetData))`, write `sheetData`, then write what
/// `finish` returns.
///
/// A kind the part has is due right after its run of elements ends: at the
/// start of the next root child, or at `finish` when no child follows. A kind
/// the part lacks is due right before the first [`RootChild::Table`] or
/// [`RootChild::Wrapped`] child that comes after the kind in the table order,
/// or at `finish` when there is none. A [`RootChild::Wrapped`] child counts
/// as a child of its kind, so a missing kind that comes before the wrapped
/// kind is due before the wrapper.
///
/// A kind that comes several times in a row is [`Slot::Wanted`] each time,
/// whether or not the schema lets it repeat. The placer does not call
/// [`WorksheetChild::repeats`]; a caller that must not write a second
/// element of a kind that does not repeat checks it.
///
/// Limits: a [`RootChild::Other`] element has no place in the table order,
/// so a kind missing from the part is due at the next `Table` or `Wrapped`
/// child of a later kind, and may be written after an `Other` element that a
/// writer knowing that element's schema would place after the kind. A part
/// whose children are out of the table order gives [`Slot::Late`] for a child
/// whose kind is already due.
#[derive(Debug, Clone)]
pub struct ChildPlacer {
    wanted: BTreeMap<WorksheetChild, Seen>,
}

/// A child of the root, as the caller reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootChild {
    /// An element of the main namespace that is a child of the table.
    Table(WorksheetChild),
    /// An `mc:AlternateContent` whose first `mc:Choice` has, as its first
    /// element child, a child of the table of this kind (Excel wraps
    /// `oleObjects` and `controls` this way).
    Wrapped(WorksheetChild),
    /// An element of another namespace, or a wrapper whose kind is not
    /// known: it has no place in the table order.
    Other,
}

impl RootChild {
    /// The kind of the table the child is, or wraps.
    const fn kind(self) -> Option<WorksheetChild> {
        match self {
            Self::Table(kind) | Self::Wrapped(kind) => Some(kind),
            Self::Other => None,
        }
    }
}

/// What [`ChildPlacer::start`] returns for a child of the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The wanted kinds whose position in the table order has been reached,
    /// in table order: write them (or what to append) before this child.
    pub due: Vec<Due>,
    /// What this child is to the caller.
    pub slot: Slot,
}

/// What a child of the root is to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Not wanted, or not in the table: write it as it is, where it is.
    Pass,
    /// A wanted kind: the caller replaces it, edits inside it, or keeps it
    /// and appends after the last one.
    Wanted,
    /// A wanted kind wrapped in `mc:AlternateContent`: the caller writes the
    /// wrapper unchanged where it is. The kind counts as found, so the caller
    /// must not write a second element of the kind.
    Wrapped,
    /// A wanted kind that comes after its position was passed (a part whose
    /// children are out of order): the caller skips it or writes it
    /// unchanged, but must not write it twice.
    Late,
}

/// A wanted kind whose position in the table order has been reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Due {
    /// The wanted kind of child.
    pub child: WorksheetChild,
    /// Whether an element of the kind started before.
    pub found: bool,
}

impl ChildPlacer {
    /// A placer for the kinds of child the caller writes, replaces, edits or
    /// appends to.
    pub fn new(wanted: impl IntoIterator<Item = WorksheetChild>) -> Self {
        let seen = Seen {
            found: false,
            due: false,
        };
        Self {
            wanted: wanted.into_iter().map(|child| (child, seen)).collect(),
        }
    }

    /// A child of the root starts. The returned `due` lists the wanted kinds
    /// not yet due that the child does not itself hold, in table order: a
    /// kind the part has (an element of it started before) and a kind that
    /// comes before the child's kind in the table order. An
    /// [`RootChild::Other`] child makes only the kinds the part has due.
    pub fn start(&mut self, child: RootChild) -> Placement {
        let kind = child.kind();
        let due = self.take_due(|wanted, seen| {
            (seen.found && Some(wanted) != kind) || kind.is_some_and(|k| wanted < k)
        });
        let slot = match kind.and_then(|k| self.wanted.get_mut(&k)) {
            None => Slot::Pass,
            Some(seen) if seen.due => Slot::Late,
            Some(seen) => {
                seen.found = true;
                match child {
                    RootChild::Wrapped(_) => Slot::Wrapped,
                    _ => Slot::Wanted,
                }
            }
        };
        Placement { due, slot }
    }

    /// The root ends: every wanted kind not yet due, in table order.
    pub fn finish(&mut self) -> Vec<Due> {
        self.take_due(|_, _| true)
    }

    /// Marks the kinds not due yet that `pick` accepts as due and returns
    /// them in table order.
    fn take_due(&mut self, pick: impl Fn(WorksheetChild, &Seen) -> bool) -> Vec<Due> {
        self.wanted
            .iter_mut()
            .filter(|(child, seen)| !seen.due && pick(**child, seen))
            .map(|(&child, seen)| {
                seen.due = true;
                Due {
                    child,
                    found: seen.found,
                }
            })
            .collect()
    }
}

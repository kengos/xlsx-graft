//! [`Change`]: a rename or a delete of a sheet, as `apply` applies it to the
//! formulas of the defined names and of the other parts, and what a delete
//! keeps of the sheets (`Deleted`, `SheetIds`).
//!
//! A formula names a sheet by its name, letter case aside. A name that a
//! sheet has had is given a number once (`SheetIds`), so what a delete needs
//! of each sheet (before the deleted one, the deleted one, after it, or not
//! a sheet) is 2 bits for each name rather than the sheet's name: a delete
//! takes `names / 4` bytes, whatever the number of deletes before it.

use super::state::SheetEntry;
use std::collections::BTreeMap;
use std::sync::Arc;

/// What happened to a sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Change {
    /// The sheet `old` is now named `new`.
    Rename {
        /// The old name, as the workbook spelled it.
        old: Arc<str>,
        /// The new name.
        new: String,
    },
    /// A sheet is deleted.
    Delete(Deleted),
}

impl Change {
    /// The name of the sheet the change is about, as the workbook spelled it
    /// before the change: the old name of a rename, the name of the sheet
    /// deleted.
    pub(super) fn sheet(&self) -> &Arc<str> {
        match self {
            Self::Rename { old, .. } => old,
            Self::Delete(deleted) => &deleted.name,
        }
    }
}

/// Where a sheet was when another was deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    /// No sheet has the name.
    Absent = 0,
    /// The sheet is before the deleted one.
    Before = 1,
    /// The sheet is the deleted one.
    Deleted = 2,
    /// The sheet is after the deleted one.
    After = 3,
}

impl Side {
    /// The 2 bits that encode the side: its number above.
    fn bits(self) -> u8 {
        self as u8
    }

    /// The side that the lowest 2 bits of `bits` encode.
    fn from_bits(bits: u8) -> Self {
        match bits & 3 {
            0 => Self::Absent,
            1 => Self::Before,
            2 => Self::Deleted,
            _ => Self::After,
        }
    }
}

/// A sheet deleted: the [`Side`] of each name, and the numbers of the names of
/// the sheets beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Deleted {
    /// The deleted sheet's name, as the workbook spelled it.
    pub(super) name: Arc<str>,
    /// The [`Side`] of each name, by [`SheetIds`] number, 2 bits each, 4 to
    /// a byte, the lowest bits first. A name numbered after the delete is
    /// [`Side::Absent`].
    pub(super) sides: Vec<u8>,
    /// The number of the name of the sheet before the deleted one, if any.
    /// Its spelling is read from [`SheetIds::spelling`], so a delete holds
    /// no text of a sheet's name beyond the deleted sheet's own.
    pub(super) prev: Option<u32>,
    /// The number of the name of the sheet after the deleted one, if any.
    pub(super) next: Option<u32>,
}

impl Deleted {
    /// The delete of the sheet at `position` of `sheets`, the sheets in the
    /// order they have before it. When two sheets have the same name, letter
    /// case aside, the side of the name is the one of the first of them.
    pub(super) fn new(sheets: &[SheetEntry], position: usize, ids: &SheetIds) -> Self {
        let mut sides = vec![0; ids.len().div_ceil(4)];
        for (index, sheet) in sheets.iter().enumerate() {
            if side_of(&sides, sheet.name_id) != Side::Absent {
                continue;
            }
            let side = match index.cmp(&position) {
                std::cmp::Ordering::Less => Side::Before,
                std::cmp::Ordering::Equal => Side::Deleted,
                std::cmp::Ordering::Greater => Side::After,
            };
            let slot = sides.get_mut(slot_of(sheet.name_id));
            slot.into_iter()
                .for_each(|byte| *byte |= side.bits() << shift_of(sheet.name_id));
        }
        let beside =
            |at: Option<usize>| at.and_then(|at| sheets.get(at)).map(|sheet| sheet.name_id);
        Self {
            name: Arc::from(sheets.get(position).map_or("", |sheet| sheet.name.as_str())),
            sides,
            prev: beside(position.checked_sub(1)),
            next: beside(position.checked_add(1)),
        }
    }

    /// The side of the sheet called `name`, letter case aside.
    pub(super) fn side(&self, ids: &SheetIds, name: &str) -> Side {
        ids.get(name)
            .map_or(Side::Absent, |id| side_of(&self.sides, id))
    }
}

/// The index in `sides` of the byte that holds the side of the name `id`.
fn slot_of(id: u32) -> usize {
    usize::try_from(id / 4).unwrap_or(usize::MAX)
}

/// How far the side of the name `id` is shifted in its byte.
fn shift_of(id: u32) -> u32 {
    (id % 4) * 2
}

/// The side of the name `id` in `sides`; [`Side::Absent`] when `id` is
/// beyond the end of `sides`.
fn side_of(sides: &[u8], id: u32) -> Side {
    let byte = sides.get(slot_of(id)).copied().unwrap_or(0);
    Side::from_bits(byte >> shift_of(id))
}

/// The names that sheets have had, each numbered once, letter case aside.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct SheetIds {
    /// The number of each name, in lower case.
    ids: BTreeMap<String, u32>,
    /// The latest spelling of each name, by number.
    spellings: Vec<String>,
}

impl SheetIds {
    /// Returns the number of the name `name`, numbering it when no sheet has
    /// had it, and records `name` as the latest spelling of the name.
    pub(super) fn id(&mut self, name: &str) -> u32 {
        let folded = name.to_lowercase();
        let next = u32::try_from(self.spellings.len()).unwrap_or(u32::MAX);
        let id = *self.ids.entry(folded).or_insert(next);
        match self.spellings.get_mut(slot(id)) {
            Some(spelling) => name.clone_into(spelling),
            None => self.spellings.push(name.to_owned()),
        }
        id
    }

    /// The number of the name `name`, letter case aside, if a sheet has had it.
    pub(super) fn get(&self, name: &str) -> Option<u32> {
        self.ids.get(&name.to_lowercase()).copied()
    }

    /// The spelling of the name `id`, the latest a sheet has had; `""` for a
    /// number no name has.
    pub(super) fn spelling(&self, id: u32) -> &str {
        self.spellings.get(slot(id)).map_or("", String::as_str)
    }

    /// How many names are numbered.
    pub(super) fn len(&self) -> usize {
        self.spellings.len()
    }
}

/// The index of the name `id` in [`SheetIds`]'s spellings.
fn slot(id: u32) -> usize {
    usize::try_from(id).unwrap_or(usize::MAX)
}

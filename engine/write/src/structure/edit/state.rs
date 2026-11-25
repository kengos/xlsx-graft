//! The state a [`StructureEdit`](super::StructureEdit) edits: the sheets in
//! their present order, the sheets deleted, the defined names, the view
//! pointers and the flags `apply` reads, built from the
//! [`Workbook`](super::super::model::Workbook) `open` read.
//!
//! A sheet keeps one `key` from `open` to `apply`, whatever happens to its
//! position, so a defined name scoped to a sheet and a view that points at
//! one name it by key and keep naming it after a move or a delete. The calls
//! (`sheets.rs`, `names.rs`, `formula.rs`, `views.rs`) change this state as
//! they are made; `apply` only writes it out.

use super::change::{Change, SheetIds};
use crate::PartWriter;
use crate::structure::model::{SheetKind, SheetState, Workbook};
use std::collections::BTreeSet;
use std::convert::Infallible;
use std::sync::Arc;
use xlsx_graft_cell::escape;

/// Where a sheet's part is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Origin {
    /// A sheet of the workbook that was opened.
    Original {
        /// The `Id` of the workbook part's relationship to it.
        relationship_id: String,
        /// The index of its part in `Package::parts`.
        part: usize,
        /// What kind of sheet it is.
        kind: SheetKind,
    },
    /// A sheet added by this edit: the position of its content in
    /// [`State::added`].
    Added {
        /// The position of its content in [`State::added`].
        index: usize,
    },
}

/// One sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SheetEntry {
    /// The sheet's identity: `open` numbers the sheets from 0, each sheet
    /// added takes the next number, and nothing changes one.
    pub(super) key: usize,
    /// Its name, its `_xHHHH_` escapes undone.
    pub(super) name: String,
    /// The number of its name in [`State::sheet_ids`].
    pub(super) name_id: u32,
    /// Whether the name was changed after `open`.
    pub(super) renamed: bool,
    /// Its `sheetId`.
    pub(super) sheet_id: u32,
    /// Its visibility (`state`).
    pub(super) state: SheetState,
    /// Where its part is.
    pub(super) origin: Origin,
}

/// The sheet a defined name is scoped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    /// The workbook.
    Book,
    /// The sheet with this key.
    Sheet(usize),
    /// A `localSheetId` that names no sheet of the workbook that was opened:
    /// written as it was, whatever happens to the sheets. Such a name is
    /// never the one `set_defined_name` replaces: when the workbook has a sheet
    /// at that position, a name of the same spelling set for that sheet is
    /// added beside it, and both are written with the same `localSheetId`.
    Raw(u32),
}

/// A defined name's formula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Text {
    /// As the workbook has it: with the `_xHHHH_` escapes still in it.
    Original(String),
    /// Changed by this edit: with the `_xHHHH_` escapes undone.
    Changed(String),
}

/// One `definedName`, in the order it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NameEntry {
    /// The `name` as it is written, its `_xHHHH_` escapes in it.
    pub(super) written: String,
    /// The sheet it is scoped to.
    pub(super) scope: Scope,
    /// Its formula.
    pub(super) text: Text,
    /// The index in `Workbook::defined_names` of the definedName it came
    /// from, `None` for one this edit added.
    pub(super) origin: Option<usize>,
    /// Whether a replacement changed how the name is spelled.
    pub(super) name_changed: bool,
    /// The sheet of the first delete that left a prefix of its sheet in
    /// the formula as written, because the prefix has no operand
    /// (`KeptReason::UnknownOperand`).
    pub(super) unresolved: Option<Arc<str>>,
}

/// Which sheet a view points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Pointer {
    /// The sheet with this key.
    Key(usize),
    /// A position past the sheets of the workbook that was opened: not
    /// touched.
    Raw,
}

/// One `workbookView`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ViewEntry {
    /// Its `activeTab`.
    pub(super) active: Pointer,
    /// Its `firstSheet`.
    pub(super) first: Pointer,
}

/// What the calls of a [`StructureEdit`](super::StructureEdit) have made of
/// the workbook so far.
pub(super) struct State<'r> {
    /// The sheets, in the order they are written.
    pub(super) sheets: Vec<SheetEntry>,
    /// The sheets deleted, in the order they were.
    pub(super) deleted: Vec<SheetEntry>,
    /// The key the next sheet added takes.
    pub(super) next_key: usize,
    /// The `sheetId` the next sheet added takes: one past the largest the
    /// workbook that was opened had, `None` when that is past `u32::MAX`.
    /// Deleting a sheet does not give its id back.
    pub(super) next_sheet_id: Option<u32>,
    /// The content of each sheet added, by [`Origin::Added`]'s `index`;
    /// `None` for the default content.
    pub(super) added: Vec<Option<Box<dyn PartWriter + 'r>>>,
    /// The defined names, in the order they are written.
    pub(super) names: Vec<NameEntry>,
    /// The indexes in `Workbook::defined_names` of the defined names removed.
    pub(super) removed_names: BTreeSet<usize>,
    /// The workbook views, in order.
    pub(super) views: Vec<ViewEntry>,
    /// The names of the tables and their display names, which no defined
    /// name may have.
    pub(super) tables: Vec<String>,
    /// Whether a delete wrote `#REF!` into a formula.
    pub(super) ref_written: bool,
    /// The renames and deletes made, in the order of the calls, each as the
    /// `Change` that rewrote the defined names' formulas: what `apply` writes
    /// into the formulas of the other parts. A move and an add are no change
    /// to a formula.
    pub(super) changes: Vec<Change>,
    /// The names the sheets have had, which a delete in `changes` numbers
    /// the sheets by.
    pub(super) sheet_ids: SheetIds,
}

impl State<'_> {
    /// The state of `book`, before any call.
    pub(super) fn new(book: &Workbook) -> Self {
        let len = book.sheets().len();
        let mut sheet_ids = SheetIds::default();
        let sheets = (book.sheets().iter().enumerate())
            .map(|(key, sheet)| SheetEntry {
                key,
                name: sheet.name().to_owned(),
                name_id: sheet_ids.id(sheet.name()),
                renamed: false,
                sheet_id: sheet.sheet_id(),
                state: sheet.state(),
                origin: Origin::Original {
                    relationship_id: sheet.relationship_id().to_owned(),
                    part: sheet.part(),
                    kind: sheet.kind(),
                },
            })
            .collect();
        let names = (book.defined_names().iter().enumerate())
            .map(|(index, name)| NameEntry {
                written: name.name().to_owned(),
                scope: name
                    .local_sheet_id()
                    .map_or(Scope::Book, |id| scope_of(id, len)),
                text: Text::Original(name.text().to_owned()),
                origin: Some(index),
                name_changed: false,
                unresolved: None,
            })
            .collect();
        let views = (book.views().iter())
            .map(|view| ViewEntry {
                active: pointer(view.active_tab(), len),
                first: pointer(view.first_sheet(), len),
            })
            .collect();
        let tables = (book.tables().iter())
            .flat_map(|table| [table.name(), table.display_name()])
            .flatten()
            .map(str::to_owned)
            .collect();
        Self {
            sheets,
            deleted: Vec::new(),
            next_key: len,
            next_sheet_id: (book.sheets().iter().map(|sheet| sheet.sheet_id()).max())
                .map_or(Some(1), |most| most.checked_add(1)),
            added: Vec::new(),
            names,
            removed_names: BTreeSet::new(),
            views,
            tables,
            ref_written: false,
            changes: Vec::new(),
            sheet_ids,
        }
    }
}

/// The scope a `localSheetId` of `id` gives among `len` sheets.
fn scope_of(id: u32, len: usize) -> Scope {
    let index = usize::try_from(id).ok().filter(|&index| index < len);
    index.map_or(Scope::Raw(id), Scope::Sheet)
}

/// The pointer of an `activeTab` or `firstSheet` of `value` (0 when not
/// written) among `len` sheets.
fn pointer(value: Option<u32>, len: usize) -> Pointer {
    let index = usize::try_from(value.unwrap_or(0))
        .ok()
        .filter(|&index| index < len);
    index.map_or(Pointer::Raw, Pointer::Key)
}

/// `text` as an ST_Xstring value: its `_xHHHH_` escapes made, the form a
/// defined name's `name` and text are compared and written in.
pub(super) fn escaped(text: &str) -> String {
    let mut out = String::new();
    escape::<Infallible>(text, |piece| {
        out.push_str(piece);
        Ok(())
    })
    .unwrap_or_default();
    out
}

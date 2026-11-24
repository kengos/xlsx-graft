//! [`Plan::build`]: what the state says to write of the workbook part.

use super::{NameOut, Plan, SheetOut, ViewOut};
use crate::structure::edit::state::{Origin, Pointer, Scope, State, Text, escaped};
use crate::structure::model::Workbook;
use std::collections::{BTreeMap, BTreeSet};

impl Plan {
    /// The plan for `state`, over `book` as it was opened, `rids` being the
    /// `Id` of the relationship made for each sheet added, by its
    /// [`Origin::Added`] index.
    pub(in crate::structure::edit) fn build(
        state: &State<'_>,
        book: &Workbook,
        rids: &BTreeMap<usize, String>,
    ) -> Self {
        let position = |key: usize| {
            (state.sheets.iter())
                .position(|sheet| sheet.key == key)
                .unwrap_or(0)
        };
        // The sheets changed when the list differs from the opened one in
        // which sheets it holds (by key), their order or their names: a sheet
        // added and deleted, a move to the same place and a rename to the
        // same name change nothing.
        let opened = (book.sheets().iter().enumerate()).map(|(key, sheet)| (key, sheet.name()));
        let now = (state.sheets.iter()).map(|entry| (entry.key, entry.name.as_str()));
        let sheets = now.ne(opened).then(|| {
            (state.sheets.iter())
                .map(|entry| match &entry.origin {
                    Origin::Original { .. } => SheetOut::Original {
                        index: entry.key,
                        name: entry.renamed.then(|| escaped(&entry.name)),
                    },
                    Origin::Added { index } => SheetOut::Added {
                        name: escaped(&entry.name),
                        sheet_id: entry.sheet_id,
                        rid: rids.get(index).cloned().unwrap_or_default(),
                    },
                })
                .collect()
        });
        let mut rebuilt = BTreeMap::new();
        let mut added = Vec::new();
        for entry in &state.names {
            let local = match entry.scope {
                Scope::Book => None,
                Scope::Sheet(key) => Some(u32::try_from(position(key)).unwrap_or(u32::MAX)),
                Scope::Raw(id) => Some(id),
            };
            let out = NameOut {
                name: entry.written.clone(),
                local,
                text: match &entry.text {
                    Text::Original(text) => text.clone(),
                    Text::Changed(text) => escaped(text),
                },
            };
            let original = entry
                .origin
                .and_then(|index| Some((index, book.defined_names().get(index)?)));
            match original {
                Some((index, name)) => {
                    let same = matches!(entry.text, Text::Original(_))
                        && !entry.name_changed
                        && local == name.local_sheet_id();
                    if !same {
                        rebuilt.insert(index, out);
                    }
                }
                None => added.push(out),
            }
        }
        let views = (state.views.iter().enumerate())
            .map(|(index, view)| {
                let original = book.views().get(index);
                ViewOut {
                    active: moved(
                        view.active,
                        original.and_then(|v| v.active_tab()),
                        &position,
                    ),
                    first: moved(
                        view.first,
                        original.and_then(|v| v.first_sheet()),
                        &position,
                    ),
                }
            })
            .collect();
        let removed: BTreeSet<usize> = state.removed_names.clone();
        let insert_names = !added.is_empty();
        Self {
            sheets,
            names_changed: !removed.is_empty() || !rebuilt.is_empty() || !added.is_empty(),
            names_total: state.names.len(),
            removed,
            rebuilt,
            added,
            views,
            ref_written: state.ref_written,
            insert_calc: state.ref_written && !book.calc_pr(),
            insert_names: insert_names && !book.defined_names_element(),
        }
    }

    /// Whether the sheets differ from the opened ones.
    pub(in crate::structure::edit) fn sheets_changed(&self) -> bool {
        self.sheets.is_some()
    }

    /// Whether the part differs from the original. The views are not part
    /// of it: they change only when the sheets change (an add before the
    /// active sheet, a move or a delete), which `sheets` already covers.
    pub(in crate::structure::edit) fn changed(&self) -> bool {
        self.sheets_changed() || self.names_changed || self.ref_written
    }
}

/// The value of `activeTab` or `firstSheet` for a view that points at
/// `pointer`, `None` when it stays as it was: the position of the sheet
/// when that is not the original value (0 when the attribute is not there).
fn moved(
    pointer: Pointer,
    original: Option<u32>,
    position: &dyn Fn(usize) -> usize,
) -> Option<String> {
    let Pointer::Key(key) = pointer else {
        return None;
    };
    let at = position(key);
    let was = usize::try_from(original.unwrap_or(0)).unwrap_or(usize::MAX);
    (at != was).then(|| at.to_string())
}

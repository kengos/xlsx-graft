//! [`Plan::build`]: what the state says to write of the workbook part.

use super::names::NameOut;
use super::{Plan, SheetOut, ViewOut};
use crate::structure::edit::state::ViewEntry;
use crate::structure::edit::state::{Origin, Pointer, Scope, State, Text, escaped};
use crate::structure::model::{BookView, SheetState, Workbook};
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
        // which sheets it holds (by key), their order, their names or their
        // states: a sheet added and deleted, a move to the same place, a
        // rename to the same name and a state set back change nothing.
        let opened = (book.sheets().iter().enumerate())
            .map(|(key, sheet)| (key, sheet.name(), sheet.state()));
        let now = (state.sheets.iter()).map(|entry| (entry.key, entry.name.as_str(), entry.state));
        let titles = (state
            .sheets
            .iter()
            .map(|entry| (entry.key, entry.name.as_str())))
        .ne((book.sheets().iter().enumerate()).map(|(key, sheet)| (key, sheet.name())));
        let sheets = now.ne(opened).then(|| {
            (state.sheets.iter())
                .map(|entry| match &entry.origin {
                    Origin::Original { .. } => {
                        let was = book.sheets().get(entry.key).map(|sheet| sheet.state());
                        SheetOut::Original {
                            index: entry.key,
                            name: entry.renamed.then(|| escaped(&entry.name)),
                            state: (was != Some(entry.state)).then(|| attribute(entry.state)),
                        }
                    }
                    Origin::Added { index } => SheetOut::Added {
                        name: escaped(&entry.name),
                        sheet_id: entry.sheet_id,
                        rid: rids.get(index).cloned().unwrap_or_default(),
                        state: attribute(entry.state),
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
        let views: Vec<ViewOut> = (state.views.iter().zip(book.views()))
            .map(|(view, original)| view_out(view, Some(original), &position))
            .collect();
        // The view of a workbook that has none is written when it names a
        // sheet that is not the first.
        let insert_views = (book.views().is_empty())
            .then(|| {
                state
                    .views
                    .first()
                    .map(|view| view_out(view, None, &position))
            })
            .flatten()
            .filter(ViewOut::is_set);
        let removed: BTreeSet<usize> = state.removed_names.clone();
        let insert_names = !added.is_empty();
        Self {
            sheets,
            titles,
            names_changed: !removed.is_empty() || !rebuilt.is_empty() || !added.is_empty(),
            names_total: state.names.len(),
            removed,
            rebuilt,
            added,
            views_changed: views.iter().any(ViewOut::is_set),
            views,
            views_block: insert_views.is_some() && !book.book_views_element(),
            insert_views,
            ref_written: state.ref_written,
            insert_calc: state.ref_written && !book.calc_pr(),
            insert_names: insert_names && !book.defined_names_element(),
        }
    }

    /// Whether the `sheets` element differs from the opened one: in which
    /// sheets it holds (by key), their order, their names or their states.
    fn sheets_changed(&self) -> bool {
        self.sheets.is_some()
    }

    /// Whether the sheets the app part lists differ from the opened ones: the
    /// sheets it holds (by key), their order or their names. A sheet's state is
    /// not here: `TitlesOfParts` lists hidden sheets too, so the app part's
    /// lists stay right when only a state changes.
    pub(in crate::structure::edit) fn titles_changed(&self) -> bool {
        self.titles
    }

    /// Whether the part differs from the original.
    pub(in crate::structure::edit) fn changed(&self) -> bool {
        self.sheets_changed()
            || self.names_changed
            || self.views_changed
            || self.insert_views.is_some()
            || self.ref_written
    }
}

/// The `state` attribute of a sheet in `state`: `None` for a visible sheet,
/// which has none.
fn attribute(state: SheetState) -> Option<&'static str> {
    match state {
        SheetState::Visible => None,
        SheetState::Hidden => Some("hidden"),
        SheetState::VeryHidden => Some("veryHidden"),
    }
}

/// What to write of `view`, the view that was `original` when opened
/// (`None` for a view the edit adds, whose attributes are not written).
fn view_out(
    view: &ViewEntry,
    original: Option<&BookView>,
    position: &dyn Fn(usize) -> usize,
) -> ViewOut {
    ViewOut {
        active: moved(
            view.active,
            original.and_then(BookView::active_tab),
            position,
        ),
        first: moved(
            view.first,
            original.and_then(BookView::first_sheet),
            position,
        ),
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

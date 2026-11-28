//! The edits `layout` makes to the sheets of the workbook: which is active
//! and which are hidden.

use xlsx_graft_write::structure::StructureEdit;
use xlsx_graft_write::structure::model::SheetState;

/// Makes the last sheet of `names` (the sheets' names in order) the active
/// sheet when there are two or more, hides the second when there are three
/// or more and makes the third very hidden when there are four or more. A
/// call the edit refuses (the last sheet is hidden already, a sheet is the
/// last visible one) is not made, and the returned notes say which.
pub(super) fn change(edit: &mut StructureEdit<'_>, names: &[String]) -> Vec<String> {
    let mut notes = Vec::new();
    if let [.., last] = names
        && names.len() >= 2
        && let Err(error) = edit.set_active_sheet(last)
    {
        notes.push(format!("did not make {last:?} the active sheet: {error}"));
    }
    for (at, state, text) in [
        (1, SheetState::Hidden, "hide"),
        (2, SheetState::VeryHidden, "very hide"),
    ] {
        if names.len() > at + 1
            && let Err(error) = edit.set_sheet_state(&names[at], state)
        {
            notes.push(format!("did not {text} {:?}: {error}", names[at]));
        }
    }
    notes
}

//! The defined names of the workbook part that the report holds: a name
//! whose formula is malformed and holds the name of a sheet changed, a name
//! whose formula names a table removed, and a name that keeps a prefix
//! of a deleted sheet because it has no operand
//! (`SheetPrefix::operand` is `None`).

use super::super::formula::read as read_formula;
use super::super::snapshot::Snapshot;
use super::super::state::State;
use super::keeper::Keeper;
use super::text::{Context, defined_at};
use crate::structure::{KeptReason, KeptReference};

/// The defined names of the workbook part that are reported, up to
/// `remaining` of them (the number of references the report can still hold),
/// and how many more there are.
///
/// A name is judged as `State::rewrite_formulas` left it, so one whose
/// formula was rewritten no longer holds the name of a sheet changed.
pub(super) fn names(
    snapshot: &Snapshot,
    state: &State<'_>,
    context: &Context,
    remaining: usize,
) -> (Vec<KeptReference>, u64) {
    let mut keeper = Keeper::new(snapshot.workbook_name.as_str(), remaining);
    for entry in &state.names {
        let (malformed, tables) = context.inspect(&read_formula(&entry.text));
        if malformed.is_none() && tables.is_empty() && entry.unresolved.is_none() {
            continue;
        }
        let at = Some(defined_at(&entry.written));
        for table in &tables {
            let reason = KeptReason::DeletedTable;
            keeper.keep("definedName", at.as_ref(), None, Some(table), reason);
        }
        if let Some(sheet) = &malformed {
            let reason = KeptReason::Malformed;
            keeper.keep("definedName", at.as_ref(), Some(sheet), None, reason);
        }
        if let Some(sheet) = &entry.unresolved {
            let reason = KeptReason::UnknownOperand;
            keeper.keep("definedName", at.as_ref(), Some(sheet), None, reason);
        }
    }
    keeper.finish()
}

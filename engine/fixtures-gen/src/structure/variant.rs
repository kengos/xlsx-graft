//! The two edits `structure` makes to each workbook: what each does to its
//! sheets and defined names, and the file name suffix each is written under.

use xlsx_graft_write::structure::{StructureEdit, StructureError};

/// The name the first sheet is renamed to. It needs quotes in a formula (a
/// space) and is not ASCII.
pub(super) const RENAMED: &str = "Graft 名前 1";

/// The name of the sheet added.
pub(super) const ADDED: &str = "Graft added";

/// The workbook-scoped defined name the rename variant sets.
pub(super) const DEFINED_NAME: &str = "graft_name";

/// An edit of the sheets of a workbook.
#[derive(Clone, Copy)]
pub(super) enum Variant {
    /// The first sheet renamed to [`RENAMED`], [`ADDED`] added at position
    /// 0, the last sheet moved to position 1 and the defined name
    /// [`DEFINED_NAME`] set to `'Graft 名前 1'!$A$1`.
    Rename,
    /// [`ADDED`] added after the last sheet and the first sheet deleted.
    Delete,
}

impl Variant {
    /// Both variants, in the order they are written.
    pub(super) const ALL: [Self; 2] = [Self::Rename, Self::Delete];

    /// The text between the workbook's name and its extension.
    pub(super) fn suffix(self) -> &'static str {
        match self {
            Self::Rename => "rename",
            Self::Delete => "delete",
        }
    }

    /// Makes the edit on `edit`, a workbook whose sheets are named `names`
    /// in order. A workbook with no sheet gives `""` for the first name,
    /// which the edit refuses as a sheet it does not have.
    pub(super) fn apply(
        self,
        edit: &mut StructureEdit<'_>,
        names: &[String],
    ) -> Result<(), StructureError> {
        let first = names.first().map_or("", String::as_str);
        let last = names.last().map_or("", String::as_str);
        match self {
            Self::Rename => {
                edit.rename_sheet(first, RENAMED)?;
                edit.add_sheet(ADDED, Some(0), None)?;
                // The only sheet has its new name by now.
                let last = if last == first { RENAMED } else { last };
                edit.move_sheet(last, 1)?;
                let formula = format!("'{RENAMED}'!$A$1");
                edit.set_defined_name(DEFINED_NAME, None, &formula)
            }
            Self::Delete => {
                edit.add_sheet(ADDED, None, None)?;
                edit.delete_sheet(first)
            }
        }
    }
}

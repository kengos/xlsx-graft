//! [`XlsxGraftSheet`]: one entry of a book's list of sheets as it crosses
//! the C ABI, with its kind and whether it shows.

use xlsx_graft_read::{Sheet, SheetKind, SheetState};

/// A worksheet: the one kind with rows to read.
pub const XLSX_GRAFT_SHEET_WORKSHEET: u32 = 0;
/// A chartsheet, which holds no cells.
pub const XLSX_GRAFT_SHEET_CHARTSHEET: u32 = 1;
/// A dialog sheet, which holds no cells.
pub const XLSX_GRAFT_SHEET_DIALOGSHEET: u32 = 2;
/// A macro sheet, whose cells are not read.
pub const XLSX_GRAFT_SHEET_MACROSHEET: u32 = 3;
/// A sheet whose part is not known, for a reason the book's diagnostics
/// say; opening its rows is `XLSX_GRAFT_ERR_SHEET_UNREADABLE`.
pub const XLSX_GRAFT_SHEET_UNKNOWN: u32 = 4;

/// A sheet that shows.
pub const XLSX_GRAFT_STATE_VISIBLE: u32 = 0;
/// A hidden sheet, which a user can show again.
pub const XLSX_GRAFT_STATE_HIDDEN: u32 = 1;
/// A sheet hidden so that only a program can show it again.
pub const XLSX_GRAFT_STATE_VERY_HIDDEN: u32 = 2;

/// One sheet of a book, in the order the workbook lists them.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XlsxGraftSheet {
    /// The UTF-8 bytes of its name, not NUL-terminated.
    pub name: *const u8,
    /// How many bytes `name` points at.
    pub name_len: usize,
    /// One of the `XLSX_GRAFT_SHEET_*` kinds.
    pub kind: u32,
    /// One of the `XLSX_GRAFT_STATE_*` states.
    pub state: u32,
}

impl XlsxGraftSheet {
    /// `sheet`, its name lent from it.
    pub(crate) fn of(sheet: &Sheet) -> Self {
        let kind = match sheet.part().map(|part| part.kind()) {
            Some(SheetKind::Worksheet) => XLSX_GRAFT_SHEET_WORKSHEET,
            Some(SheetKind::Chartsheet) => XLSX_GRAFT_SHEET_CHARTSHEET,
            Some(SheetKind::Dialogsheet) => XLSX_GRAFT_SHEET_DIALOGSHEET,
            Some(SheetKind::Macrosheet) => XLSX_GRAFT_SHEET_MACROSHEET,
            None => XLSX_GRAFT_SHEET_UNKNOWN,
        };
        let state = match sheet.state() {
            SheetState::Visible => XLSX_GRAFT_STATE_VISIBLE,
            SheetState::Hidden => XLSX_GRAFT_STATE_HIDDEN,
            SheetState::VeryHidden => XLSX_GRAFT_STATE_VERY_HIDDEN,
        };
        Self {
            name: sheet.name().as_ptr(),
            name_len: sheet.name().len(),
            kind,
            state,
        }
    }
}

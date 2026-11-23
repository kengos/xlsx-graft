//! The types of what [`Workbook`](super::Workbook) reads of the workbook
//! part, and their getters: a [`Sheet`], a [`DefinedName`], a [`BookView`]
//! and a [`Table`]. Their fields are written by the reading in `walk.rs`,
//! `link.rs` and `tables.rs`.

/// One sheet of the workbook (`sheets/sheet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    pub(super) name: String,
    pub(super) sheet_id: u32,
    pub(super) state: SheetState,
    pub(super) relationship_id: String,
    pub(super) part: usize,
    pub(super) kind: SheetKind,
}

/// Whether a sheet shows (ST_SheetState).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetState {
    /// It shows, the default.
    Visible,
    /// It is hidden, and the user may show it.
    Hidden,
    /// It is hidden, and only a program may show it.
    VeryHidden,
}

/// What kind of sheet a part is, as its relationship's type says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetKind {
    /// A worksheet: cells (ECMA-376-1:2016 §12.3.24).
    Worksheet,
    /// A chartsheet: one chart (ECMA-376-1:2016 §12.3.2).
    Chartsheet,
    /// A dialogsheet: an old dialog box (ECMA-376-1:2016 §12.3.7).
    Dialogsheet,
    /// An Excel 4.0 macro sheet, international or not ([MS-OFFMACRO2]
    /// 2.2.1.1 and 2.2.1.2).
    Macrosheet,
}

/// One `definedName` as it is written: its attributes and its text, XML
/// references undone but `_xHHHH_` escapes not (`xlsx-graft-verify`'s
/// `book/defined.rs` compares defined names without undoing `_xHHHH_`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinedName {
    pub(super) attributes: Vec<(String, String)>,
    pub(super) name: String,
    pub(super) local_sheet_id: Option<u32>,
    pub(super) text: String,
}

/// One `workbookView` (ECMA-376-1:2016 §18.2.30).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookView {
    pub(super) active_tab: Option<u32>,
    pub(super) first_sheet: Option<u32>,
}

/// A table part, with the names it is written with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub(super) part: usize,
    pub(super) name: Option<String>,
    pub(super) display_name: Option<String>,
}

impl Sheet {
    /// Its name, its `_xHHHH_` escapes undone.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its `sheetId`.
    #[must_use]
    pub fn sheet_id(&self) -> u32 {
        self.sheet_id
    }

    /// Whether it shows.
    #[must_use]
    pub fn state(&self) -> SheetState {
        self.state
    }

    /// The `Id` of the workbook part's relationship it is tied to (its
    /// `r:id`).
    #[must_use]
    pub fn relationship_id(&self) -> &str {
        &self.relationship_id
    }

    /// The index of its part in
    /// [`Package::parts`](xlsx_graft_package::Package::parts).
    #[must_use]
    pub fn part(&self) -> usize {
        self.part
    }

    /// What kind of sheet it is.
    #[must_use]
    pub fn kind(&self) -> SheetKind {
        self.kind
    }
}

impl DefinedName {
    /// Every attribute, in the order written, with the name as written
    /// (`localSheetId`, or with a prefix), the namespace declarations left
    /// out, each value as the XML reader normalizes it.
    #[must_use]
    pub fn attributes(&self) -> &[(String, String)] {
        &self.attributes
    }

    /// Its `name`, as written.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its `localSheetId`: the sheet it is scoped to, `None` for the
    /// workbook's.
    #[must_use]
    pub fn local_sheet_id(&self) -> Option<u32> {
        self.local_sheet_id
    }

    /// Its text, the formula, as written.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl BookView {
    /// Its `activeTab`, `None` when not written.
    #[must_use]
    pub fn active_tab(&self) -> Option<u32> {
        self.active_tab
    }

    /// Its `firstSheet`, `None` when not written.
    #[must_use]
    pub fn first_sheet(&self) -> Option<u32> {
        self.first_sheet
    }
}

impl Table {
    /// The index of the table's part in
    /// [`Package::parts`](xlsx_graft_package::Package::parts).
    #[must_use]
    pub fn part(&self) -> usize {
        self.part
    }

    /// Its `name`, as written, `None` when not written.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Its `displayName`, as written, `None` when not written.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }
}

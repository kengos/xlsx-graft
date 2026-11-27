//! [`SheetProtection`]: the `sheetProtection` element
//! (ECMA-376-1:2016 §18.3.1.85).

use super::base64::encode;
use super::{PasswordHash, write_attributes};
use std::io::{self, Write};

/// What a protected sheet lets a user do, as Excel's Protect Sheet dialog
/// has it: a field is true when the operation is allowed. `Default` is the
/// dialog's: selecting cells allowed, everything else not.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one flag for each operation the dialog lists"
)]
pub struct SheetProtection {
    /// Select locked cells (default true).
    pub allow_select_locked_cells: bool,
    /// Select unlocked cells (default true).
    pub allow_select_unlocked_cells: bool,
    /// Format cells.
    pub allow_format_cells: bool,
    /// Format columns.
    pub allow_format_columns: bool,
    /// Format rows.
    pub allow_format_rows: bool,
    /// Insert columns.
    pub allow_insert_columns: bool,
    /// Insert rows.
    pub allow_insert_rows: bool,
    /// Insert hyperlinks.
    pub allow_insert_hyperlinks: bool,
    /// Delete columns.
    pub allow_delete_columns: bool,
    /// Delete rows.
    pub allow_delete_rows: bool,
    /// Sort.
    pub allow_sort: bool,
    /// Use AutoFilter.
    pub allow_auto_filter: bool,
    /// Use PivotTables.
    pub allow_pivot_tables: bool,
    /// Edit objects.
    pub allow_edit_objects: bool,
    /// Edit scenarios.
    pub allow_edit_scenarios: bool,
    /// The password to unprotect the sheet, none for no password.
    pub password: Option<PasswordHash>,
}

impl Default for SheetProtection {
    fn default() -> Self {
        Self {
            allow_select_locked_cells: true,
            allow_select_unlocked_cells: true,
            allow_format_cells: false,
            allow_format_columns: false,
            allow_format_rows: false,
            allow_insert_columns: false,
            allow_insert_rows: false,
            allow_insert_hyperlinks: false,
            allow_delete_columns: false,
            allow_delete_rows: false,
            allow_sort: false,
            allow_auto_filter: false,
            allow_pivot_tables: false,
            allow_edit_objects: false,
            allow_edit_scenarios: false,
            password: None,
        }
    }
}

impl SheetProtection {
    /// The attributes that say an operation is forbidden, in the order of
    /// the schema, each with the value the schema gives it when the
    /// attribute is absent (`true` for all but `objects`, `scenarios` and
    /// the two `select`s).
    fn forbidden(&self) -> [(&'static str, bool, bool); 15] {
        [
            ("objects", !self.allow_edit_objects, false),
            ("scenarios", !self.allow_edit_scenarios, false),
            ("formatCells", !self.allow_format_cells, true),
            ("formatColumns", !self.allow_format_columns, true),
            ("formatRows", !self.allow_format_rows, true),
            ("insertColumns", !self.allow_insert_columns, true),
            ("insertRows", !self.allow_insert_rows, true),
            ("insertHyperlinks", !self.allow_insert_hyperlinks, true),
            ("deleteColumns", !self.allow_delete_columns, true),
            ("deleteRows", !self.allow_delete_rows, true),
            ("selectLockedCells", !self.allow_select_locked_cells, false),
            ("sort", !self.allow_sort, true),
            ("autoFilter", !self.allow_auto_filter, true),
            ("pivotTables", !self.allow_pivot_tables, true),
            (
                "selectUnlockedCells",
                !self.allow_select_unlocked_cells,
                false,
            ),
        ]
    }

    /// Writes `<sheetProtection>`: the password as `algorithmName="SHA-512"`,
    /// `hashValue`, `saltValue` and `spinCount` when there is one (the
    /// legacy `password` attribute is never written), `sheet="1"`, and an
    /// attribute for each operation only when it differs from the default
    /// the schema gives an absent attribute. `prefix` goes before the
    /// element name.
    ///
    /// # Errors
    ///
    /// The error of `out`.
    pub fn write(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        write!(out, "<{prefix}sheetProtection")?;
        if let Some(password) = &self.password {
            write_attributes(
                out,
                &[
                    ("algorithmName", "SHA-512".to_owned()),
                    ("hashValue", encode(&password.hash())),
                    ("saltValue", encode(&password.salt())),
                    ("spinCount", password.spin_count().to_string()),
                ],
            )?;
        }
        out.write_all(br#" sheet="1""#)?;
        let differs = self
            .forbidden()
            .into_iter()
            .filter(|&(_, forbidden, default)| forbidden != default)
            .map(|(name, forbidden, _)| (name, if forbidden { "1" } else { "0" }.to_owned()))
            .collect::<Vec<_>>();
        write_attributes(out, &differs)?;
        out.write_all(b"/>")
    }
}

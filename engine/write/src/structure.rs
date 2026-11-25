//! The rules for the names a structure edit writes, and the form a sheet
//! is written with in a formula.
//!
//! The rules are the checks of `xlsx-graft-verify` and the reader, written
//! the other way, with stricter ones of the writer's own: a name this
//! module accepts is one that `Structure::check` of `xlsx-graft-verify`
//! reports no problem with, and one it refuses is named by the first rule
//! it breaks.
//!
//! - **A sheet name** (`names.rs`): ECMA-376-1:2016 §18.2.19 and
//!   MS-OI29500 on it (note a): not empty, at most 31 UTF-16 code units,
//!   none of `[ ] : * ? / \`, no `'` first or last, no C0 control
//!   character or character XML 1.0 cannot hold, not `History` (Excel's
//!   own screens refuse it), and no other sheet's name, letter case aside.
//! - **A defined name** (`names/defined.rs`): ECMA-376-1:2016 §18.17.2.5 and
//!   MS-OI29500 on it, §18.2.5: a name a formula can name, with no `?`, and
//!   not `\` alone or `\` and one character, at most 255
//!   UTF-16 code units, not of the form of a cell reference (`A1`, `R1C1`)
//!   and not `TRUE` or `FALSE`, no `_xlnm.` prefix except for seven of
//!   the built-in names, not `_xlnm.Database` (which a sheet holds, never
//!   the workbook), no `_xlfn.`,
//!   `_xlws.`, `_xludf.`, `_xleta.` or `_xlpm.` prefix, and no table's name,
//!   letter case aside.
//!   A defined name accepted here passes `Structure::check` when its scope
//!   is a sheet the workbook has and its formula names only sheets the
//!   workbook has.
//! - **The prefix of a sheet in a formula** (`prefix.rs`): the quotes
//!   ECMA-376-1:2016 §18.17.2.3 asks of a sheet name, and a 3-D
//!   reference's two names.
//! - **The workbook part, as a structure edit reads it** (`model.rs`):
//!   [`model::Workbook`] holds the sheets, the defined names, the views
//!   and the relationships of the workbook part, and the names of the
//!   tables. It is read the way `xlsx-graft-read` reads the sheets, and
//!   pushes the same diagnostics for a sheet it cannot tie to a part. It
//!   is not re-exported: call it as `structure::model::Workbook`.
//! - **What a structure edit left as it was** (`report.rs`):
//!   [`StructureReport`], the formulas it did not rewrite and the parts it
//!   could not read.
//! - **A structure edit** (`edit.rs`): [`StructureEdit`] adds, renames,
//!   deletes and moves sheets and sets and removes defined names on the
//!   model, rewriting the defined names' formulas and the views' sheet
//!   positions to match.

pub mod model;

mod edit;
mod error;
mod names;
mod prefix;
mod report;

pub use edit::StructureEdit;
pub use error::{DefinedNameFault, SheetNameFault, StructureError};
use names::builtin_spelling;
pub use names::{check_defined_name, check_sheet_name};
pub use prefix::{needs_quotes, sheet_prefix};
pub use report::{KeptReason, KeptReference, StructureReport};

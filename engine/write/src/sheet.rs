//! The parts of a worksheet that a new sheet and an edit of an existing one
//! write the same way: the order of the root's children, where the kinds a
//! caller writes go among them, and the values written in `cols`, `row`,
//! `sheetViews`, `mergeCells` and `sheetProtection`.
//!
//! - **The order** (`child.rs`, `placer.rs`): [`WorksheetChild`] lists the 39
//!   children of CT_Worksheet in the order of ECMA-376-1:2016 §18.3.1.99.
//!   A caller names the kinds it writes, replaces or edits, and
//!   [`ChildPlacer`] says, as the caller reads the root's children in
//!   document order, before which one each is written. A caller that edits
//!   a worksheet read from a workbook and a caller that writes a new
//!   worksheet use the same `ChildPlacer`.
//! - **The values** (`range.rs`, `columns.rs`, `rows.rs`, `panes.rs`,
//!   `merges.rs`, `protection.rs`, `password.rs`): a [`CellRange`],
//!   [`ColumnWidth`], [`Columns`], [`RowHeight`], [`FrozenPanes`],
//!   [`MergedCells`], [`SheetProtection`] and [`PasswordHash`] are made by
//!   `new` (or `sha512`), which refuses what Excel cannot hold with a
//!   [`SheetError`], so any value `new` returned can be written without
//!   another check.
//! - **The elements**: each type that is an element writes it with `write`
//!   (or `write_sheet_views`) as raw bytes to a `&mut dyn Write`, with the
//!   root's prefix and `:` (or `""`) before each element name; numbers are
//!   written as a cell's value is ([`xlsx_graft_cell::number_text`]), cell
//!   references as [`CellRef`](xlsx_graft_cell::CellRef) writes them. An
//!   element with nothing in it (no merged cells, no column widths) is not
//!   written. Tests that read the written XML back (verify, read) belong to
//!   the pull requests that call these writers.
//!
//! - **Editing a worksheet in a package** (`edit.rs`): a [`SheetEdit`] reads
//!   a worksheet part once, checks each call (merge and unmerge cells, set
//!   column widths and row heights, freeze panes, protect) against what it
//!   read and the calls before, and writes the changes as one
//!   [`PartRewriter`](crate::PartRewriter) given to
//!   [`PackageEdit::replace`](crate::PackageEdit::replace): it streams the
//!   part, writes the elements a call changed with the writers above, places
//!   the kinds the part lacks with the same [`ChildPlacer`], and writes every
//!   other byte as it was. Call
//!   [`StructureEdit::apply`](crate::structure::StructureEdit::apply) before
//!   [`SheetEdit::apply`].
//!
//! # Examples
//!
//! A [`PartWriter`](crate::PartWriter) for a new sheet writes each element at
//! the [`Due`] the [`ChildPlacer`] returns:
//!
//! ```
//! use std::io::Write;
//! use xlsx_graft_write::sheet::{
//!     CellRange, ChildPlacer, ColumnWidth, Columns, Due, FrozenPanes, MergedCells,
//!     PasswordHash, RootChild, RowHeight, SheetProtection, WorksheetChild,
//! };
//! use xlsx_graft_write::{PartWriter, WriteError};
//!
//! struct NewSheet {
//!     columns: Columns,
//!     panes: FrozenPanes,
//!     merges: MergedCells,
//!     protection: SheetProtection,
//!     row: RowHeight,
//! }
//!
//! impl NewSheet {
//!     fn write_due(&self, out: &mut dyn Write, due: Due) -> Result<(), WriteError> {
//!         match due.child {
//!             WorksheetChild::SheetViews => self.panes.write_sheet_views(out, "")?,
//!             WorksheetChild::Cols => self.columns.write(out, "")?,
//!             WorksheetChild::MergeCells => self.merges.write(out, "")?,
//!             WorksheetChild::SheetProtection => self.protection.write(out, "")?,
//!             _ => {}
//!         }
//!         Ok(())
//!     }
//! }
//!
//! const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
//!
//! impl PartWriter for NewSheet {
//!     fn write(&mut self, out: &mut dyn Write) -> Result<(), WriteError> {
//!         let kinds = [
//!             WorksheetChild::SheetViews,
//!             WorksheetChild::Cols,
//!             WorksheetChild::MergeCells,
//!             WorksheetChild::SheetProtection,
//!         ];
//!         let mut placer = ChildPlacer::new(kinds);
//!         out.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
//!         write!(out, r#"<worksheet xmlns="{MAIN}">"#)?;
//!         for due in placer.start(RootChild::Table(WorksheetChild::SheetData)).due {
//!             self.write_due(out, due)?;
//!         }
//!         write!(out, r#"<sheetData><row r="{}""#, self.row.row())?;
//!         for (name, value) in self.row.attributes() {
//!             write!(out, r#" {name}="{value}""#)?;
//!         }
//!         out.write_all(b"/></sheetData>")?;
//!         for due in placer.finish() {
//!             self.write_due(out, due)?;
//!         }
//!         out.write_all(b"</worksheet>")?;
//!         Ok(())
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut sheet = NewSheet {
//!     columns: Columns::new(vec![ColumnWidth::new(1, 2, 12.5)?])?,
//!     panes: FrozenPanes::new(1, 0)?,
//!     merges: MergedCells::new(vec![CellRange::parse("A1:B2")?])?,
//!     protection: SheetProtection {
//!         password: Some(PasswordHash::sha512("secret", [7; 16], 100_000)?),
//!         ..SheetProtection::default()
//!     },
//!     row: RowHeight::new(3, 30.0)?,
//! };
//! let mut bytes = Vec::new();
//! sheet.write(&mut bytes)?;
//! let xml = String::from_utf8(bytes)?;
//! let at = |name| xml.find(name).ok_or(name);
//! // The order of ECMA-376-1:2016 §18.3.1.99.
//! assert!(at("<sheetViews>")? < at("<cols>")?);
//! assert!(at("<cols>")? < at("<sheetData>")?);
//! assert!(at("</sheetData>")? < at("<sheetProtection")?);
//! assert!(at("<sheetProtection")? < at("<mergeCells")?);
//! # Ok(())
//! # }
//! ```

mod base64;
mod child;
mod columns;
mod edit;
mod error;
mod merges;
mod panes;
mod password;
mod placer;
mod protection;
mod range;
mod rows;

use std::io::{self, Write};
use xlsx_graft_cell::xml_attribute;

pub use child::WorksheetChild;
pub use columns::{ColumnWidth, Columns};
pub use edit::SheetEdit;
pub use error::SheetError;
pub use merges::MergedCells;
pub use panes::FrozenPanes;
pub use password::PasswordHash;
pub use placer::{ChildPlacer, Due, Placement, RootChild, Slot};
pub use protection::SheetProtection;
pub use range::CellRange;
pub use rows::RowHeight;

/// Writes the attributes, each as ` name="value"` with the value written as
/// an XML attribute.
fn write_attributes(out: &mut dyn Write, attributes: &[(&str, String)]) -> io::Result<()> {
    for (name, value) in attributes {
        write!(out, " {name}=\"")?;
        xml_attribute(value, |piece| out.write_all(piece.as_bytes()))?;
        out.write_all(b"\"")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;

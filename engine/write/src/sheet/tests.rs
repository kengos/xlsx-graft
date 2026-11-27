//! Tests for the `sheet` module, driven through its public items.
//! `child` covers the order of a worksheet's children, `placer` where the
//! kinds a caller writes go among them, `range` the cell range, `values`
//! what each value type refuses at its boundaries, `sweep` the search for
//! overlapping ranges, `password` and `base64` the password hash and the
//! text it is written as, and `writers` the bytes each element is written
//! as.

mod base64;
mod child;
mod password;
mod placer;
mod range;
mod sweep;
mod values;
mod writers;

use crate::sheet::CellRange;
use std::io;
use xlsx_graft_cell::CellRef;

/// The text `write` writes to a buffer in memory.
pub(super) fn written(write: impl FnOnce(&mut dyn io::Write) -> io::Result<()>) -> String {
    let mut out = Vec::new();
    write(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

/// The cell at an A1 reference.
pub(super) fn cell(text: &str) -> CellRef {
    CellRef::parse(text).unwrap()
}

/// The range of an A1 range.
pub(super) fn range(text: &str) -> CellRange {
    CellRange::parse(text).unwrap()
}

//! The shared strings workbook: a worksheet `Data` whose strings are all
//! different, so the shared strings table grows with the rows.

use rust_xlsxwriter::{Workbook, XlsxError};
use std::path::Path;

/// The header row, one title per column, in column order.
pub const SHARED_HEADER: [&str; 3] = ["number", "first", "second"];

/// Writes to `path` a workbook with one worksheet, `Data`, holding the
/// [`SHARED_HEADER`] row and then `rows` data rows. Data row `n` (the
/// worksheet's row `n + 1`), for `n` from 1, holds:
///
/// - `number`: `n`;
/// - `first`: the string `row <n>: the first of two strings in the row`,
///   with `<n>` as seven digits (48 bytes);
/// - `second`: the string `row <n>: the second of two strings in the row`,
///   with `<n>` as seven digits (49 bytes).
///
/// Every string is different from every other, so the shared strings table
/// (`xl/sharedStrings.xml`) holds `2 * rows` strings, none repeated. With
/// 1,000,000 rows the strings' UTF-8 text is about 97 MB (2,000,000 strings
/// of 48 and 49 bytes) and, with the reader's offset of each string, past the
/// reader's `sharedStringsSpillBytes` default of 32 MiB; with 100,000 rows the
/// text is about 9.7 MB, and with the offsets still under 32 MiB.
///
/// The worksheet is written in rust_xlsxwriter's low memory mode: each row
/// goes to a temporary file as it is written, so the memory used by the
/// worksheet does not grow with `rows`. Unlike constant memory mode, each
/// string is written to the shared strings table, not into its cell, and
/// that table stays in the workbook's memory: it grows with the number of
/// different strings, here `2 * rows`.
///
/// # Errors
///
/// [`XlsxError`]: `rows` is more than a worksheet holds below the header
/// row, or the workbook or its temporary files cannot be written.
pub fn write_shared_book(path: &Path, rows: u32) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet_with_low_memory()?;
    sheet.set_name("Data")?;
    for (column, title) in (0..).zip(SHARED_HEADER) {
        sheet.write_string(0, column, title)?;
    }
    for row in 1..=rows {
        sheet.write_number(row, 0, f64::from(row))?;
        let first = format!("row {row:07}: the first of two strings in the row");
        let second = format!("row {row:07}: the second of two strings in the row");
        sheet.write_string(row, 1, first)?;
        sheet.write_string(row, 2, second)?;
    }
    workbook.save(path)
}

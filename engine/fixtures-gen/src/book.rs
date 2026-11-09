//! One generated workbook: a worksheet `Data` with a header row and the
//! data rows, written a row at a time.

use rust_xlsxwriter::{Format, Workbook, XlsxError};
use std::path::Path;

/// The header row, one title per column, in column order.
pub const HEADER: [&str; 5] = ["number", "fraction", "text", "flag", "date"];

/// 2024-01-01 as a serial date number in the 1900 date system, which
/// rust_xlsxwriter's workbooks use.
const FIRST_DATE: f64 = 45_292.0;

/// Writes to `path` a workbook with one worksheet, `Data`, holding the
/// [`HEADER`] row and then `rows` data rows. Data row `n` (the worksheet's
/// row `n + 1`), for `n` from 1, holds:
///
/// - `number`: `n`;
/// - `fraction`: `n / 4`;
/// - `text`: the string `text <n>`;
/// - `flag`: whether `n` is even;
/// - `date`: 2024-01-01 plus `(n - 1) % 366` days, under the number format
///   `yyyy-mm-dd`, in a column wide enough to show it.
///
/// The worksheet is written in rust_xlsxwriter's `constant_memory` mode:
/// each row goes to a temporary file as it is written, so the memory used
/// does not grow with `rows`, and each string is written in its cell
/// (`t="inlineStr"`) rather than in a shared strings table.
///
/// # Errors
///
/// [`XlsxError`]: `rows` is more than a worksheet holds below the header
/// row, or the workbook or its temporary files cannot be written.
pub fn write_book(path: &Path, rows: u32) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let date = Format::new().set_num_format("yyyy-mm-dd");
    let sheet = workbook.add_worksheet_with_constant_memory()?;
    sheet.set_name("Data")?;
    // Wide enough for Excel to show `yyyy-mm-dd` rather than `########`.
    sheet.set_column_width(4, 11)?;
    for (column, title) in (0..).zip(HEADER) {
        sheet.write_string(0, column, title)?;
    }
    for row in 1..=rows {
        let n = f64::from(row);
        sheet.write_number(row, 0, n)?;
        sheet.write_number(row, 1, n / 4.0)?;
        sheet.write_string(row, 2, format!("text {row}"))?;
        sheet.write_boolean(row, 3, row % 2 == 0)?;
        sheet.write_number_with_format(row, 4, FIRST_DATE + f64::from((row - 1) % 366), &date)?;
    }
    workbook.save(path)
}

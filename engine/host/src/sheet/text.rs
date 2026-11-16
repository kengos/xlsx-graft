//! [`GridText`]: the grid as text, for a person. Not a contract: it may
//! change.

use super::cell::GridValue;
use super::{Grid, GridSheet, Span};
use crate::report::Counts;
use crate::text::DiagnosticsText;
use crate::view::View;
use std::fmt::{self, Write as _};
use xlsx_graft_read::CellRef;

/// The view of a grid as text: a line saying which sheet it is and its used
/// range, its `dimension` and the range shown; a Markdown table of the cells
/// shown, with the column letters above and the row numbers on the left; the
/// merged ranges and what was left out; the diagnostics as
/// [`DiagnosticsText`] writes them; and a last line of the number of cells
/// listed, the counts of the diagnostics listed and whether the view is
/// `ok`.
///
/// ```text
/// Sheet1 (index 0, worksheet, visible) — used A1:F120, dimension A1:F120, shown A1:F100
/// |  | A | B | C |
/// | --- | --- | --- | --- |
/// | 1 | Name | Qty\|Unit | 2024-01-15 |
/// | 2 | line\nbreak |  | 3 |
/// Merged: A1:C1, B5:B9
/// … 20 more rows with values below row 100
/// … more columns with values right of column C
///
/// warn
///   ...
/// 5 cells, 0 errors, 1 warning, 0 notices — ok
/// ```
///
/// Every row and column of the range shown is a row and a column of the
/// table, a cell with no value being empty. A cell is written as `read
/// --format text` writes it, but for the `|` of a string or an error value,
/// which is written `\|` as well so that it stays in its column; a string
/// cut at 1,000 characters ends in `…`. `Merged:` is there only when a
/// merged range overlaps the range shown, and each line of what was left
/// out only when it applies. When the range shown is none, the table is
/// replaced by `nothing to show`. When the range shown has more than
/// [`MAX_DRAWN_CELLS`](Self::MAX_DRAWN_CELLS) cells, the table is replaced by
/// `table of <rows> rows and <columns> columns is too large to draw; the
/// cells with values follow` and a line `B2: <value>` for each cell with a
/// value, from the top, written as a cell of the table is. When the view has
/// an `error`, the first line is replaced by `could not read: <message>`, and
/// what was read before it follows.
#[derive(Debug, Clone, Copy)]
pub struct GridText<'a>(pub &'a View<Grid>);

impl GridText<'_> {
    /// The most cells of the range shown (its rows times its columns, the
    /// column letters and row numbers not counted) that the table is drawn
    /// for, empty cells included. Past it, the table is replaced by a line
    /// saying so and the cells with a value, one `B2: value` a line.
    pub const MAX_DRAWN_CELLS: u64 = 100_000;
}

impl fmt::Display for GridText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let view = self.0;
        let grid = &view.content;
        match &view.error {
            Some(error) => writeln!(f, "could not read: {}", error.message)?,
            None => (grid.sheet.iter()).try_for_each(|sheet| heading(f, sheet, grid))?,
        }
        if grid.sheet.is_some() {
            body(f, grid, view.error.is_none())?;
        }
        if !view.diagnostics.is_empty() {
            writeln!(f)?;
            write!(f, "{}", DiagnosticsText(&view.diagnostics))?;
        }
        let cells =
            (grid.rows.iter()).fold(0_usize, |sum, row| sum.saturating_add(row.cells.len()));
        let noun = if cells == 1 { "cell" } else { "cells" };
        let verdict = if view.ok { "ok" } else { "not ok" };
        let counts = Counts::tally(&view.diagnostics);
        writeln!(f, "{cells} {noun}, {counts} — {verdict}")
    }
}

/// The first line: the sheet, and the three ranges.
fn heading(f: &mut fmt::Formatter<'_>, sheet: &GridSheet, grid: &Grid) -> fmt::Result {
    let range = |span: Option<Span>| span.map_or_else(|| "none".to_owned(), |s| s.0.to_string());
    let dimension = (grid.dimension.as_ref()).map_or("none", |echo| echo.as_str());
    writeln!(
        f,
        "{} (index {}, {}, {}) — used {}, dimension {dimension}, shown {}",
        sheet.name,
        sheet.index,
        sheet.kind,
        sheet.state,
        range(grid.used),
        range(grid.shown),
    )
}

/// The table, the merged ranges and what was left out; `nothing to show`
/// when there is no table and the sheet was read without a failure.
fn body(f: &mut fmt::Formatter<'_>, grid: &Grid, read: bool) -> fmt::Result {
    let Some(Span(shown)) = grid.shown else {
        return if read {
            writeln!(f, "nothing to show")
        } else {
            Ok(())
        };
    };
    let (first, last) = (shown.first(), shown.last());
    let rows = u64::from(last.row()).saturating_sub(u64::from(first.row()));
    let columns = u64::from(last.column()).saturating_sub(u64::from(first.column()));
    let (rows, columns) = (rows.saturating_add(1), columns.saturating_add(1));
    if rows.saturating_mul(columns) > GridText::MAX_DRAWN_CELLS {
        list(f, grid, (rows, columns))?;
    } else {
        table(f, grid, first, last)?;
    }
    if !grid.merges.is_empty() {
        f.write_str("Merged: ")?;
        for (position, span) in grid.merges.iter().enumerate() {
            let separator = if position == 0 { "" } else { ", " };
            write!(f, "{separator}{}", span.0)?;
        }
        writeln!(f)?;
    }
    if grid.truncated.merges {
        let line = if grid.merges_over_max {
            "… more merged ranges overlap the range shown"
        } else {
            "… the sheet has more merged ranges than were read; some may overlap the range shown"
        };
        writeln!(f, "{line}")?;
    }
    if grid.truncated.rows {
        let (below, last) = (grid.rows_below, shown.last().row());
        let noun = if below == 1 { "row" } else { "rows" };
        writeln!(f, "… {below} more {noun} with values below row {last}")?;
    }
    if grid.truncated.columns {
        let letters = column_letters(shown.last().column());
        writeln!(f, "… more columns with values right of column {letters}")?;
    }
    Ok(())
}

/// The cells with a value, a line each (`B2: value`), under a line saying
/// that the table of `rows` and `columns` is too large to draw.
fn list(f: &mut fmt::Formatter<'_>, grid: &Grid, (rows, columns): (u64, u64)) -> fmt::Result {
    write!(f, "table of {rows} rows and {columns} columns")?;
    f.write_str(" is too large to draw; the cells with values follow\n")?;
    for cell in grid.rows.iter().flat_map(|row| row.cells.iter()) {
        write!(f, "{}: ", cell.reference)?;
        write_value(f, &cell.value)?;
        writeln!(f)?;
    }
    Ok(())
}

/// The table of the cells from `first` to `last`, a line each for the
/// column letters, the rule under them and each row.
fn table(f: &mut fmt::Formatter<'_>, grid: &Grid, first: CellRef, last: CellRef) -> fmt::Result {
    let columns = first.column()..=last.column();
    f.write_str("|  |")?;
    for column in columns.clone() {
        write!(f, " {} |", column_letters(column))?;
    }
    f.write_str("\n| --- |")?;
    for _ in columns.clone() {
        f.write_str(" --- |")?;
    }
    writeln!(f)?;
    let mut present = grid.rows.iter().peekable();
    for number in first.row()..=last.row() {
        let mut cells = (present.next_if(|row| row.row == number))
            .map(|row| row.cells.iter())
            .unwrap_or_default()
            .peekable();
        write!(f, "| {number} |")?;
        for column in columns.clone() {
            match cells.next_if(|cell| cell.reference.column() == column) {
                Some(cell) => {
                    f.write_char(' ')?;
                    write_value(f, &cell.value)?;
                    f.write_str(" |")?;
                }
                None => f.write_str("  |")?,
            }
        }
        writeln!(f)?;
    }
    Ok(())
}

/// The letters of the column `column`: `A`, `Z`, `AA`, `XFD`.
fn column_letters(column: u16) -> String {
    let cell = CellRef::new(column, 1).map(|cell| cell.to_string());
    let cell = cell.unwrap_or_default();
    cell.trim_end_matches(|c: char| c.is_ascii_digit())
        .to_owned()
}

/// Writes `value` as `read --format text` does, but for the `|`: a number
/// as Rust writes it, in an exponent's form below 1e-5 and from 1e16; a
/// boolean `TRUE` or `FALSE`; a date or time in ISO 8601; elapsed time in
/// hours that do not wrap at a day (`26:03:00.000`); a string or an error
/// value escaped, and ending in `…` when it was cut.
fn write_value(f: &mut fmt::Formatter<'_>, value: &GridValue) -> fmt::Result {
    match value {
        GridValue::String(text) | GridValue::Error(text) => {
            for char in text.as_str().chars() {
                match char {
                    '\\' => f.write_str("\\\\")?,
                    '\t' => f.write_str("\\t")?,
                    '\n' => f.write_str("\\n")?,
                    '\r' => f.write_str("\\r")?,
                    '|' => f.write_str("\\|")?,
                    char => f.write_char(char)?,
                }
            }
            if text.truncated() {
                f.write_char('…')?;
            }
            Ok(())
        }
        GridValue::Number(number) if *number == 0.0 || (1e-5..1e16).contains(&number.abs()) => {
            write!(f, "{number}")
        }
        GridValue::Number(number) => write!(f, "{number:e}"),
        GridValue::Bool(bool) => f.write_str(if *bool { "TRUE" } else { "FALSE" }),
        GridValue::Date(date) => write!(f, "{date}"),
        GridValue::Time(time) => write!(f, "{time}"),
        GridValue::DateTime(at) => write!(f, "{at}"),
        GridValue::Elapsed(elapsed) => {
            let ms = elapsed.milliseconds();
            let (hours, minutes) = (ms / 3_600_000, ms / 60_000 % 60);
            let (seconds, ms) = (ms / 1_000 % 60, ms % 1_000);
            write!(f, "{hours}:{minutes:02}:{seconds:02}.{ms:03}")
        }
    }
}

#[cfg(test)]
mod tests;

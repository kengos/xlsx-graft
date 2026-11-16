//! Reading a sheet's rows one at a time into the cells the grid lists: the
//! cells with a value inside the window, which part of the sheet has values
//! beyond it, and the sheet's used range. What is held is the cells listed
//! (at most [`SheetRequest::MAX_CELLS`]) and a few numbers, never the rows
//! of the sheet.

use super::cell::{GridCell, GridValue};
use super::request::SheetRequest;
use super::{GridRow, Truncated};
use xlsx_graft_read::{CellRange, CellRef, Row, Value};

/// A rectangle of cells, by column and row numbers, from 1.
#[derive(Debug, Clone, Copy)]
struct Frame {
    first_column: u16,
    first_row: u32,
    last_column: u16,
    last_row: u32,
}

impl Frame {
    /// Whether the cell at `column` and `row` is inside.
    fn contains(&self, column: u16, row: u32) -> bool {
        (self.first_column..=self.last_column).contains(&column)
            && (self.first_row..=self.last_row).contains(&row)
    }
}

/// What the rows read came to.
pub(super) struct Collected {
    /// The rows with cells inside the window, from the top.
    pub(super) rows: Vec<GridRow>,
    /// Whether cells with values are below or to the right of the window.
    pub(super) truncated: Truncated,
    /// How many rows have a cell with a value inside the range, below the
    /// window.
    pub(super) rows_below: u64,
    /// The first to the last row and column of the cells with a value, in
    /// the whole sheet.
    pub(super) used: Option<CellRange>,
    /// The window, to the last row and column of the used range.
    pub(super) shown: Option<CellRange>,
}

/// Takes the rows of a sheet in order and keeps what the request asks to
/// show.
pub(super) struct Collector {
    /// The cells the request is about: its range, or the whole sheet.
    area: Frame,
    /// The part of the area that is listed: from its top left corner, as
    /// many rows and columns as the request allows, and no more cells than
    /// [`SheetRequest::MAX_CELLS`] cells or [`SheetRequest::MAX_TEXT_CHARS`]
    /// characters. Its last row moves up when a row would pass either.
    window: Frame,
    /// The corners of the cells with a value, so far.
    used: Option<Frame>,
    /// How many cells are listed.
    cells: usize,
    /// How many characters the strings and error values listed have in all.
    chars: usize,
    rows: Vec<GridRow>,
    truncated: Truncated,
    rows_below: u64,
}

impl Collector {
    /// A collector for `request`.
    pub(super) fn new(request: &SheetRequest) -> Self {
        let area = match request.range {
            Some(range) => Frame {
                first_column: range.first().column(),
                first_row: range.first().row(),
                last_column: range.last().column(),
                last_row: range.last().row(),
            },
            None => Frame {
                first_column: 1,
                first_row: 1,
                last_column: CellRef::MAX_COLUMN,
                last_row: CellRef::MAX_ROW,
            },
        };
        let columns = request.max_columns.saturating_sub(1);
        let rows = request.max_rows.saturating_sub(1);
        let window = Frame {
            last_column: area
                .first_column
                .saturating_add(columns)
                .min(area.last_column),
            last_row: area.first_row.saturating_add(rows).min(area.last_row),
            ..area
        };
        Self {
            area,
            window,
            used: None,
            cells: 0,
            chars: 0,
            rows: Vec::new(),
            truncated: Truncated::default(),
            rows_below: 0,
        }
    }

    /// Takes the next row of the sheet.
    pub(super) fn row(&mut self, row: &Row) {
        let number = row.number();
        let mut listed: Vec<GridCell> = (filled(row))
            .filter(|(column, _)| self.window.contains(*column, number))
            .filter_map(|(column, value)| cell(column, number, value))
            .collect();
        let mut chars = (listed.iter()).fold(0_usize, |sum, cell| {
            sum.saturating_add(cell.value.text_chars())
        });
        if self.cells.saturating_add(listed.len()) > SheetRequest::MAX_CELLS
            || self.chars.saturating_add(chars) > SheetRequest::MAX_TEXT_CHARS
        {
            // A row is listed whole or not at all: the window's last row
            // becomes the row before this one, so this row and the rows
            // after it are not listed.
            self.window.last_row = number.saturating_sub(1);
            listed.clear();
            chars = 0;
        }
        let mut below = false;
        for (column, _) in filled(row) {
            self.use_cell(column, number);
            if !self.area.contains(column, number) {
                continue;
            }
            let past_bottom = number > self.window.last_row;
            self.truncated.columns |= column > self.window.last_column;
            self.truncated.rows |= past_bottom;
            below |= past_bottom;
        }
        self.cells = self.cells.saturating_add(listed.len());
        self.chars = self.chars.saturating_add(chars);
        if !listed.is_empty() {
            self.rows.push(GridRow {
                row: number,
                cells: listed,
            });
        }
        self.rows_below = self.rows_below.saturating_add(u64::from(below));
    }

    /// Widens the used range to the cell at `column` and `row`.
    fn use_cell(&mut self, column: u16, row: u32) {
        let used = self.used.get_or_insert(Frame {
            first_column: column,
            first_row: row,
            last_column: column,
            last_row: row,
        });
        used.first_column = used.first_column.min(column);
        used.first_row = used.first_row.min(row);
        used.last_column = used.last_column.max(column);
        used.last_row = used.last_row.max(row);
    }

    /// What the rows came to.
    pub(super) fn finish(self) -> Collected {
        let used = self.used.and_then(range);
        let shown = used.and_then(|used| {
            let corner = CellRange::new(CellRef::new(1, 1)?, used.last());
            range(self.window)?.intersect(corner)
        });
        Collected {
            rows: self.rows,
            truncated: self.truncated,
            rows_below: self.rows_below,
            used,
            shown,
        }
    }
}

/// The cells of `row` that hold a value, with their column numbers.
fn filled(row: &Row) -> impl Iterator<Item = (u16, Value<'_>)> {
    (1..=CellRef::MAX_COLUMN)
        .zip(row.values())
        .filter(|(_, value)| !matches!(value, Value::Empty))
}

/// The cell at `column` and `row` holding `value`; `None` when it holds none.
fn cell(column: u16, row: u32, value: Value<'_>) -> Option<GridCell> {
    Some(GridCell {
        reference: CellRef::new(column, row)?,
        value: GridValue::of(value)?,
    })
}

/// The cells of `frame`; `None` when it holds none (its last row above its
/// first).
fn range(frame: Frame) -> Option<CellRange> {
    (frame.first_row <= frame.last_row).then_some(())?;
    let first = CellRef::new(frame.first_column, frame.first_row)?;
    let last = CellRef::new(frame.last_column, frame.last_row)?;
    Some(CellRange::new(first, last))
}

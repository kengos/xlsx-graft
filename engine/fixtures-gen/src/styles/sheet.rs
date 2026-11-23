//! [`SheetCells`]: the rewriter of a worksheet that gives A1, A2, ... a cell
//! format each.

mod attributes;
mod row;

use attributes::row_number;
use std::io::{self, BufRead, Write};
use xlsx_graft_cell::{CellRef, CellValue, DateSystem, write_cell};
use xlsx_graft_write::{PartRewriter, WriteError};
use xlsx_graft_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use xlsx_graft_xml::{XmlLimits, XmlRewriter};

/// A cell of column A to write: its row, its cell format and its value.
pub(super) struct Cell {
    row: u32,
    style: u32,
    value: CellValue<'static>,
}

/// Writes a worksheet again with the cell format `style` on cell A`n` for
/// each `(style, value)` given, `n` counting from 1, in the order given.
///
/// - A cell A`n` the sheet has keeps its value or formula and has its `s`
///   replaced (or added).
/// - A row `n` that has no A cell gets one written by
///   [`write_cell`] with the value, before the row's first cell, and has its
///   `spans` attribute removed, an optional attribute that the new cell may
///   lie outside.
/// - A row `n` the sheet does not have is added with that cell, before the
///   next row, or before `</sheetData>` when no row follows.
/// - `<sheetData/>` and `<row r="n"/>` are written as a start tag and an
///   end tag with the new cell between them.
///
/// Rows after the last of the cells given and everything outside
/// `sheetData` are written as they were. Every `<row>` in `sheetData` must
/// have an `r` that parses as a row number, and in each row given a cell, the
/// first `<c>` must have an `r` that parses as a cell reference; a sheet that
/// does not is refused with [`WriteError::Io`], as is a `sheetData` whose
/// name has a namespace prefix, since the rows and cells added have none. The
/// `dimension` is not updated, so a cell added outside the range it gives
/// stays outside that range. Rows are taken to be in ascending order of `r`,
/// as ECMA-376-1:2016 §18.3.1.73 requires, and a sheet whose rows are not may
/// be written with two `<row>` elements of the same `r`.
pub(super) struct SheetCells {
    cells: Vec<Cell>,
}

impl SheetCells {
    /// A rewriter of the cells `cells`, in the rows 1, 2, ... .
    pub(super) fn new(cells: Vec<(u32, CellValue<'static>)>) -> Self {
        let numbered = cells.into_iter().zip(1..);
        let cells = numbered.map(|((style, value), row)| Cell { row, style, value });
        Self {
            cells: cells.collect(),
        }
    }
}

impl PartRewriter for SheetCells {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        let mut walk = Walk {
            rewriter: XmlRewriter::new(original, out, &XmlLimits::default()),
            cells: &self.cells,
            done: 0,
        };
        let mut mode = Mode::Before;
        let mut buf = Vec::new();
        while let Some(event) = walk.rewriter.next(&mut buf)? {
            mode = walk.step(mode, event)?;
        }
        Ok(())
    }
}

/// Where the walk is in the sheet.
enum Mode {
    /// Before `sheetData`.
    Before,
    /// Directly in `sheetData`.
    Data,
    /// In a row, `depth` elements deep inside its children; `pending` is
    /// the row's start while it is not known whether the row has an A cell.
    Row {
        depth: u32,
        pending: Option<Pending>,
    },
    /// After `sheetData`.
    After,
}

/// A row to which a cell is to be written, not yet decided: its start tag,
/// the events after it that are not elements, and the cell.
struct Pending {
    start: BytesStart<'static>,
    held: Vec<Event<'static>>,
    style: u32,
    cell: String,
}

/// The rewriter and the cells not yet written: the `done` first of `cells`
/// are.
struct Walk<'a, R, W> {
    rewriter: XmlRewriter<R, W>,
    cells: &'a [Cell],
    done: usize,
}

/// The error of a sheet that is refused, for `detail`.
fn refuse(detail: impl std::fmt::Display) -> WriteError {
    WriteError::Io(io::Error::other(detail.to_string()))
}

/// Whether the element `start` is named `name`, a prefix aside.
fn named(start: &BytesStart<'_>, name: &str) -> bool {
    start.local_name().as_ref() == name
}

impl<R: BufRead, W: Write> Walk<'_, R, W> {
    fn write(&mut self, event: Event<'_>) -> Result<(), WriteError> {
        Ok(self.rewriter.write(event)?)
    }

    /// Writes `xml`, which holds tags and is written as it is.
    fn raw(&mut self, xml: &str) -> Result<(), WriteError> {
        self.write(Event::Text(BytesText::from_escaped(xml)))
    }

    /// The next step: `event` in `mode`, and the mode after it.
    fn step(&mut self, mode: Mode, event: Event<'_>) -> Result<Mode, WriteError> {
        match mode {
            Mode::Before => self.before(event),
            Mode::Data => self.data(event),
            Mode::Row { depth, pending } => self.row(depth, pending, event),
            Mode::After => {
                self.write(event)?;
                Ok(Mode::After)
            }
        }
    }

    fn before(&mut self, event: Event<'_>) -> Result<Mode, WriteError> {
        match event {
            Event::Start(start) if named(&start, "sheetData") => {
                prefixless(&start)?;
                self.write(Event::Start(start))?;
                Ok(Mode::Data)
            }
            Event::Empty(start) if named(&start, "sheetData") => {
                prefixless(&start)?;
                let end = BytesEnd::new(start.name().as_ref().to_owned());
                self.write(Event::Start(start.borrow()))?;
                self.rows_before(u32::MAX)?;
                self.write(Event::End(end))?;
                Ok(Mode::After)
            }
            event => {
                self.write(event)?;
                Ok(Mode::Before)
            }
        }
    }

    fn data(&mut self, event: Event<'_>) -> Result<Mode, WriteError> {
        match event {
            Event::Start(start) if named(&start, "row") => {
                let number = row_number(&start)?;
                self.rows_before(number)?;
                self.row_start(start, number, false)
            }
            Event::Empty(start) if named(&start, "row") => {
                let number = row_number(&start)?;
                self.rows_before(number)?;
                self.row_start(start, number, true)
            }
            Event::End(end) if end.local_name().as_ref() == "sheetData" => {
                self.rows_before(u32::MAX)?;
                self.write(Event::End(end))?;
                Ok(Mode::After)
            }
            event => {
                self.write(event)?;
                Ok(Mode::Data)
            }
        }
    }
}

/// Refuses an element whose name has a namespace prefix.
fn prefixless(start: &BytesStart<'_>) -> Result<(), WriteError> {
    match start.name().prefix() {
        Some(_) => Err(refuse("sheetData has a namespace prefix")),
        None => Ok(()),
    }
}

/// The text of the cell `cell` writes: `<c>` with its reference, `s` and
/// value.
fn cell_xml(cell: &Cell) -> Result<String, WriteError> {
    let at = CellRef::parse(&format!("A{}", cell.row)).map_err(refuse)?;
    let mut xml = Vec::new();
    let system = DateSystem::Compatibility1900;
    // Whether a string was cut is not asked: the catalog's are short.
    let _whole = write_cell(&mut xml, at, Some(cell.style), &cell.value, system).map_err(refuse)?;
    Ok(String::from_utf8_lossy(&xml).into_owned())
}

#[cfg(test)]
mod tests;

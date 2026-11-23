//! `SheetCells` in a row: the rows added before it, and the cell of column A
//! it is given.

use super::attributes::{column, rebuilt};
use super::{Cell, Mode, Pending, Walk, cell_xml, named};
use std::io::{BufRead, Write};
use xlsx_graft_write::WriteError;
use xlsx_graft_xml::events::{BytesEnd, BytesStart, Event};

impl<R: BufRead, W: Write> Walk<'_, R, W> {
    /// Writes each row of the cells not yet written that is before the row
    /// `before`, as a row that holds its cell alone.
    pub(super) fn rows_before(&mut self, before: u32) -> Result<(), WriteError> {
        while let Some(cell) = self.cells.get(self.done).filter(|cell| cell.row < before) {
            let xml = format!(r#"<row r="{}">{}</row>"#, cell.row, cell_xml(cell)?);
            self.raw(&xml)?;
            self.done += 1;
        }
        Ok(())
    }

    /// The start of the row `number`, which `empty` says is `<row/>`: the
    /// mode after it. The cell is written to the row now when it is empty,
    /// and in the row once its first cell is read otherwise.
    pub(super) fn row_start(
        &mut self,
        start: BytesStart<'_>,
        number: u32,
        empty: bool,
    ) -> Result<Mode, WriteError> {
        let cell: Option<&Cell> = self.cells.get(self.done).filter(|c| c.row == number);
        let Some(cell) = cell else {
            return if empty {
                self.write(Event::Empty(start))?;
                Ok(Mode::Data)
            } else {
                self.write(Event::Start(start))?;
                Ok(Mode::Row {
                    depth: 0,
                    pending: None,
                })
            };
        };
        self.done += 1;
        let xml = cell_xml(cell)?;
        if empty {
            let end = BytesEnd::new(start.name().as_ref().to_owned());
            self.write(Event::Start(rebuilt(&start, "spans", None)?))?;
            self.raw(&xml)?;
            self.write(Event::End(end))?;
            return Ok(Mode::Data);
        }
        let pending = Pending {
            start: start.into_owned(),
            held: Vec::new(),
            style: cell.style,
            cell: xml,
        };
        Ok(Mode::Row {
            depth: 0,
            pending: Some(pending),
        })
    }

    /// `event` in a row.
    pub(super) fn row(
        &mut self,
        depth: u32,
        pending: Option<Pending>,
        event: Event<'_>,
    ) -> Result<Mode, WriteError> {
        match pending {
            Some(pending) => self.resolve(pending, event),
            None => self.pass(depth, event),
        }
    }

    /// `event` in a row whose cell is written, `depth` elements deep.
    fn pass(&mut self, depth: u32, event: Event<'_>) -> Result<Mode, WriteError> {
        let depth = match &event {
            Event::Start(_) => depth + 1,
            Event::End(_) if depth == 0 => {
                self.write(event)?;
                return Ok(Mode::Data);
            }
            Event::End(_) => depth - 1,
            _ => depth,
        };
        self.write(event)?;
        Ok(Mode::Row {
            depth,
            pending: None,
        })
    }

    /// `event`, the next in a row that is to be given a cell and whose start
    /// is held: the first element or the end of the row decides whether the
    /// row has an A cell. Whatever comes before it (white space, a comment)
    /// is held with the start.
    fn resolve(&mut self, mut pending: Pending, event: Event<'_>) -> Result<Mode, WriteError> {
        match event {
            Event::Start(cell) if a_cell(&cell)? => {
                self.release(&mut pending, false)?;
                let style = Some(pending.style);
                self.write(Event::Start(rebuilt(&cell, "s", style)?))?;
                Ok(Mode::Row {
                    depth: 1,
                    pending: None,
                })
            }
            Event::Empty(cell) if a_cell(&cell)? => {
                self.release(&mut pending, false)?;
                let style = Some(pending.style);
                self.write(Event::Empty(rebuilt(&cell, "s", style)?))?;
                Ok(Mode::Row {
                    depth: 0,
                    pending: None,
                })
            }
            event @ (Event::Start(_) | Event::Empty(_) | Event::End(_)) => {
                self.release(&mut pending, true)?;
                self.pass(0, event)
            }
            event => {
                pending.held.push(event.into_owned());
                Ok(Mode::Row {
                    depth: 0,
                    pending: Some(pending),
                })
            }
        }
    }

    /// Writes the held row start and what follows it; with `insert`, the
    /// start with its `spans` attribute removed and then the new cell.
    fn release(&mut self, pending: &mut Pending, insert: bool) -> Result<(), WriteError> {
        let start = if insert {
            rebuilt(&pending.start, "spans", None)?
        } else {
            pending.start.clone()
        };
        self.write(Event::Start(start))?;
        for event in std::mem::take(&mut pending.held) {
            self.write(event)?;
        }
        if insert {
            self.raw(&pending.cell)?;
        }
        Ok(())
    }
}

/// Whether `start` is a `<c>` in column A.
fn a_cell(start: &BytesStart<'_>) -> Result<bool, WriteError> {
    Ok(named(start, "c") && column(start)? == 1)
}

//! Writes the `sheetData` of the part with the `row`s that have a height in
//! the table: the height of a row that is there set or removed, and a row
//! that is not there written in its place. Only the start tags of the `row`s
//! are looked at; the cells are written as they were.

use super::{RowOps, row_attributes};
use crate::error::WriteError;
use crate::sheet::RowHeight;
use crate::sheet::edit::apply::Rw;
use crate::sheet::edit::name::is_main;
use crate::sheet::edit::pass::through;
use crate::sheet::edit::tag::{attribute, name_of, retag, write_empty, write_end};
use std::ops::RangeBounds;
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};
use xlsx_graft_xml::events::{BytesStart, Event};

/// Where the walk of the rows is.
struct Walk {
    /// The row number of the last `row` read.
    previous: u32,
    /// The greatest row number read: the rows with a height up to it were
    /// written.
    done: u32,
}

/// The row number of `row`: its `r`, or the one after `previous` when it has
/// none.
///
/// # Errors
///
/// [`WriteError::Invalid`] for an `r` that is not a row of the sheet.
fn number(rw: &Rw<'_>, row: &BytesStart<'_>, previous: u32) -> Result<u32, WriteError> {
    let number = match attribute(row, "r") {
        Some(text) => text.trim().parse::<u32>().ok(),
        None => previous.checked_add(1),
    };
    number
        .filter(|&number| RowHeight::new(number, 0.0).is_ok())
        .ok_or_else(|| {
            WriteError::Invalid(Diagnostic::XmlSchemaInvalid {
                offset: rw.offset(),
                reason: SchemaReason::InvalidValue,
                name: Echo::new("r"),
            })
        })
}

impl RowOps {
    /// Writes a `row` for each height to set whose row is in `rows`.
    fn insert(
        &self,
        rw: &mut Rw<'_>,
        prefix: &str,
        rows: impl RangeBounds<u32>,
    ) -> Result<(), WriteError> {
        for height in self.0.range(rows).filter_map(|(_, height)| height.as_ref()) {
            write_empty(rw, &format!("{prefix}row"), &row_attributes(height))?;
        }
        Ok(())
    }

    /// Writes the `row` `row` (`opened`: it has an end tag), after the rows
    /// with a height that come before it, with its height set or removed
    /// when the table has one for its row. A row whose number was already
    /// read is written as it is.
    fn row(
        &self,
        rw: &mut Rw<'_>,
        prefix: &str,
        (row, opened): (&BytesStart<'_>, bool),
        walk: &mut Walk,
    ) -> Result<(), WriteError> {
        let at = number(rw, row, walk.previous)?;
        walk.previous = at;
        let mut changes = None;
        if at > walk.done {
            self.insert(rw, prefix, walk.done + 1..at)?;
            walk.done = at;
            changes = self.0.get(&at).map(|height| match height {
                Some(height) => height.attributes().map(|(key, value)| (key, Some(value))),
                None => [("ht", None), ("customHeight", None)],
            });
        }
        let tag = changes.map_or_else(|| row.borrow(), |changes| retag(row, &changes));
        if opened {
            rw.write(Event::Start(tag))?;
            through(rw, 1, true)
        } else {
            Ok(rw.write(Event::Empty(tag))?)
        }
    }

    /// Writes the `sheetData` `start` (`opened`: it has an end tag) with the
    /// heights of the table.
    ///
    /// # Errors
    ///
    /// The error of the reader or the writer; [`WriteError::Invalid`] for a
    /// `row` whose `r` is not a row of the sheet.
    pub(in crate::sheet::edit) fn stream(
        &self,
        rw: &mut Rw<'_>,
        prefix: &str,
        (start, opened): (BytesStart<'static>, bool),
    ) -> Result<(), WriteError> {
        if !opened {
            if !self.has_heights() {
                return Ok(rw.write(Event::Empty(start))?);
            }
            let name = name_of(&start);
            rw.write(Event::Start(start))?;
            self.insert(rw, prefix, 1..)?;
            return write_end(rw, &name);
        }
        rw.write(Event::Start(start))?;
        let mut walk = Walk {
            previous: 0,
            done: 0,
        };
        let mut buf = Vec::new();
        while let Some(event) = rw.next(&mut buf)? {
            match event {
                Event::Start(row) if is_row(rw, &row) => {
                    self.row(rw, prefix, (&row, true), &mut walk)?;
                }
                Event::Empty(row) if is_row(rw, &row) => {
                    self.row(rw, prefix, (&row, false), &mut walk)?;
                }
                Event::Start(other) => {
                    rw.write(Event::Start(other))?;
                    through(rw, 1, true)?;
                }
                Event::End(end) => {
                    self.insert(rw, prefix, walk.done + 1..)?;
                    rw.write(Event::End(end))?;
                    break;
                }
                other => rw.write(other)?,
            }
        }
        Ok(())
    }
}

/// Whether `row` is a `row` of the main namespace.
fn is_row(rw: &Rw<'_>, row: &BytesStart<'_>) -> bool {
    is_main(&rw.resolve_element(row.name()), "row")
}

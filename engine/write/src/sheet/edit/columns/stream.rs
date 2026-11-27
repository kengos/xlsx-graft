//! Writes a `cols` of the part with the widths of the table: each `col`
//! cut at the edges of the intervals, and a new `col` for the widths set in
//! the gaps between the `col`s and after the last.

use super::{ColumnOps, LAST, Piece};
use crate::error::WriteError;
use crate::sheet::ColumnWidth;
use crate::sheet::edit::apply::Rw;
use crate::sheet::edit::name::is_main;
use crate::sheet::edit::pass::through;
use crate::sheet::edit::tag::{attribute, has_besides, name_of, retag, write_empty, write_end};
use xlsx_graft_cell::number_text;
use xlsx_graft_xml::events::{BytesStart, Event};

/// The attributes of a `col` that a width sets.
const WIDTH: [&str; 4] = ["min", "max", "width", "customWidth"];

/// The first and last column of `col`, when its `min` and `max` are columns
/// in order.
fn columns_of(col: &BytesStart<'_>) -> Option<(u16, u16)> {
    let number = |key| attribute(col, key)?.trim().parse::<u16>().ok();
    let (min, max) = (number("min")?, number("max")?);
    ColumnWidth::new(min, max, 0.0).ok().map(|_| (min, max))
}

/// Writes `<col>`s for the widths (first column, last column, width).
fn write_cols(rw: &mut Rw<'_>, prefix: &str, widths: &[(u16, u16, f64)]) -> Result<(), WriteError> {
    for (first, last, width) in widths {
        let attributes = [
            ("min", first.to_string()),
            ("max", last.to_string()),
            ("width", number_text(*width).unwrap_or_default()),
            ("customWidth", "1".to_owned()),
        ];
        write_empty(rw, &format!("{prefix}col"), &attributes)?;
    }
    Ok(())
}

/// Writes `col` as it is, with the rest of it when `opened`.
fn copy_col(rw: &mut Rw<'_>, col: &BytesStart<'_>, opened: bool) -> Result<(), WriteError> {
    if opened {
        rw.write(Event::Start(col.borrow()))?;
        through(rw, 1, true)
    } else {
        Ok(rw.write(Event::Empty(col.borrow()))?)
    }
}

/// Whether `col` is a `col` of the main namespace.
fn is_col(rw: &Rw<'_>, col: &BytesStart<'_>) -> bool {
    is_main(&rw.resolve_element(col.name()), "col")
}

impl ColumnOps {
    /// The widths to write in the columns after `covered` up to `to`.
    fn fresh(&self, covered: u16, to: u16) -> Vec<(u16, u16, f64)> {
        self.widths(covered.saturating_add(1), to)
    }

    /// Writes `col` (`opened`: it has an end tag), the columns `min` to
    /// `max`, as the table says: as it is when no interval touches it, else
    /// cut into runs, each with its width set or removed, and a run that is
    /// left with no attribute but `min` and `max` not written.
    fn write_col(
        &self,
        rw: &mut Rw<'_>,
        col: &BytesStart<'_>,
        (min, max): (u16, u16),
        opened: bool,
    ) -> Result<(), WriteError> {
        let pieces = self.pieces(min, max);
        if pieces.iter().all(|piece| piece.width.is_none()) {
            return copy_col(rw, col, opened);
        }
        through(rw, usize::from(opened), false)?;
        let bare = !has_besides(col, &WIDTH);
        for piece in pieces
            .into_iter()
            .filter(|p| !(bare && p.width == Some(None)))
        {
            let Piece { first, last, width } = piece;
            let set = |width: f64| number_text(width).unwrap_or_default();
            let mut changes = vec![
                ("min", Some(first.to_string())),
                ("max", Some(last.to_string())),
            ];
            changes.extend(width.into_iter().flat_map(|width| {
                [
                    ("width", width.map(set)),
                    ("customWidth", width.map(|_| "1".to_owned())),
                ]
            }));
            rw.write(Event::Empty(retag(col, &changes)))?;
        }
        Ok(())
    }

    /// Writes the `col` `col` (`opened`: it has an end tag) as `write_col`
    /// says, after the widths set in the gap before it, and returns the last
    /// column covered, `covered` for a `col` whose `min` and `max` are not
    /// columns in order, which is written as it is.
    fn col(
        &self,
        rw: &mut Rw<'_>,
        prefix: &str,
        col: &BytesStart<'_>,
        opened: bool,
        covered: u16,
    ) -> Result<u16, WriteError> {
        match columns_of(col) {
            None => {
                copy_col(rw, col, opened)?;
                Ok(covered)
            }
            Some((min, max)) => {
                write_cols(rw, prefix, &self.fresh(covered, min - 1))?;
                self.write_col(rw, col, (min, max), opened)?;
                Ok(covered.max(max))
            }
        }
    }

    /// Writes the `cols` number `index` `start` (`opened`: it has an end
    /// tag) with the widths of the table. `covered` is the last column the
    /// `col`s before it cover; the returned number is the same after the
    /// `cols` number `index`. The widths past the last `col` are written at the end of the
    /// last `cols`. `covered` is the greatest `max` read, so for `col`s not in
    /// ascending order of `min` (`<col min="5" max="6"/><col min="1"
    /// max="2"/>`) a width in columns below it that no `col` covers is not
    /// written: Excel repairs such a `cols` itself.
    ///
    /// # Errors
    ///
    /// The error of the reader or the writer.
    pub(in crate::sheet::edit) fn stream(
        &self,
        rw: &mut Rw<'_>,
        prefix: &str,
        (start, opened): (BytesStart<'static>, bool),
        (index, mut covered): (usize, u16),
    ) -> Result<u16, WriteError> {
        // Whether no `cols` comes after the `cols` number `index`.
        let last = index + 1 >= self.elements;
        let rest_from = |covered: u16| if last { covered } else { LAST };
        if !opened {
            let rest = self.fresh(rest_from(covered), LAST);
            if rest.is_empty() {
                rw.write(Event::Empty(start))?;
                return Ok(covered);
            }
            let name = name_of(&start);
            rw.write(Event::Start(start))?;
            write_cols(rw, prefix, &rest)?;
            write_end(rw, &name)?;
            return Ok(LAST);
        }
        rw.write(Event::Start(start))?;
        let mut buf = Vec::new();
        while let Some(event) = rw.next(&mut buf)? {
            match event {
                Event::Start(col) if is_col(rw, &col) => {
                    covered = self.col(rw, prefix, &col, true, covered)?;
                }
                Event::Empty(col) if is_col(rw, &col) => {
                    covered = self.col(rw, prefix, &col, false, covered)?;
                }
                Event::Start(other) => {
                    rw.write(Event::Start(other))?;
                    through(rw, 1, true)?;
                }
                Event::End(end) => {
                    write_cols(rw, prefix, &self.fresh(rest_from(covered), LAST))?;
                    rw.write(Event::End(end))?;
                    break;
                }
                other => rw.write(other)?,
            }
        }
        Ok(covered)
    }
}

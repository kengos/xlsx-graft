//! The column widths a `SheetEdit` sets: a table of intervals of columns,
//! each to a width or to none, where a later call overwrites the intervals
//! it overlaps. The `col`s of the part are not held; `columns/stream.rs`
//! applies the table to them as they are read.

mod stream;

use crate::sheet::{ColumnWidth, Columns, SheetError};
use std::collections::BTreeMap;
use std::io::{self, Write};

/// The last column, XFD.
const LAST: u16 = 16_384;

/// A run of the columns of a `col`: where it is in a table's interval, the
/// interval's width (`Some(None)`: the width is removed), or `None` for a
/// run no interval covers.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Piece {
    first: u16,
    last: u16,
    width: Option<Option<f64>>,
}

/// Intervals of columns, none overlapping, by their first column: the last
/// column and the width to set (`None`: to remove).
#[derive(Debug)]
pub(super) struct ColumnOps {
    intervals: BTreeMap<u16, (u16, Option<f64>)>,
    /// How many `cols` the part has: the widths that no `col` is in the gap
    /// before are written at the end of the last one.
    elements: usize,
}

impl ColumnOps {
    pub(super) fn new(elements: usize) -> Self {
        Self {
            intervals: BTreeMap::new(),
            elements,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Sets the width of the columns `first` to `last`, as
    /// `SheetEdit::set_column_width` says.
    pub(super) fn set(
        &mut self,
        first: u16,
        last: u16,
        width: Option<f64>,
    ) -> Result<(), SheetError> {
        ColumnWidth::new(first, last, width.unwrap_or(0.0))?;
        // The intervals that overlap, taken out whole; the parts of them
        // outside `first` to `last` are put back.
        let overlapping: Vec<(u16, (u16, Option<f64>))> = (self.intervals.range(..=last).rev())
            .take_while(|(_, (end, _))| *end >= first)
            .map(|(start, value)| (*start, *value))
            .collect();
        for (start, (end, old)) in overlapping {
            self.intervals.remove(&start);
            if start < first {
                self.intervals.insert(start, (first - 1, old));
            }
            if end > last {
                self.intervals.insert(last + 1, (end, old));
            }
        }
        self.intervals.insert(first, (last, width));
        Ok(())
    }

    /// The intervals' parts in `from` to `to`, in order: first column, last
    /// column and width.
    fn within(&self, from: u16, to: u16) -> Vec<(u16, u16, Option<f64>)> {
        let found = (self.intervals.range(..=to).rev())
            .take_while(|(_, (end, _))| *end >= from)
            .map(|(start, (end, width))| (*start.max(&from), *end.min(&to), *width))
            .filter(|(first, last, _)| first <= last);
        let mut found: Vec<_> = found.collect();
        found.reverse();
        found
    }

    /// The columns `min` to `max` of a `col`, cut at the edges of the
    /// intervals: the runs in order.
    fn pieces(&self, min: u16, max: u16) -> Vec<Piece> {
        let mut pieces = Vec::new();
        let mut next = min;
        for (first, last, width) in self.within(min, max) {
            if next < first {
                pieces.push(Piece {
                    first: next,
                    last: first - 1,
                    width: None,
                });
            }
            pieces.push(Piece {
                first,
                last,
                width: Some(width),
            });
            next = last + 1;
        }
        if next <= max {
            pieces.push(Piece {
                first: next,
                last: max,
                width: None,
            });
        }
        pieces
    }

    /// The parts of the intervals that set a width, in `from` to `to`.
    fn widths(&self, from: u16, to: u16) -> Vec<(u16, u16, f64)> {
        (self.within(from, to).into_iter())
            .filter_map(|(first, last, width)| width.map(|width| (first, last, width)))
            .collect()
    }

    /// Writes a new `cols` of the widths set, for a part that has none.
    pub(super) fn write_new(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        let widths = (self.widths(1, LAST).into_iter())
            .filter_map(|(first, last, width)| ColumnWidth::new(first, last, width).ok())
            .collect();
        // The table's intervals are in ascending order without overlap and
        // `set` checked each one with `ColumnWidth::new`, so `Columns::new`
        // returns `Ok`; an `Err` would write nothing.
        let columns = Columns::new(widths);
        (columns.into_iter()).try_for_each(|columns| columns.write(out, prefix))
    }
}

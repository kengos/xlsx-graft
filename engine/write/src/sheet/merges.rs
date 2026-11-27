//! [`MergedCells`]: the `mergeCells` of a sheet (ECMA-376-1:2016
//! §18.3.1.55 and §18.3.1.54).

mod sweep;

use super::{CellRange, SheetError, write_attributes};
use std::io::{self, Write};

pub(crate) use sweep::first_overlap;

/// The ranges of cells a sheet merges: none of one cell, none that share a
/// cell, and no more than a sheet holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedCells {
    ranges: Vec<CellRange>,
}

impl MergedCells {
    /// The most merged ranges a sheet holds.
    const LIMIT: usize = 262_144;

    /// The ranges to merge, as given.
    ///
    /// # Errors
    ///
    /// [`SheetError::TooManyMerges`] for more than 262,144 ranges, checked
    /// first; [`SheetError::MergeSingleCell`] for the first range of one
    /// cell; then [`SheetError::MergeOverlap`] for two that share a cell,
    /// `range` the one later in the order given.
    pub fn new(ranges: Vec<CellRange>) -> Result<Self, SheetError> {
        if ranges.len() > Self::LIMIT {
            return Err(SheetError::TooManyMerges {
                count: ranges.len(),
                limit: Self::LIMIT,
            });
        }
        if let Some(&range) = ranges.iter().find(|range| range.is_single_cell()) {
            return Err(SheetError::MergeSingleCell { range });
        }
        match first_overlap(&[], &ranges) {
            Some((range, other)) => Err(SheetError::MergeOverlap { range, other }),
            None => Ok(Self { ranges }),
        }
    }

    /// The ranges, in the order given.
    #[must_use]
    pub fn ranges(&self) -> &[CellRange] {
        &self.ranges
    }

    /// Writes `<mergeCells count="n">` with a `<mergeCell ref="A1:B2"/>` for
    /// each range, in the order given; nothing for no ranges. `prefix` goes
    /// before the element names.
    ///
    /// # Errors
    ///
    /// The error of `out`.
    pub fn write(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        if self.ranges.is_empty() {
            return Ok(());
        }
        write!(out, r#"<{prefix}mergeCells count="{}">"#, self.ranges.len())?;
        for range in &self.ranges {
            write!(out, "<{prefix}mergeCell")?;
            write_attributes(out, &[("ref", range.to_string())])?;
            out.write_all(b"/>")?;
        }
        write!(out, "</{prefix}mergeCells>")
    }
}

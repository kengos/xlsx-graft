//! [`Extent`]: what a worksheet says of its extent, taken from the start
//! tags of its `dimension` and of the `mergeCell` children of its
//! `mergeCells`: only their `ref` attribute, as the crate root says.
//!
//! Both elements are passed over like every other child of `worksheet`
//! that is not read; nothing here checks their shape, so a `dimension` or a
//! `mergeCell` that is wrong never stops the rows from being read.

use super::SheetRows;
use crate::CellRange;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit};
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::BytesStart;

/// The most characters of a `dimension`'s `ref` that are kept: a `ref` that
/// names a range has at most 21 (`XFD1048576:XFD1048576`), so a longer one
/// is no range, and what is kept does not grow with the part's XML event.
pub(super) const MAX_DIMENSION_CHARS: usize = 1_000;

/// The `dimension` and the merged ranges read so far.
#[derive(Debug)]
pub(super) struct Extent {
    /// The `ref` of the `dimension`, as written.
    pub(super) dimension: Option<String>,
    /// The merged ranges kept, in the sheet's order.
    pub(super) merges: Vec<CellRange>,
    /// How many `mergeCell` elements came, kept or not.
    pub(super) count: u64,
    /// The most ranges kept.
    limit: Limit,
    /// Whether the walk is inside a `mergeCells` that has content.
    pub(super) in_merges: bool,
}

impl Extent {
    /// An extent nothing was read for, keeping at most `limit` ranges.
    pub(super) fn new(limit: Limit) -> Self {
        Self {
            dimension: None,
            merges: Vec::new(),
            count: 0,
            limit,
            in_merges: false,
        }
    }

    /// Takes the `ref` of the `dimension` that starts with `start`.
    pub(super) fn dimension(&mut self, start: &BytesStart<'_>) {
        // A new string of the first characters only: `truncate` would keep
        // the capacity allocated for the whole value.
        self.dimension = with_value(start, |text| {
            text.chars().take(MAX_DIMENSION_CHARS).collect()
        });
    }

    /// Takes the `mergeCell` that starts with `start`: counts it, and keeps
    /// its `ref` while no more than the limit came, if the `ref` names a
    /// range.
    pub(super) fn merge(&mut self, start: &BytesStart<'_>) {
        self.count = self.count.saturating_add(1);
        if self.count <= self.limit.value {
            let range = reference(start).and_then(|text| CellRange::parse(&text).ok());
            self.merges.extend(range);
        }
    }

    /// The limit the ranges are kept under.
    pub(super) fn limit(&self) -> Limit {
        self.limit
    }
}

/// The value of the attribute `ref` of `start`, with no prefix, as XML 1.0
/// normalizes it; `None` when there is none or it cannot be decoded.
fn reference(start: &BytesStart<'_>) -> Option<String> {
    with_value(start, str::to_owned)
}

/// What `make` makes of the value of the attribute `ref` of `start`, as
/// [`reference`] says; `None` when there is no value.
fn with_value<T>(start: &BytesStart<'_>, make: impl FnOnce(&str) -> T) -> Option<T> {
    let attribute = start
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == "ref")?;
    let value = attribute.normalized_value(XmlVersion::Implicit1_0).ok()?;
    Some(make(&value))
}

impl<R> SheetRows<R> {
    /// Whether the sheet has more `mergeCell` elements than `maxMergedCells`,
    /// so that [`merges`](Self::merges) lists the first ones only. Known once
    /// [`next_row`](Self::next_row) has returned `false`; for a sheet that
    /// stopped early it says only whether the limit was passed before it.
    #[must_use]
    pub fn merges_cut(&self) -> bool {
        let extent = &self.walk.extent;
        extent.count > extent.limit().value
    }

    /// Pushes `merged_cells_too_many` against the sheet's part when the
    /// sheet has more `mergeCell` elements than `maxMergedCells` and the
    /// part was read to its end ([`complete`](Self::complete) is `true`);
    /// otherwise pushes nothing, since a part that stopped early has not
    /// shown how many it holds.
    ///
    /// The limit only decides how many ranges [`merges`](Self::merges)
    /// keeps (the first `maxMergedCells`). Reading the rows never pushes it,
    /// nor does it stop them, so a caller that reads cells (the CLI's
    /// `read`) adds no diagnostic for it; a caller that lists the merged
    /// ranges calls this once it has read the rows, to say that the list is
    /// cut. Call it once, after the rows are read: each call pushes the
    /// diagnostic again.
    pub fn push_merge_limit(&self, diagnostics: &mut Diagnostics) {
        let extent = &self.walk.extent;
        if self.complete && extent.count > extent.limit().value {
            let too_many = Diagnostic::MergedCellsTooMany {
                count: extent.count,
                limit: extent.limit(),
            };
            diagnostics.push(too_many, self.location.clone());
        }
    }
}

#[cfg(test)]
mod tests;

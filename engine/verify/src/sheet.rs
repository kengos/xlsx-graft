//! [`SheetCheck`]: what Excel repairs in a worksheet as it opens it, found
//! as the part is read as a stream, event by event, so a large worksheet
//! costs no more memory than a small one; the merged cells alone are kept,
//! up to `maxMergedCells`.
//!
//! Every problem is found, not only the first, each once. Which are
//! problems, and how heavy, is what Excel for Mac 16.114.3 did as it opened
//! a workbook that had it: an error where it offered to repair the
//! workbook, or moved a value without a word; a notice where it dropped
//! what was wrong without one.
//!
//! - **Where**: a row's `r` from 1 to 1,048,576 and a cell's up to XFD in
//!   its row (`cell_reference_invalid`), each past the one before
//!   (`cell_order_invalid`), as `xlsx_graft_read::SheetRows` reads them; a
//!   row or cell without `r` comes after the one before. Excel drops a row
//!   or cell that goes back or repeats one, so one refused is not kept: the
//!   next is placed after the one before it, and the cells of a row
//!   refused are not placed.
//! - **Shared strings**: a `t="s"` cell's index past the table
//!   (`shared_string_missing`).
//! - **Styles**: a cell's `s` (`cell_style_missing`) or a column's `style`
//!   (`column_style_missing`) at or past the number of cell formats; a
//!   row's `s` too, where its `customFormat` puts it in force
//!   (§18.3.1.73), is a notice (`row_style_missing`), as Excel drops it
//!   without a word.
//! - **Merged cells**: a range that overlaps one taken before it in the
//!   sheet, the same range twice included, is dropped, as Excel drops the
//!   second of two; two that touch at an edge or a corner do not overlap,
//!   and one dropped is no reason to drop another (`merged_cells_overlap`).
//!   A `ref` that does not read as a range in the sheet is not checked.
//!   Past `maxMergedCells`, the sheet is refused (`merged_cells_too_many`)
//!   and the ranges past it are not checked.
//! - **Text**: an inline string (`t="inlineStr"`) longer than 32,767
//!   UTF-16 code units, its runs together and its phonetic runs left out
//!   (`inline_string_too_long`). A formula's cached string is not checked:
//!   Excel keeps it whole.
//!
//! Only the elements of the root's namespace, `worksheet` in SpreadsheetML,
//! are read, where the schema puts them; anything else is passed over with
//! all it holds. Attributes are read unprefixed. The `dimension` is not
//! read: Excel does not use it, and writes it again as it saves.

mod elements;
mod merges;
mod place;

use crate::collect::attribute;
use crate::count::Unsigned;
use elements::{Cell, Kind};
use merges::Merges;
use place::Place;
use std::borrow::Cow;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit, Location};
use xlsx_graft_xml::events::BytesStart;
use xlsx_graft_xml::name::{Namespace, ResolveResult};

/// The namespaces of SpreadsheetML, Transitional and Strict
/// (ECMA-376-1:2016 §8.1, ECMA-376-4:2016 §8.1).
const SPREADSHEETML: [&str; 2] = [
    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "http://purl.oclc.org/ooxml/spreadsheetml/main",
];

/// What the workbook holds that a sheet's cells refer to: how many shared
/// strings and how many cell formats (`cellXfs`). `None` for one not
/// known, whose references are not checked.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Counts {
    /// The shared strings, 0 for a workbook without the table.
    pub(crate) strings: Option<u64>,
    /// The cell formats.
    pub(crate) styles: Option<u64>,
}

/// An element of the worksheet read, where the schema puts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frame {
    Worksheet,
    Cols,
    SheetData,
    Row,
    Cell,
    Value,
    Inline,
    Run,
    Text,
    MergeCells,
}

/// The checks of one worksheet, handed its XML event by event, pushing
/// what they find against its part.
pub(crate) struct SheetCheck<'a> {
    diagnostics: &'a mut Diagnostics,
    location: Location,
    counts: Counts,
    /// The root's namespace, once it is a `worksheet`.
    root: Option<&'static str>,
    /// Whether the root was read.
    rooted: bool,
    frames: Vec<Frame>,
    /// How many elements deep the walk is inside one passed over.
    skipped: u64,
    place: Place,
    cell: Cell,
    merges: Merges,
}

impl<'a> SheetCheck<'a> {
    /// The checks of the worksheet `part`, against `counts`, keeping
    /// `max_merged_cells` merged ranges, pushing into `diagnostics`.
    pub(crate) fn new(
        part: &str,
        counts: Counts,
        max_merged_cells: Limit,
        diagnostics: &'a mut Diagnostics,
    ) -> Self {
        Self {
            diagnostics,
            location: Location::default().with_part(part),
            counts,
            root: None,
            rooted: false,
            frames: Vec::new(),
            skipped: 0,
            place: Place::default(),
            cell: Cell::default(),
            merges: Merges::new(max_merged_cells),
        }
    }

    /// Reads the start of an element, of namespace `resolved` and local
    /// name `local`: a start tag when `opened`, else an empty element.
    pub(crate) fn start(
        &mut self,
        resolved: &ResolveResult<'_>,
        local: &str,
        start: &BytesStart<'_>,
        opened: bool,
    ) {
        if self.skipped > 0 {
            self.skipped = self.skipped.saturating_add(u64::from(opened));
            return;
        }
        let namespace = match resolved {
            ResolveResult::Bound(Namespace(ns)) => SPREADSHEETML.into_iter().find(|&n| n == *ns),
            _ => None,
        };
        let frame = match self.frames.last() {
            None if !self.rooted && namespace.is_some() && local == "worksheet" => {
                self.root = namespace;
                Some(Frame::Worksheet)
            }
            Some(&frame) if namespace.is_some() && namespace == self.root => {
                self.child(frame, local, start)
            }
            _ => None,
        };
        self.rooted = true;
        match frame {
            Some(frame) if opened => self.frames.push(frame),
            Some(frame) => self.close(frame),
            None => self.skipped = u64::from(opened),
        }
    }

    /// Reads the end of the element open.
    pub(crate) fn end(&mut self) {
        if self.skipped > 0 {
            self.skipped -= 1;
        } else if let Some(frame) = self.frames.pop() {
            self.close(frame);
        }
    }

    /// Reads text that `read` gives, in the element open, when a check
    /// needs it: the element's character data, its references resolved.
    pub(crate) fn text<'t>(&mut self, read: impl FnOnce() -> Cow<'t, str>) {
        if self.skipped > 0 {
            return;
        }
        match (self.frames.last(), self.cell.kind) {
            (Some(Frame::Value), Kind::Shared) => self.cell.index.push(&read()),
            (Some(Frame::Text), Kind::Inline) => self.cell.length.push(&read()),
            _ => {}
        }
    }

    /// Ends the checks: pushes what the merged cells hold.
    pub(crate) fn finish(self) {
        self.merges.finish(self.diagnostics, &self.location);
    }

    /// The element `local` inside `frame`, that `start` starts, read as it
    /// starts, if it is one the checks read.
    fn child(&mut self, frame: Frame, local: &str, start: &BytesStart<'_>) -> Option<Frame> {
        match (frame, local) {
            (Frame::Worksheet, "cols") => Some(Frame::Cols),
            (Frame::Worksheet, "sheetData") => Some(Frame::SheetData),
            (Frame::Worksheet, "mergeCells") => Some(Frame::MergeCells),
            (Frame::Cols, "col") => {
                self.column(start);
                None
            }
            (Frame::SheetData, "row") => {
                self.row(start);
                Some(Frame::Row)
            }
            (Frame::Row, "c") => {
                self.cell(start);
                Some(Frame::Cell)
            }
            (Frame::Cell, "v") => {
                // A second `v`, which the schema does not allow, is read
                // alone, not joined to the first.
                self.cell.index = Unsigned::default();
                Some(Frame::Value)
            }
            (Frame::Cell, "is") => Some(Frame::Inline),
            (Frame::Inline, "r") => Some(Frame::Run),
            (Frame::Inline | Frame::Run, "t") => Some(Frame::Text),
            (Frame::MergeCells, "mergeCell") => {
                self.merges.add(attribute(start, "ref").as_deref());
                None
            }
            _ => None,
        }
    }

    /// Ends `frame`.
    fn close(&mut self, frame: Frame) {
        match frame {
            Frame::Text => self.cell.length.end(),
            Frame::Cell => self.finish_cell(),
            _ => {}
        }
    }

    /// Pushes `diagnostic` against the part.
    #[track_caller]
    fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic, self.location.clone());
    }
}

#[cfg(test)]
mod tests;

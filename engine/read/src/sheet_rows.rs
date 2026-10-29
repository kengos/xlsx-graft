//! [`SheetRows`]: the rows of one worksheet, read as a stream from its part
//! one row at a time as the caller asks for them, as the crate root says.
//! The part is read through an [`EntryStream`] beside the package
//! ([`PartStream`]); the walk of its XML is `walk.rs`.

mod cell;
mod shape;
mod walk;

use crate::{DateSystem, ReadError, ReadLimits, Row, SharedStrings, Styles};
use cell::Cells;
use std::cell::RefCell;
use std::io::{self, BufReader, Read, Seek};
use std::rc::Rc;
use walk::Walk;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit, Location};
use xlsx_graft_package::{EntryStream, Package, PackageError};
use xlsx_graft_xml::XmlStreamReader;

/// The rows of one worksheet, from [`Book::rows`](crate::Book::rows).
///
/// [`next_row`](Self::next_row) reads the part up to the end of the next
/// row and puts it in the caller's [`Row`]. The memory it holds does not
/// grow with the rows: one XML event, under `maxXmlEventBytes`, the walk's
/// frames, and the row the caller hands in, whose text is held to
/// `maxRowBytes` (`row_too_large`) and whose values to 16,384.
///
/// A worksheet (ECMA-376-1:2016 §18.3) is read as a stream: of
/// `worksheet`, only `sheetData` is read, and of it each `row` and each
/// `c`; every other child, a cell's formula (`f`, its cached value `v`
/// being the value) and the extension lists are skipped unread, the
/// `dimension` included, which writers do not all keep right. Only the
/// rows written come, each with its number, and each holds the value of
/// every column from A to its last cell, a column with no cell being
/// empty.
///
/// - **Where**: a row's `r` is its number, from 1 to 1,048,576, and a
///   cell's `r` its A1 reference, up to XFD, in its row. Without `r`, a
///   row is the one after the row before, and a cell the one after the cell
///   before in its row, or A (MS-OI29500 on §18.3.1.4, note c). Rows go
///   down, and a row's cells across, each past the one before: a row or
///   cell at or before the one before is `cell_order_invalid` (Office takes
///   two rows of one number, MS-OI29500 on §18.3.1.73, note a, but does not
///   say what they hold together, so readers would differ), and a place
///   outside the sheet or a cell outside its row `cell_reference_invalid`.
/// - **What**: a cell's `t` (§18.18.11) says what its `v` is. `n`, the
///   default, a number, made the date or time its format says
///   ([`Value::from_serial`](crate::Value::from_serial)); `s` the index of
///   a shared string, which a table without it makes
///   `shared_string_missing`; `str` a string, `b` a boolean, `e` an error
///   value, and `d` an ISO 8601 date-time
///   ([`Value::from_iso`](crate::Value::from_iso), white space around it
///   not taken: `v` is an ST_Xstring). An `inlineStr` cell's value is its
///   `is`, read as a shared string is. The text of a `v` and a `t` has its
///   `_xHHHH_` escapes undone, and a cell without its `v` or `is` is
///   empty. A `v` its type does not take (a number that is not finite, an
///   empty error value), and an `s` that is not an `unsignedInt` on a
///   number, is `xml_schema_invalid`. A number's `s` is the cell format the
///   styles part says the kind of; a string's is not read.
/// - **Schema**: as for the workbook part, against CT_Worksheet,
///   CT_SheetData, CT_Row and CT_Cell (Strict `sml.xsd`, and the
///   Transitional twin, whose `legacyDrawing` and `legacyDrawingHF` are
///   taken in either kind, as the workbook part takes
///   `dateCompatibility`), and CT_Rst for `is`; the values of the
///   attributes not read are not checked. Markup compatibility is taken as
///   for the workbook part, and a `mc:AlternateContent` right inside
///   `worksheet`, where Excel writes form controls, is skipped unread; one
///   in `sheetData` is refused.
///
/// Reading stops at the first problem, with its one diagnostic, against
/// the sheet's part: the rows before it have come, and none comes after.
/// A cell of a shared strings table not known stops it too, the table's
/// own diagnostic standing for it. The part is read to its end whatever
/// stops it, and a damaged entry's own diagnostic is the only one, as for
/// the shared strings table; the rows that came before were read from its
/// bytes too.
pub struct SheetRows<R> {
    /// The part's XML, until its end or the problem that stopped it.
    xml: Option<XmlStreamReader<BufReader<PartStream<R>>>>,
    strings: Rc<RefCell<SharedStrings>>,
    styles: Rc<Styles>,
    date_system: DateSystem,
    /// Where a diagnostic about the part points.
    location: Location,
    walk: Walk,
    /// A shared string looked up.
    scratch: String,
    /// The most bytes of text a row may hold.
    max_row_bytes: Limit,
}

/// The part read through `stream`, from the package it shares with the
/// book and the other readers: borrowed only while a read lasts.
struct PartStream<R> {
    package: Rc<RefCell<Package<R>>>,
    stream: EntryStream,
    /// What the stream pushes, until the reader hands it on.
    diagnostics: Diagnostics,
}

/// Why reading the part stopped before its end.
#[derive(Debug)]
enum Halt {
    /// A problem in the part, to push unless the entry turns out damaged;
    /// `None` when one already pushed stands for it.
    Refused(Option<Diagnostic>),
    /// The reader of the part failed: the zip entry, which reported why
    /// when it stopped, or the host.
    Io(io::Error),
    /// The shared strings table's spool failed.
    Failed(ReadError),
}

impl From<Diagnostic> for Halt {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Refused(Some(diagnostic))
    }
}

impl<R: Read + Seek> Read for PartStream<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let mut package = self.package.try_borrow_mut().map_err(io::Error::other)?;
        package.read_stream(&mut self.stream, out, &mut self.diagnostics)
    }
}

impl<R: Read + Seek> SheetRows<R> {
    /// The rows of the part `stream` reads from `package`, at `location`.
    pub(crate) fn new(
        package: Rc<RefCell<Package<R>>>,
        stream: EntryStream,
        strings: Rc<RefCell<SharedStrings>>,
        styles: Rc<Styles>,
        date_system: DateSystem,
        limits: &ReadLimits,
        location: Location,
    ) -> Self {
        let part = PartStream {
            package,
            stream,
            diagnostics: Diagnostics::new(),
        };
        Self {
            xml: Some(XmlStreamReader::new(BufReader::new(part), &limits.xml)),
            strings,
            styles,
            date_system,
            location,
            walk: Walk::new(),
            scratch: String::new(),
            max_row_bytes: limits.max_row_bytes,
        }
    }

    /// Reads the next row into `row`, in place of what it held: `true`
    /// then, `false` once there is none, pushing into `diagnostics` the
    /// problem that stopped the reading, if one did. After `false`, what
    /// `row` holds is no row of the sheet.
    ///
    /// Only the rows the part writes come, in order, each with its number.
    /// Reading stops at the first problem; the rows before it have come
    /// already. Whatever stops it, the part is read to its end, so a
    /// damaged zip entry reports itself alone, in place of what its damage
    /// made the part look like; the rows read before are its bytes too.
    ///
    /// # Errors
    ///
    /// [`ReadError::Package`] when reading the package fails,
    /// [`ReadError::Spool`] when the shared strings table's spool does.
    pub fn next_row(
        &mut self,
        row: &mut Row,
        diagnostics: &mut Diagnostics,
    ) -> Result<bool, ReadError> {
        let Some(xml) = self.xml.as_mut() else {
            return Ok(false);
        };
        let mut cells = Cells {
            row,
            strings: &self.strings,
            styles: &self.styles,
            date_system: self.date_system,
            scratch: &mut self.scratch,
            max_row_bytes: self.max_row_bytes,
        };
        let halt = match self.walk.next(xml, &mut cells) {
            Ok(true) => return Ok(true),
            Ok(false) => None,
            Err(halt) => Some(halt),
        };
        let xml = self.xml.take();
        xml.map_or(Ok(false), |xml| self.finish(xml, halt, diagnostics))
    }

    /// Ends the reading at `halt`, or at the end of the part: reads the
    /// part to its end and pushes what stopped it.
    fn finish(
        &self,
        xml: XmlStreamReader<BufReader<PartStream<R>>>,
        halt: Option<Halt>,
        diagnostics: &mut Diagnostics,
    ) -> Result<bool, ReadError> {
        let mut source = xml.into_inner();
        let refused = match halt {
            None => None,
            Some(Halt::Failed(error)) => return Err(error),
            Some(Halt::Io(error)) if !source.get_ref().stream.is_stopped() => {
                return Err(PackageError::Io(error).into());
            }
            Some(Halt::Io(_)) => None,
            Some(Halt::Refused(diagnostic)) => diagnostic,
        };
        // Read to the end, so a damaged entry reports itself in place of
        // what its damage made the part look like.
        let drained = io::copy(&mut source, &mut io::sink());
        let part = source.into_inner();
        let stopped = part.stream.is_stopped();
        for record in part.diagnostics.finish() {
            diagnostics.push(record.diagnostic, record.location);
        }
        if let Err(error) = drained
            && !stopped
        {
            return Err(PackageError::Io(error).into());
        }
        if let Some(diagnostic) = refused
            && !stopped
        {
            diagnostics.push(diagnostic, self.location.clone());
        }
        Ok(false)
    }
}

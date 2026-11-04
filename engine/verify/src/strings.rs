//! [`read`]: a workbook's shared strings table, read before its sheets
//! are: how many strings it holds, which a cell's `t="s"` index is checked
//! against, and every string held to the 32,767 characters, counted in
//! UTF-16 code units, Excel keeps in a cell (`shared_string_too_long`).
//!
//! A string (`si`) is its `t`, or the `t` of each of its runs together
//! (ECMA-376-1:2016 §18.4.8), counted as a cell's inline string is
//! ([`Utf16`]): each `_xHHHH_` escape is the one code unit it stands for,
//! an unpaired surrogate included, as Excel counts it. Its phonetic runs
//! (`rPh`, §18.4.6) are a hint about how the text is read, not part of it,
//! and Excel does not count them with it: Excel for Mac 16.114.3 opens a
//! string of 32,767 characters with ten more in a phonetic run without a
//! repair. Every string is checked, whether a cell refers to it or not:
//! Excel repairs one no cell refers to all the same.
//!
//! Only the elements of the root's namespace, `sst` in SpreadsheetML, are
//! read, where the schema puts them; anything else is passed over with
//! all it holds. The part is read as a stream, event by event, keeping no
//! more of a string than its length so far, so a large table costs no
//! more memory than a small one.
//!
//! How many strings the table holds is known once it is read to its end,
//! its root an `sst`: when reading stops before, or the root is another,
//! it is not, and no cell's index is checked against it. Why reading
//! stops is not reported here: the part is read again as every part is,
//! and reported then. A string found too long before the zip entry turns
//! out damaged is reported all the same, as a worksheet's problems are.

use crate::count::Utf16;
use std::io::{BufRead, BufReader, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_package::{Package, PackageError};
use xlsx_graft_xml::events::Event;
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlLimits, XmlStreamError, XmlStreamReader};

/// The most UTF-16 code units of a string Excel keeps in a cell.
const MAX_UNITS: u64 = 32_767;

/// The namespaces of SpreadsheetML, Transitional and Strict
/// (ECMA-376-1:2016 §8.1, ECMA-376-4:2016 §8.1).
const SPREADSHEETML: [&str; 2] = [
    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "http://purl.oclc.org/ooxml/spreadsheetml/main",
];

/// An element of the table read, where the schema puts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frame {
    Table,
    Item,
    Run,
    Text,
}

/// Reads the shared strings table at `index` of `package` under `limits`,
/// as the module says, pushing `shared_string_too_long` into
/// `diagnostics` for every string longer than Excel keeps, and what
/// opening the part finds into `unreported`: how many strings it holds,
/// or `None` when that is not known.
pub(crate) fn read<R: Read + Seek>(
    package: &mut Package<R>,
    index: usize,
    limits: &XmlLimits,
    unreported: &mut Diagnostics,
    diagnostics: &mut Diagnostics,
) -> Result<Option<u64>, PackageError> {
    let location = (package.parts().get(index))
        .map(|part| Location::default().with_part(part.name().as_str()))
        .unwrap_or_default();
    let Some(entry) = package.open_part(index, unreported)? else {
        return Ok(None);
    };
    let mut xml = XmlStreamReader::new(BufReader::new(entry), limits);
    match walk(&mut xml, &location, diagnostics) {
        Ok(count) => Ok(count),
        Err(XmlStreamError::Invalid(_)) => Ok(None),
        // A zip entry that stops on damage of its own has reported it.
        Err(XmlStreamError::Io(_)) if xml.get_ref().get_ref().is_stopped() => Ok(None),
        Err(XmlStreamError::Io(error)) => Err(PackageError::Io(error)),
    }
}

/// Reads `xml` to its end, pushing at `location` every string longer than
/// Excel keeps: how many strings it holds, if its root is an `sst`.
fn walk<R: BufRead>(
    xml: &mut XmlStreamReader<R>,
    location: &Location,
    diagnostics: &mut Diagnostics,
) -> Result<Option<u64>, XmlStreamError> {
    let mut buf = Vec::new();
    let mut table = Table::default();
    loop {
        let event = xml.read_event_into(&mut buf)?;
        let opened = matches!(event, Event::Start(_));
        match event {
            Event::Start(start) | Event::Empty(start) => {
                let (resolved, local) = xml.resolve_element(start.name());
                table.start(&resolved, local.as_ref(), opened, location, diagnostics);
            }
            Event::End(_) => table.end(location, diagnostics),
            Event::Text(text) => table.text(&text.xml10_content()),
            Event::CData(data) => table.text(&data.xml10_content()),
            Event::GeneralRef(reference) => table.text(&crate::parts::resolve(&reference)),
            Event::Eof => return Ok(table.root.map(|_| table.count)),
            _ => {}
        }
        buf.clear();
    }
}

/// The table as it is read.
#[derive(Debug, Default)]
struct Table {
    /// The root's namespace, once it is an `sst`.
    root: Option<&'static str>,
    /// Whether the root was read.
    rooted: bool,
    frames: Vec<Frame>,
    /// How many elements deep the walk is inside one passed over.
    skipped: u64,
    /// The strings so far.
    count: u64,
    /// The length of the string open.
    length: Utf16,
}

impl Table {
    /// Reads the start of an element, of namespace `resolved` and local
    /// name `local`: a start tag when `opened`, else an empty element.
    fn start(
        &mut self,
        resolved: &ResolveResult<'_>,
        local: &str,
        opened: bool,
        location: &Location,
        diagnostics: &mut Diagnostics,
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
            None if !self.rooted && namespace.is_some() && local == "sst" => {
                self.root = namespace;
                Some(Frame::Table)
            }
            Some(&frame) if namespace.is_some() && namespace == self.root => match (frame, local) {
                (Frame::Table, "si") => Some(Frame::Item),
                (Frame::Item, "r") => Some(Frame::Run),
                (Frame::Item | Frame::Run, "t") => Some(Frame::Text),
                _ => None,
            },
            _ => None,
        };
        self.rooted = true;
        if frame == Some(Frame::Item) {
            self.length = Utf16::default();
        }
        match frame {
            Some(frame) if opened => self.frames.push(frame),
            Some(frame) => self.close(frame, location, diagnostics),
            None => self.skipped = u64::from(opened),
        }
    }

    /// Reads the end of the element open.
    fn end(&mut self, location: &Location, diagnostics: &mut Diagnostics) {
        if self.skipped > 0 {
            self.skipped -= 1;
        } else if let Some(frame) = self.frames.pop() {
            self.close(frame, location, diagnostics);
        }
    }

    /// Reads `text`, in the element open.
    fn text(&mut self, text: &str) {
        if self.skipped == 0 && self.frames.last() == Some(&Frame::Text) {
            self.length.push(text);
        }
    }

    /// Ends `frame`: a string, pushed at `location` when it is too long.
    fn close(&mut self, frame: Frame, location: &Location, diagnostics: &mut Diagnostics) {
        match frame {
            Frame::Text => self.length.end(),
            Frame::Item => {
                let (index, length) = (self.count, self.length.units());
                self.count = self.count.saturating_add(1);
                if length > MAX_UNITS {
                    let diagnostic = Diagnostic::SharedStringTooLong { index, length };
                    diagnostics.push(diagnostic, location.clone());
                }
            }
            Frame::Table | Frame::Run => {}
        }
    }
}

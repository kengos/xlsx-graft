//! Tests for the crate, driven through `Workbook::find`, `related` and
//! `SharedStrings::read` and `get` on packages the tests build (`zip.rs`).
//! Each file under `tests/` covers one aspect.

mod failures;
mod limits;
mod schema;
mod strings;
mod workbook;
mod zip;

use crate::{Lookup, ReadLimits, SharedStrings, Workbook};
use std::cell::Cell;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_spool::{MemorySpools, SpoolFactory};
use zip::Item;

/// The Transitional SpreadsheetML namespace.
const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
/// The Strict SpreadsheetML namespace.
const STRICT_NS: &str = "http://purl.oclc.org/ooxml/spreadsheetml/main";
/// The Transitional `sharedStrings` relationship type.
const SHARED_STRINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";
/// The Transitional `officeDocument` relationship type.
const OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
/// The workbook's Relationships part.
const WORKBOOK_RELS: &str = "/xl/_rels/workbook.xml.rels";

/// The reader every test reads a package from: bytes in memory whose reads
/// fail when they start in a range a test sets. One reader type for every
/// test, since the coverage gate counts a line generic over the reader as
/// covered only when one instantiation covers it.
#[derive(Debug)]
struct Reader {
    inner: Cursor<Vec<u8>>,
    /// Reads that start in `start..end` fail, as the host would, and so
    /// does a seek to `start`.
    poisoned: Rc<Cell<(u64, u64)>>,
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let (start, end) = self.poisoned.get();
        if (start..end).contains(&self.inner.position()) {
            return Err(io::Error::other("the disk went away"));
        }
        self.inner.read(buf)
    }
}

impl Seek for Reader {
    /// Fails a seek to the start of the poisoned range.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let (start, end) = self.poisoned.get();
        if start < end && pos == SeekFrom::Start(start) {
            return Err(io::Error::other("the disk went away"));
        }
        self.inner.seek(pos)
    }
}

/// What reading found: each diagnostic, with where it points.
type Seen = Vec<(Diagnostic, String)>;

/// The diagnostics in `diagnostics`, each with where it points: its entry,
/// part, or relationships part and `#` its relationship's `Id`.
fn seen(diagnostics: Diagnostics) -> Seen {
    let place = |location: Location| {
        let id = location.relationship_id.map(|id| id.as_str().to_owned());
        match (location.entry, location.part, location.rels_part) {
            (Some(entry), _, _) => entry.as_str().to_owned(),
            (None, Some(part), _) => part.as_str().to_owned(),
            (None, None, Some(rels)) => format!("{}#{}", rels.as_str(), id.unwrap_or_default()),
            (None, None, None) => String::new(),
        }
    };
    (diagnostics.finish().into_iter())
        .map(|record| (record.diagnostic, place(record.location)))
        .collect()
}

/// `(diagnostic, place)`, as [`seen`] lists one.
fn at(diagnostic: Diagnostic, place: &str) -> (Diagnostic, String) {
    (diagnostic, place.to_owned())
}

/// The relationship `id` of `rel_type` to `target`.
fn rel(id: &str, rel_type: &str, target: &str) -> String {
    format!(r#"<Relationship Id="{id}" Type="{rel_type}" Target="{target}"/>"#)
}

/// A Relationships part `name` holding `body`.
fn rels(name: &str, body: &str) -> Item {
    let ns = "http://schemas.openxmlformats.org/package/2006/relationships";
    let xml = format!(r#"<Relationships xmlns="{ns}">{body}</Relationships>"#);
    Item::stored(name, xml.as_bytes())
}

/// The content types stream: a `Default` for `rels` and one for `xml`.
fn content_types() -> Item {
    let types = concat!(
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        r#"<Default Extension="xml" ContentType="application/xml"/>"#,
        "</Types>",
    );
    Item::stored("[Content_Types].xml", types.as_bytes())
}

/// A package: the content types stream, the package relationships `main`,
/// the workbook part, its relationships `related`, then `more`.
fn package_of(main: &str, related: &str, more: Vec<Item>) -> Vec<Item> {
    let mut items = vec![
        content_types(),
        rels("_rels/.rels", main),
        Item::stored("xl/workbook.xml", b"<workbook/>"),
        rels("xl/_rels/workbook.xml.rels", related),
    ];
    items.extend(more);
    items
}

/// A workbook whose shared strings table is `part`, at
/// `xl/sharedStrings.xml`.
fn book_with(part: Item) -> Vec<Item> {
    let main = rel("rId1", OFFICE_DOCUMENT, "xl/workbook.xml");
    let related = rel("rId1", SHARED_STRINGS, "sharedStrings.xml");
    package_of(&main, &related, vec![part])
}

/// A workbook whose shared strings table holds `body` in a Transitional
/// `sst`.
fn book(body: &str) -> Vec<Item> {
    let xml = format!(r#"<sst xmlns="{NS}">{body}</sst>"#);
    book_with(Item::stored("xl/sharedStrings.xml", xml.as_bytes()))
}

/// The package of `items`, which must open.
fn open(items: Vec<Item>, diagnostics: &mut Diagnostics) -> Package<Reader> {
    open_poisoned(items, diagnostics, Rc::default())
}

/// [`open`], its reader failing in the range `poisoned` holds.
fn open_poisoned(
    items: Vec<Item>,
    diagnostics: &mut Diagnostics,
    poisoned: Rc<Cell<(u64, u64)>>,
) -> Package<Reader> {
    let reader = Reader {
        inner: Cursor::new(zip::build(items)),
        poisoned,
    };
    Package::open(reader, &Limits::default(), diagnostics)
        .unwrap()
        .unwrap()
}

/// Reads the shared strings of the package of `items` under `limits`,
/// spilling to `spools`.
fn read_with(
    items: Vec<Item>,
    limits: &ReadLimits,
    spools: &mut dyn SpoolFactory,
) -> (SharedStrings, Seen) {
    let mut diagnostics = Diagnostics::new();
    let mut package = open(items, &mut diagnostics);
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let strings =
        SharedStrings::read(&mut package, &workbook, limits, spools, &mut diagnostics).unwrap();
    (strings, seen(diagnostics))
}

/// [`read_with`] under the default limits, in memory.
fn read(items: Vec<Item>) -> (SharedStrings, Seen) {
    read_with(items, &ReadLimits::default(), &mut MemorySpools)
}

/// Every string of `strings`, looked up in order until one is not found;
/// asserts the table was read.
fn values(strings: &mut SharedStrings) -> Vec<String> {
    assert!(matches!(strings, SharedStrings::Table(_)), "{strings:?}");
    let mut out = Vec::new();
    let mut value = String::new();
    while strings.get(out.len() as u64, &mut value).unwrap() == Lookup::Found {
        out.push(value.clone());
    }
    out
}

/// The strings of a table of `body`, which must read without a
/// diagnostic.
fn clean(body: &str) -> Vec<String> {
    let (mut strings, seen) = read(book(body));
    assert_eq!(seen, vec![]);
    values(&mut strings)
}

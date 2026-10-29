//! A worksheet's rows: what a `Book` opens to and its `SheetRows` read,
//! and the helpers the files under `rows/` share.

mod failures;
mod order;
mod reading;
mod schema;

use super::zip::Item;
use super::{
    NS, OFFICE_DOCUMENT, Reader, SHARED_STRINGS, Seen, content_types, rel, rels, seen, zip,
};
use crate::{Book, ReadLimits, Row, Value};
use std::cell::Cell;
use std::io::Cursor;
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics};
use xlsx_graft_package::Limits;
use xlsx_graft_spool::MemorySpools;

/// The Transitional `worksheet` relationship type.
const WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
/// The Transitional `styles` relationship type.
const STYLES: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
/// The Transitional relationships namespace, of `r:id`.
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// Where the first sheet's part is.
const SHEET: &str = "/xl/worksheets/sheet1.xml";

/// A package whose workbook part lists a sheet named for each of `sheets`,
/// the worksheet part `xl/worksheets/sheet{n}.xml` holding its XML, with
/// `date1904` when `date1904`; then the workbook's relationships `related`
/// and `more`.
fn workbook(sheets: &[(&str, &str)], date1904: bool, related: &str, more: Vec<Item>) -> Vec<Item> {
    let listed: String = (sheets.iter().zip(1..))
        .map(|((name, _), n)| format!(r#"<sheet name="{name}" sheetId="{n}" r:id="rId{n}"/>"#))
        .collect();
    let pr = if date1904 {
        r#"<workbookPr date1904="1"/>"#
    } else {
        ""
    };
    let xml =
        format!(r#"<workbook xmlns="{NS}" xmlns:r="{R}">{pr}<sheets>{listed}</sheets></workbook>"#);
    let mut relationships: String = (1..=sheets.len())
        .map(|n| {
            rel(
                &format!("rId{n}"),
                WORKSHEET,
                &format!("worksheets/sheet{n}.xml"),
            )
        })
        .collect();
    relationships.push_str(related);
    let main = rel("rId1", OFFICE_DOCUMENT, "xl/workbook.xml");
    let mut items = vec![
        content_types(),
        rels("_rels/.rels", &main),
        Item::stored("xl/workbook.xml", xml.as_bytes()),
        rels("xl/_rels/workbook.xml.rels", &relationships),
    ];
    for ((_, sheet), n) in sheets.iter().zip(1..) {
        let name = format!("xl/worksheets/sheet{n}.xml");
        items.push(Item::stored(&name, sheet.as_bytes()));
    }
    items.extend(more);
    items
}

/// A Transitional `worksheet` whose `sheetData` holds `body`.
fn worksheet(body: &str) -> String {
    format!(r#"<worksheet xmlns="{NS}"><sheetData>{body}</sheetData></worksheet>"#)
}

/// A workbook of one sheet, `Sheet1`, whose XML is `sheet`, and a shared
/// strings table of `a` and `b`.
fn book_of(sheet: &str) -> Vec<Item> {
    let sst = format!(r#"<sst xmlns="{NS}"><si><t>a</t></si><si><t>b</t></si></sst>"#);
    let related = rel("rId9", SHARED_STRINGS, "sharedStrings.xml");
    let strings = Item::stored("xl/sharedStrings.xml", sst.as_bytes());
    workbook(&[("Sheet1", sheet)], false, &related, vec![strings])
}

/// Opens the book of `items` under `limits`, its reader failing in the
/// range `poisoned` holds.
fn open_with(
    items: Vec<Item>,
    limits: &ReadLimits,
    poisoned: Rc<Cell<(u64, u64)>>,
    diagnostics: &mut Diagnostics,
) -> Option<Book<Reader>> {
    let reader = Reader {
        inner: Cursor::new(zip::build(items)),
        poisoned,
    };
    let mut spools = MemorySpools;
    Book::open(reader, &Limits::default(), limits, &mut spools, diagnostics).unwrap()
}

/// Opens the book of `items`, which must open.
fn open(items: Vec<Item>, diagnostics: &mut Diagnostics) -> Book<Reader> {
    open_with(items, &ReadLimits::default(), Rc::default(), diagnostics).unwrap()
}

/// What a value is, in a word and its text: `s:` a string, `n:` a number,
/// `b:` a boolean, `date:`, `time:`, `dt:` and `el:` (milliseconds) a date
/// or time, `e:` an error value, and nothing for an empty cell.
fn show(value: Value<'_>) -> String {
    match value {
        Value::Empty => String::new(),
        Value::String(text) => format!("s:{text}"),
        Value::Number(number) => format!("n:{number}"),
        Value::Bool(bool) => format!("b:{bool}"),
        Value::Date(date) => format!("date:{date}"),
        Value::Time(time) => format!("time:{time}"),
        Value::DateTime(time) => format!("dt:{time}"),
        Value::Elapsed(elapsed) => format!("el:{}", elapsed.milliseconds()),
        Value::Error(text) => format!("e:{text}"),
    }
}

/// A row read: its number and its values, shown.
type Shown = (u32, Vec<String>);

/// `row` shown.
fn shown(row: &Row) -> Shown {
    (row.number(), row.values().map(show).collect())
}

/// Every row of the first sheet of `book`, shown, and what was found.
fn read_book(book: &Book<Reader>, mut diagnostics: Diagnostics) -> (Vec<Shown>, Seen) {
    let mut rows = book.rows(0).unwrap();
    let mut row = Row::new();
    let mut read = Vec::new();
    while rows.next_row(&mut row, &mut diagnostics).unwrap() {
        read.push(shown(&row));
    }
    assert!(!rows.next_row(&mut row, &mut diagnostics).unwrap());
    (read, seen(diagnostics))
}

/// Every row of the first sheet of the book of `items`, and what was
/// found.
fn read(items: Vec<Item>) -> (Vec<Shown>, Seen) {
    let mut diagnostics = Diagnostics::new();
    let book = open(items, &mut diagnostics);
    read_book(&book, diagnostics)
}

/// The rows of a sheet whose `sheetData` holds `body`, and what was found.
fn rows(body: &str) -> (Vec<Shown>, Seen) {
    read(book_of(&worksheet(body)))
}

/// The rows of a sheet whose `sheetData` holds `body`, which must read
/// without a diagnostic.
fn clean(body: &str) -> Vec<Shown> {
    let (rows, seen) = rows(body);
    assert_eq!(seen, vec![], "{body}");
    rows
}

/// `(number, values)`, as [`shown`] has a row.
fn row(number: u32, values: &[&str]) -> Shown {
    (
        number,
        values.iter().map(|&value| value.to_owned()).collect(),
    )
}

/// What a diagnostic is, without the offset it was found at:
/// `schema:{reason}:{name}` for `xml_schema_invalid`, else its code.
fn brief(diagnostic: &Diagnostic) -> String {
    match diagnostic {
        Diagnostic::XmlSchemaInvalid { reason, name, .. } => {
            format!("schema:{reason:?}:{}", name.as_str())
        }
        other => other.code().as_str().to_owned(),
    }
}

/// The diagnostics of `seen`, each [`brief`] and where it points.
fn briefs(seen: &Seen) -> Vec<(String, String)> {
    (seen.iter())
        .map(|(diagnostic, place)| (brief(diagnostic), place.clone()))
        .collect()
}

/// `(brief, the first sheet's part)`, as [`briefs`] lists one.
fn on_sheet(brief: &str) -> (String, String) {
    (brief.to_owned(), SHEET.to_owned())
}

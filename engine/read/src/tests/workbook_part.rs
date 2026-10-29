//! The workbook part: what it is read to, and the helpers the files under
//! `workbook_part/` share.

mod failures;
mod links;
mod reading;
mod schema;

use super::zip::Item;
use super::{NS, OFFICE_DOCUMENT, Seen, content_types, open, rel, rels, seen};
use crate::{ReadLimits, Workbook, WorkbookPart};
use xlsx_graft_diagnostics::Diagnostics;

/// The Transitional `worksheet` relationship type.
const WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
/// The Transitional relationships namespace, of `r:id`.
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// Where the workbook part is.
const PART: &str = "/xl/workbook.xml";

/// A package whose workbook part is `xml`, its relationships `related`,
/// then `more`.
fn book_with(xml: &str, related: &str, more: Vec<Item>) -> Vec<Item> {
    let main = rel("rId1", OFFICE_DOCUMENT, "xl/workbook.xml");
    let mut items = vec![
        content_types(),
        rels("_rels/.rels", &main),
        Item::stored("xl/workbook.xml", xml.as_bytes()),
        rels("xl/_rels/workbook.xml.rels", related),
    ];
    items.extend(more);
    items
}

/// The worksheet part `xl/worksheets/sheet{n}.xml`.
fn worksheet(n: u32) -> Item {
    Item::stored(&format!("xl/worksheets/sheet{n}.xml"), b"<worksheet/>")
}

/// A package whose Transitional `workbook` holds `body`, with `r` bound,
/// and whose relationships `rId1` and `rId2` are worksheets 1 and 2.
fn book(body: &str) -> Vec<Item> {
    let xml = format!(r#"<workbook xmlns="{NS}" xmlns:r="{R}">{body}</workbook>"#);
    let related = [1, 2]
        .map(|n| {
            rel(
                &format!("rId{n}"),
                WORKSHEET,
                &format!("worksheets/sheet{n}.xml"),
            )
        })
        .concat();
    book_with(&xml, &related, vec![worksheet(1), worksheet(2)])
}

/// Reads the workbook part of the package of `items` under `limits`, with
/// the index of each part by name.
fn read_with(items: Vec<Item>, limits: &ReadLimits) -> (Option<WorkbookPart>, Seen, Vec<String>) {
    let mut diagnostics = Diagnostics::new();
    let mut package = open(items, &mut diagnostics);
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let part = WorkbookPart::read(&mut package, &workbook, limits, &mut diagnostics).unwrap();
    let names = (package.parts().iter())
        .map(|part| part.name().as_str().to_owned())
        .collect();
    (part, seen(diagnostics), names)
}

/// [`read_with`] under the default limits, without the names.
fn read(items: Vec<Item>) -> (Option<WorkbookPart>, Seen) {
    let (part, seen, _) = read_with(items, &ReadLimits::default());
    (part, seen)
}

/// The workbook part of a `workbook` holding `body`, as [`book`] builds it,
/// which must read without a diagnostic.
fn clean(body: &str) -> WorkbookPart {
    let (part, seen) = read(book(body));
    assert_eq!(seen, vec![], "{body}");
    part.unwrap()
}

/// `sheets` holding a `sheet` of each of `sheets`: name and `r:id`, its
/// `sheetId` its place from 1.
fn sheets(sheets: &[(&str, &str)]) -> String {
    let listed: String = (sheets.iter().zip(1..))
        .map(|((name, id), n)| format!(r#"<sheet name="{name}" sheetId="{n}" r:id="{id}"/>"#))
        .collect();
    format!("<sheets>{listed}</sheets>")
}

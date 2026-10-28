//! The styles: what a workbook's styles part is read to, and the helpers
//! the files under `styles/` share.

mod failures;
mod limits;
mod reading;
mod schema;

use super::zip::Item;
use super::{NS, OFFICE_DOCUMENT, Seen, open, package_of, rel, seen};
use crate::{FormatKind, ReadLimits, Styles, Workbook};
use xlsx_graft_diagnostics::Diagnostics;

/// The Transitional `styles` relationship type.
const STYLES: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
/// Where the styles part is.
const PART: &str = "/xl/styles.xml";

/// A workbook whose styles are `part`, at `xl/styles.xml`.
fn styled_with(part: Item) -> Vec<Item> {
    let main = rel("rId1", OFFICE_DOCUMENT, "xl/workbook.xml");
    let related = rel("rId1", STYLES, "styles.xml");
    package_of(&main, &related, vec![part])
}

/// The styles part of `xml`.
fn part(xml: &str) -> Item {
    Item::stored("xl/styles.xml", xml.as_bytes())
}

/// A workbook whose styles hold `body` in a Transitional `styleSheet`.
fn styled(body: &str) -> Vec<Item> {
    styled_with(part(&format!(
        r#"<styleSheet xmlns="{NS}">{body}</styleSheet>"#
    )))
}

/// Reads the styles of the package of `items` under `limits`.
fn read_with(items: Vec<Item>, limits: &ReadLimits) -> (Styles, Seen) {
    let mut diagnostics = Diagnostics::new();
    let mut package = open(items, &mut diagnostics);
    let workbook = Workbook::find(&package, &mut diagnostics).unwrap();
    let styles = Styles::read(&mut package, &workbook, limits, &mut diagnostics).unwrap();
    (styles, seen(diagnostics))
}

/// [`read_with`] under the default limits.
fn read(items: Vec<Item>) -> (Styles, Seen) {
    read_with(items, &ReadLimits::default())
}

/// The kinds of the cell formats of styles of `body`, which must read
/// without a diagnostic.
fn kinds(body: &str) -> Vec<FormatKind> {
    let (styles, seen) = read(styled(body));
    assert_eq!(seen, vec![]);
    match styles {
        Styles::Formats(kinds) => kinds,
        other => panic!("{other:?}"),
    }
}

/// `cellXfs` holding an `xf` of each of `ids`.
fn cells(ids: &[u32]) -> String {
    let xfs: String = (ids.iter())
        .map(|id| format!(r#"<xf numFmtId="{id}"/>"#))
        .collect();
    format!("<cellXfs>{xfs}</cellXfs>")
}

/// `numFmts` defining each of `formats`, `id` and format code, the code
/// written as an attribute value already escaped.
fn numbers(formats: &[(u32, &str)]) -> String {
    let defined: String = (formats.iter())
        .map(|(id, code)| format!(r#"<numFmt numFmtId="{id}" formatCode="{code}"/>"#))
        .collect();
    format!("<numFmts>{defined}</numFmts>")
}

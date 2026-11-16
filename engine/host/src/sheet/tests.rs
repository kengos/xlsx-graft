//! Unit tests for the sheet view, driven through `describe` (and the three
//! entry points that call it) on workbooks the tests build, since Excel
//! does not write a sheet the reader stops at, or one with a million rows.
//! `shape` holds the JSON of a grid and its cells, `window` the range, the
//! rows and columns asked for and the merged ranges, `choose` the sheet
//! asked for, and `failures` what stops the reading.

mod choose;
mod failures;
mod shape;
mod window;

use super::{Grid, SheetRequest, describe};
use crate::policy::limits::HostLimits;
use crate::support::{json, main_rels, package, rel_of, rels};
use crate::view::View;
use serde_json::Value;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::ops::Range;
use xlsx_graft_read::CellRef;
use xlsx_graft_spool::{MemorySpools, Spool, SpoolFactory};

/// The Transitional SpreadsheetML namespace.
pub(super) const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
/// The Transitional relationships namespace, of `r:id`.
pub(super) const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// The Transitional relationship type `kind` (`worksheet`, `styles`, ...).
pub(super) fn rel_type(kind: &str) -> String {
    format!("http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}")
}

/// The reader every test reads a package from: bytes in memory whose reads
/// fail when they start in a range a test sets. One reader type for every
/// test that calls `describe`, since the coverage gate counts a line generic
/// over the reader as covered only when one instantiation covers it.
pub(super) struct Reader {
    pub(super) inner: Cursor<Vec<u8>>,
    /// Reads that start in this range fail, as the host would, and so does
    /// a seek to its start.
    pub(super) poisoned: Range<u64>,
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.poisoned.contains(&self.inner.position()) {
            return Err(io::Error::other("the disk went away"));
        }
        self.inner.read(buf)
    }
}

impl Seek for Reader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        if self.poisoned.start < self.poisoned.end && pos == SeekFrom::Start(self.poisoned.start) {
            return Err(io::Error::other("the disk went away"));
        }
        self.inner.seek(pos)
    }
}

/// The SpreadsheetML worksheet with `head` before its `sheetData` (a
/// `dimension`), `rows` in it, and `tail` after it (`mergeCells`).
pub(super) fn worksheet(head: &str, rows: &str, tail: &str) -> String {
    format!(r#"<worksheet xmlns="{NS}">{head}<sheetData>{rows}</sheetData>{tail}</worksheet>"#)
}

/// A worksheet of `rows` alone.
pub(super) fn rows_only(rows: &str) -> String {
    worksheet("", rows, "")
}

/// The package of a workbook whose sheets are `tags` (`sheet` elements,
/// whose `r:id` the workbook's relationships name), with `related` among the
/// workbook's relationships, and the parts `parts` beside the shared strings
/// table (`shared`, `a|b`, `line` LF `break`), the styles (cell format 1 a
/// date, 2 a time, 3 a date-time and 4 elapsed time) and the content types.
pub(super) fn workbook(tags: &str, related: &str, parts: &[(&str, &str)]) -> Vec<u8> {
    let workbook =
        format!(r#"<workbook xmlns="{NS}" xmlns:r="{R}"><sheets>{tags}</sheets></workbook>"#);
    let strings = format!(
        r#"<sst xmlns="{NS}"><si><t>shared</t></si><si><t>a|b</t></si><si><t>line
break</t></si></sst>"#
    );
    let xfs: String = [0, 14, 20, 22, 46]
        .map(|id| format!(r#"<xf numFmtId="{id}"/>"#))
        .concat();
    let styles = format!(r#"<styleSheet xmlns="{NS}"><cellXfs>{xfs}</cellXfs></styleSheet>"#);
    let relationships = format!(
        "{}{}{related}",
        rel_of("rId90", &rel_type("sharedStrings"), "sharedStrings.xml"),
        rel_of("rId91", &rel_type("styles"), "styles.xml"),
    );
    let main = main_rels();
    let workbook_rels = rels(&relationships);
    let mut entries = vec![
        ("_rels/.rels", main.as_str()),
        ("xl/workbook.xml", workbook.as_str()),
        ("xl/_rels/workbook.xml.rels", workbook_rels.as_str()),
        ("xl/sharedStrings.xml", strings.as_str()),
        ("xl/styles.xml", styles.as_str()),
    ];
    entries.extend(parts);
    package(&entries)
}

/// The package of a workbook of worksheets, one for each pair, which is the
/// sheet's name and the XML of its worksheet part, in the order given.
pub(super) fn book(sheets: &[(&str, &str)]) -> Vec<u8> {
    let mut tags = String::new();
    let mut related = String::new();
    let mut paths = Vec::new();
    for ((name, _), n) in sheets.iter().zip(1..) {
        tags.push_str(&format!(
            r#"<sheet name="{name}" sheetId="{n}" r:id="rId{n}"/>"#
        ));
        let target = format!("worksheets/sheet{n}.xml");
        related.push_str(&rel_of(&format!("rId{n}"), &rel_type("worksheet"), &target));
        paths.push(format!("xl/{target}"));
    }
    let parts: Vec<(&str, &str)> = (paths.iter().zip(sheets))
        .map(|(path, (_, xml))| (path.as_str(), *xml))
        .collect();
    workbook(&tags, &related, &parts)
}

/// A workbook of one worksheet, `Sheet1`, holding `xml`.
pub(super) fn single(xml: &str) -> Vec<u8> {
    book(&[("Sheet1", xml)])
}

/// A request for the first sheet, `range` of it, `rows` rows and `columns`
/// columns at most.
pub(super) fn asking(range: Option<&str>, rows: Option<u64>, columns: Option<u64>) -> SheetRequest {
    SheetRequest::new(None, range, rows, columns).unwrap()
}

/// The view of the workbook `bytes` under `limits`, `spools` making the
/// spools of its shared strings.
pub(super) fn view_with(
    bytes: &[u8],
    request: &SheetRequest,
    limits: &HostLimits,
    spools: &mut dyn SpoolFactory,
) -> View<Grid> {
    let reader = Reader {
        inner: Cursor::new(bytes.to_vec()),
        poisoned: 0..0,
    };
    describe(reader, request, limits, spools)
}

/// The view of the workbook `bytes` under the default limits.
pub(super) fn view(bytes: &[u8], request: &SheetRequest) -> View<Grid> {
    view_with(bytes, request, &HostLimits::default(), &mut MemorySpools)
}

/// The JSON of the view of the workbook `bytes` under the default limits.
pub(super) fn described(bytes: &[u8], request: &SheetRequest) -> Value {
    json(&view(bytes, request).to_json().unwrap())
}

/// The code of each diagnostic of a view's JSON.
pub(super) fn codes(value: &Value) -> Vec<&str> {
    let listed = value["diagnostics"].as_array().unwrap();
    listed.iter().map(|d| d["code"].as_str().unwrap()).collect()
}

/// A factory whose spools take what is written and fail every read.
pub(super) struct Unreadable;

/// A spool that fails every read.
struct Unread(Vec<u8>);

impl Spool for Unread {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.0.extend(bytes);
        Ok(())
    }

    fn read_exact_at(&mut self, _: u64, _: &mut [u8]) -> io::Result<()> {
        Err(io::Error::other("the spool went away"))
    }
}

impl SpoolFactory for Unreadable {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        Ok(Box::new(Unread(Vec::new())))
    }
}

/// The cell at column `column` and row `row`, as the sheet's XML writes
/// it, holding the number `row * 1000 + column`.
pub(super) fn number_cell(column: u16, row: u32) -> String {
    let at = CellRef::new(column, row).unwrap();
    let value = u64::from(row) * 1_000 + u64::from(column);
    format!(r#"<c r="{at}"><v>{value}</v></c>"#)
}

/// The XML of rows `1..=rows`, each with a number in columns
/// `1..=columns`.
pub(super) fn filled(rows: u32, columns: u16) -> String {
    (1..=rows)
        .map(|row| {
            let cells: String = (1..=columns)
                .map(|column| number_cell(column, row))
                .collect();
            format!(r#"<row r="{row}">{cells}</row>"#)
        })
        .collect()
}

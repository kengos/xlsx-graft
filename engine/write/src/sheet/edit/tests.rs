//! Tests for `SheetEdit`, on workbooks built in memory, opened with
//! `SheetEdit::open`, changed through its calls and written by a
//! `PackageEdit`: `open` covers what is read of the part and refused,
//! `merges`, `columns`, `rows`, `panes` and `protection` what each concern
//! writes (exact XML), `order` the order of the root's children, the prefix
//! of the root and what is written as it was, `refuse` a part whose edited element is out of order or wrapped, `wrapped` the look-ahead into
//! an `mc:AlternateContent`, and `chain` the use after `StructureEdit`.

mod chain;
mod columns;
mod edges;
mod merges;
mod open;
mod order;
mod panes;
mod protection;
mod refuse;
mod rows;
mod wrapped;

use super::SheetEdit;
use crate::sheet::CellRange;
use crate::tests::{Sink, stored};
use crate::{PackageEdit, WriteLimits};
use std::io::Cursor;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package, PartName};
use xlsx_graft_spool::MemorySpools;

pub(super) const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub(super) const STRICT_NS: &str = "http://purl.oclc.org/ooxml/spreadsheetml/main";
pub(super) const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const TYPES: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
pub(super) const TABLE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";
const WORKSHEET: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
const WORKBOOK: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";

/// The part the tests edit: the first sheet.
pub(super) const SHEET: &str = "/xl/worksheets/sheet1.xml";

/// A worksheet of the main namespace around `body`.
pub(super) fn sheet(body: &str) -> String {
    format!(r#"<worksheet xmlns="{NS}">{body}</worksheet>"#)
}

/// The range of an A1 range.
pub(super) fn r(text: &str) -> CellRange {
    CellRange::parse(text).unwrap()
}

/// The name of the part `name`.
pub(super) fn part(name: &str) -> PartName {
    PartName::parse(name).unwrap()
}

/// The zip of a workbook of the sheets `Alpha`, `Beta` and `Gamma` (as many
/// as `sheets` has contents, in `xl/worksheets/sheet<n>.xml`), and the parts
/// `extra` (name, media type, content: no override for an empty media type).
pub(super) fn zip(sheets: &[&str], extra: &[(&str, &str, &str)]) -> Vec<u8> {
    zip_with("", sheets, extra)
}

/// As [`zip`], with `before` (such as `bookViews`) before the `sheets`
/// element of the workbook part.
pub(super) fn zip_with(before: &str, sheets: &[&str], extra: &[(&str, &str, &str)]) -> Vec<u8> {
    let names = ["Alpha", "Beta", "Gamma"];
    let list: String = (names.iter().zip(1..=sheets.len()))
        .map(|(name, n)| format!(r#"<sheet name="{name}" sheetId="{n}" r:id="rId{n}"/>"#))
        .collect();
    let workbook = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}">{before}<sheets>{list}</sheets></workbook>"#
    );
    let rels: String = (1..=sheets.len())
        .map(|n| {
            format!(
                r#"<Relationship Id="rId{n}" Type="{R}/worksheet" Target="worksheets/sheet{n}.xml"/>"#
            )
        })
        .collect();
    let mut entries: Vec<(String, String, String)> = vec![
        ("xl/workbook.xml".into(), WORKBOOK.into(), workbook),
        (
            "xl/_rels/workbook.xml.rels".into(),
            String::new(),
            format!(r#"<Relationships xmlns="{REL}">{rels}</Relationships>"#),
        ),
    ];
    for (n, content) in (1..).zip(sheets) {
        let name = format!("xl/worksheets/sheet{n}.xml");
        entries.push((name, WORKSHEET.into(), (*content).into()));
    }
    entries.extend(
        (extra.iter())
            .map(|(name, kind, content)| ((*name).into(), (*kind).into(), (*content).into())),
    );
    let overrides: String = (entries.iter())
        .filter(|(_, kind, _)| !kind.is_empty())
        .map(|(name, kind, _)| format!(r#"<Override PartName="/{name}" ContentType="{kind}"/>"#))
        .collect();
    let types = format!(
        r#"<Types xmlns="{TYPES}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>{overrides}</Types>"#
    );
    let root = format!(
        r#"<Relationships xmlns="{REL}"><Relationship Id="rId1" Type="{R}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
    );
    let mut all = vec![
        ("[Content_Types].xml".to_owned(), types),
        ("_rels/.rels".to_owned(), root),
    ];
    all.extend(
        entries
            .into_iter()
            .map(|(name, _, content)| (name, content)),
    );
    let pairs: Vec<(&[u8], &[u8])> = (all.iter())
        .map(|(name, content)| (name.as_bytes(), content.as_bytes()))
        .collect();
    stored(&pairs)
}

/// The package of the zip `bytes`, read without a diagnostic.
pub(super) fn package(bytes: Vec<u8>) -> Package<Cursor<Vec<u8>>> {
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(Cursor::new(bytes), &Limits::default(), &mut diagnostics);
    assert!(diagnostics.records().is_empty());
    package.unwrap().unwrap()
}

/// The edit of the sheet `xml`, opened without a diagnostic.
pub(super) fn open(xml: &str) -> SheetEdit {
    let mut package = package(zip(&[xml], &[]));
    let mut diagnostics = Diagnostics::new();
    let edit = SheetEdit::open(
        &mut package,
        &part(SHEET),
        &WriteLimits::default(),
        &mut diagnostics,
    );
    assert!(diagnostics.records().is_empty());
    edit.unwrap().unwrap()
}

/// The package written by `PackageEdit::finish`, as bytes.
pub(super) struct Output {
    bytes: Vec<u8>,
}

impl Output {
    /// The text of the part `name`.
    pub(super) fn part(&self, name: &str) -> String {
        let mut diagnostics = Diagnostics::new();
        let package = Package::open(
            Cursor::new(self.bytes.clone()),
            &Limits::default(),
            &mut diagnostics,
        );
        let mut package = package.unwrap().unwrap();
        let index = package.find(&part(name));
        let xlsx_graft_package::Presence::Part(index) = index else {
            panic!("no part {name}");
        };
        let limit = xlsx_graft_diagnostics::Limit::default_of(1 << 28);
        let bytes = package.read_xml_part(index, limit, &mut diagnostics);
        String::from_utf8(bytes.unwrap().unwrap()).unwrap()
    }

    /// The text of the part the tests edit.
    pub(super) fn sheet(&self) -> String {
        self.part(SHEET)
    }

    /// Whether the zip holds `text` as it is: a part copied from a package
    /// whose entries are stored is, a part written again is deflated.
    pub(super) fn holds(&self, text: &str) -> bool {
        let needle = text.as_bytes();
        (self.bytes.windows(needle.len())).any(|window| window == needle)
    }
}

/// Writes `edit`, which must give a zip.
pub(super) fn finish(edit: PackageEdit<'_, Cursor<Vec<u8>>>) -> Output {
    let mut diagnostics = Diagnostics::new();
    let bytes = edit
        .finish(
            Sink::default(),
            &mut MemorySpools,
            &WriteLimits::default(),
            &mut diagnostics,
        )
        .unwrap();
    assert!(diagnostics.records().is_empty());
    Output {
        bytes: bytes.unwrap().bytes,
    }
}

/// Opens the sheet `xml`, lets `act` call the edit, applies it and writes
/// the package.
pub(super) fn run(xml: &str, act: impl FnOnce(&mut SheetEdit)) -> Output {
    run_in(zip(&[xml], &[]), act)
}

/// As [`run`], on the package `bytes`.
pub(super) fn run_in(bytes: Vec<u8>, act: impl FnOnce(&mut SheetEdit)) -> Output {
    let mut package = package(bytes);
    let mut diagnostics = Diagnostics::new();
    let mut sheet = SheetEdit::open(
        &mut package,
        &part(SHEET),
        &WriteLimits::default(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    act(&mut sheet);
    let mut edit = PackageEdit::new(package);
    sheet.apply(&mut edit).unwrap();
    finish(edit)
}

/// The text of the sheet `xml` after `act`.
pub(super) fn text(xml: &str, act: impl FnOnce(&mut SheetEdit)) -> String {
    run(xml, act).sheet()
}

/// Opens the sheet `xml`, lets `act` call the edit, applies it and writes
/// the package, which must refuse the part: no zip, and one diagnostic,
/// given as `code message part`.
pub(super) fn refusal(xml: &str, act: impl FnOnce(&mut SheetEdit)) -> String {
    let mut package = package(zip(&[xml], &[]));
    let limits = WriteLimits::default();
    let mut diagnostics = Diagnostics::new();
    let mut sheet = SheetEdit::open(&mut package, &part(SHEET), &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    act(&mut sheet);
    let mut edit = PackageEdit::new(package);
    sheet.apply(&mut edit).unwrap();
    let written = edit
        .finish(
            Sink::default(),
            &mut MemorySpools,
            &limits,
            &mut diagnostics,
        )
        .unwrap();
    assert!(written.is_none());
    let records = diagnostics.records();
    assert_eq!(records.len(), 1, "{} diagnostics", records.len());
    let at = records[0]
        .location
        .part
        .as_ref()
        .map(|echo| echo.as_str().to_owned());
    let diagnostic = &records[0].diagnostic;
    format!(
        "{} {} {}",
        diagnostic.code(),
        diagnostic.message(),
        at.unwrap_or_default()
    )
}

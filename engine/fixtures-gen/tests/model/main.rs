//! The workbook model of `xlsx_graft_write::structure::model` reads the
//! sheets of a workbook as `xlsx-graft-read` does: on every workbook of
//! `fixtures/` the sheets (name, `sheetId`, state, part and kind) are the
//! reader's, and on workbooks whose sheets cannot be tied to a part (two
//! sheets whose names differ in letter case only, two on one part, a sheet
//! whose relationship is missing, or of a type that is no sheet's) both push
//! the same diagnostics, each with its code and where it points.
//!
//! The test is here and not in `xlsx-graft-write` because the writer is
//! published before the reader (`scripts/release/crates.txt`), so it cannot
//! depend on it, and the model repeats the reader's rules instead of using
//! them: this test is what keeps the two equal.
//!
//! `fixtures/` is outside the crate: the test reads the workbooks from disk
//! when it runs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_read::{ReadLimits, Workbook, WorkbookPart};
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_write::structure::model;
use xlsx_graft_write::{DeflateWriter, WriteLimits, ZipWriter};

/// The repository's `fixtures/` folder.
fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// The paths of the workbooks in `fixtures/` (`.xlsx` and `.xlsm`), sorted.
fn workbooks() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(folder())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("xlsx" | "xlsm")
            )
        })
        .collect();
    paths.sort();
    paths
}

/// What each reading says of the sheets: name, `sheetId`, state, part
/// index and kind, the enums by their names, which are the same in both
/// crates; and the diagnostics each pushed, as the diagnostic and where it
/// points.
type Sheets = Vec<(String, u32, String, usize, String)>;
type Pushed = Vec<(Diagnostic, Location)>;

/// The diagnostics `diagnostics` holds, as what they say and where.
fn pushed(diagnostics: &Diagnostics) -> Pushed {
    (diagnostics.records().iter())
        .map(|record| (record.diagnostic.clone(), record.location.clone()))
        .collect()
}

/// The sheets of `package` as the reader gives them, when it gives a part
/// for each, and what it pushed.
fn by_read<R: Read + Seek>(package: &mut Package<R>) -> (Option<Sheets>, Pushed) {
    let mut diagnostics = Diagnostics::new();
    let found = Workbook::find(package, &mut diagnostics);
    let read = found.and_then(|workbook| {
        let limits = ReadLimits::default();
        WorkbookPart::read(package, &workbook, &limits, &mut diagnostics).unwrap()
    });
    let sheets = read.and_then(|read| {
        (read.sheets().iter())
            .map(|sheet| {
                let part = sheet.part()?;
                let state = format!("{:?}", sheet.state());
                let kind = format!("{:?}", part.kind());
                Some((
                    sheet.name().to_owned(),
                    sheet.sheet_id(),
                    state,
                    part.index(),
                    kind,
                ))
            })
            .collect()
    });
    (sheets, pushed(&diagnostics))
}

/// The sheets of `package` as the model gives them, and what it pushed.
fn by_model<R: Read + Seek>(package: &mut Package<R>) -> (Option<Sheets>, Pushed) {
    let mut diagnostics = Diagnostics::new();
    let workbook = model::Workbook::read(package, &WriteLimits::default(), &mut diagnostics);
    let sheets = workbook.unwrap().map(|workbook| {
        (workbook.sheets().iter())
            .map(|sheet| {
                let state = format!("{:?}", sheet.state());
                let kind = format!("{:?}", sheet.kind());
                (
                    sheet.name().to_owned(),
                    sheet.sheet_id(),
                    state,
                    sheet.part(),
                    kind,
                )
            })
            .collect()
    });
    (sheets, pushed(&diagnostics))
}

#[test]
fn every_workbook_of_fixtures_has_the_sheets_the_reader_reads() {
    let paths = workbooks();
    assert!(!paths.is_empty());
    for path in paths {
        let mut package = {
            let file = BufReader::new(File::open(&path).unwrap());
            let mut diagnostics = Diagnostics::new();
            Package::open(file, &Limits::default(), &mut diagnostics)
                .unwrap()
                .unwrap()
        };
        let (read, read_pushed) = by_read(&mut package);
        let (model, model_pushed) = by_model(&mut package);
        let name = path.display();
        assert_eq!(read_pushed, [], "{name}: the reader pushed");
        assert_eq!(model_pushed, [], "{name}: the model pushed");
        assert!(!read.as_ref().unwrap().is_empty(), "{name}");
        assert_eq!(model, read, "{name}");
    }
}

/// A zip of `entries`, each deflated, written by the engine's own writer.
fn zipped(entries: &[(String, String)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Vec::new());
    for (name, content) in entries {
        let mut spools = MemorySpools;
        let mut deflate = DeflateWriter::new(&mut spools, 1 << 20);
        deflate.write_all(content.as_bytes()).unwrap();
        zip.write_deflated(name.as_bytes(), deflate.finish().unwrap())
            .unwrap();
    }
    zip.finish().unwrap()
}

/// The namespace of the relationship types of a Transitional workbook.
const NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// A Relationships part holding `body`.
fn wrap(body: &str) -> String {
    format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{body}</Relationships>"#
    )
}

/// The package relationships of a package with one workbook part.
fn office_document() -> String {
    wrap(&format!(
        r#"<Relationship Id="rId1" Type="{NS}/officeDocument" Target="xl/workbook.xml"/>"#
    ))
}

/// A package whose package relationships part is `root`, whose workbook part
/// has `sheets` (the content of its `sheets` element) and whose Relationships
/// part is `rels`, with the parts `sheet1.xml`, `sheet2.xml` and `other.xml`.
fn crafted(root: &str, sheets: &str, rels: &str) -> Package<Cursor<Vec<u8>>> {
    let types = concat!(
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        r#"<Default Extension="xml" ContentType="application/xml"/>"#,
        r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
        r#"<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
        r#"<Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
        r#"</Types>"#,
    );
    let workbook = format!(
        r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{NS}"><sheets>{sheets}</sheets></workbook>"#
    );
    let entries: Vec<(String, String)> = [
        ("[Content_Types].xml", types.to_owned()),
        ("_rels/.rels", root.to_owned()),
        ("xl/workbook.xml", workbook),
        ("xl/_rels/workbook.xml.rels", rels.to_owned()),
        ("xl/worksheets/sheet1.xml", "<worksheet/>".to_owned()),
        ("xl/worksheets/sheet2.xml", "<worksheet/>".to_owned()),
        ("xl/other.xml", "<other/>".to_owned()),
    ]
    .into_iter()
    .map(|(name, content)| (name.to_owned(), content))
    .collect();
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(
        Cursor::new(zipped(&entries)),
        &Limits::default(),
        &mut diagnostics,
    );
    package.unwrap().unwrap()
}

#[test]
fn a_workbook_whose_sheets_cannot_be_tied_to_parts_gets_the_readers_diagnostics() {
    let ns = NS;
    let both = format!(
        r#"<Relationship Id="rId1" Type="{ns}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{ns}/worksheet" Target="worksheets/sheet2.xml"/>"#
    );
    let sheet = |name: &str, id: u32, rid: &str| {
        format!(r#"<sheet name="{name}" sheetId="{id}" r:id="{rid}"/>"#)
    };
    let styles = format!(
        r#"<Relationship Id="rId1" Type="{ns}/styles" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{ns}/worksheet" Target="worksheets/sheet2.xml"/>"#
    );
    let external = format!(
        r#"<Relationship Id="rId1" Type="{ns}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{ns}/worksheet" Target="https://example.com/s.xml" TargetMode="External"/>"#
    );
    let partial = format!(
        r#"<Relationship Id="rId1" Type="{ns}/styles" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{ns}/worksheet" Target="worksheets/none.xml"/>"#
    );
    let (both, styles, external, partial) =
        (wrap(&both), wrap(&styles), wrap(&external), wrap(&partial));
    let cases = [
        // The relationships were not all read, and two names are equal,
        // letter case aside.
        (
            format!("{}{}", sheet("Data", 1, "rId1"), sheet("DATA", 2, "rId2")),
            "<Relationships".to_owned(),
        ),
        // The relationships were not all read, one is of a type that is no
        // sheet's, and the other, which the package left out, is named by a
        // sheet.
        (
            format!("{}{}", sheet("A", 1, "rId1"), sheet("B", 2, "rId2")),
            partial,
        ),
        // Names equal, letter case aside.
        (
            format!("{}{}", sheet("Data", 1, "rId1"), sheet("DATA", 2, "rId2")),
            both.clone(),
        ),
        // Two sheets on one part.
        (
            format!("{}{}", sheet("A", 1, "rId1"), sheet("B", 2, "rId1")),
            both.clone(),
        ),
        // A relationship that is not there.
        (
            format!("{}{}", sheet("A", 1, "rId1"), sheet("B", 2, "rId9")),
            both.clone(),
        ),
        // A relationship of a type that is no sheet's.
        (
            format!("{}{}", sheet("A", 1, "rId1"), sheet("B", 2, "rId2")),
            styles,
        ),
        // A relationship whose target is outside the package.
        (
            format!("{}{}", sheet("A", 1, "rId1"), sheet("B", 2, "rId2")),
            external,
        ),
        // Several at once.
        (
            format!(
                "{}{}{}",
                sheet("A", 1, "rId1"),
                sheet("a", 2, "rId1"),
                sheet("C", 3, "rId9")
            ),
            both,
        ),
    ];
    for (sheets, rels) in cases {
        let mut package = crafted(&office_document(), &sheets, &rels);
        let (_, read_pushed) = by_read(&mut package);
        let (model, model_pushed) = by_model(&mut package);
        assert!(!read_pushed.is_empty(), "{sheets}");
        assert_eq!(model_pushed, read_pushed, "{sheets}");
        // When the part of a sheet cannot be decided, the model is `None`.
        assert!(model.is_none(), "{sheets}");
    }
}

#[test]
fn a_crafted_workbook_without_a_problem_is_read_as_the_reader_reads_it() {
    let rels = wrap(&format!(
        r#"<Relationship Id="rId1" Type="{NS}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{NS}/chartsheet" Target="worksheets/sheet2.xml"/>"#
    ));
    let sheets = r#"<sheet name="A" sheetId="3" state="hidden" r:id="rId2"/><sheet name="B" sheetId="4" r:id="rId1"/>"#;
    let mut package = crafted(&office_document(), sheets, &rels);
    let (read, read_pushed) = by_read(&mut package);
    let (model, model_pushed) = by_model(&mut package);
    assert_eq!((read_pushed, model_pushed), (vec![], vec![]));
    assert_eq!(model.as_ref().unwrap().len(), 2);
    assert_eq!(model, read);
}

#[test]
fn the_workbook_part_is_found_as_the_reader_finds_it() {
    let rels = wrap(&format!(
        r#"<Relationship Id="rId1" Type="{NS}/worksheet" Target="worksheets/sheet1.xml"/>"#
    ));
    let sheets = r#"<sheet name="A" sheetId="1" r:id="rId1"/>"#;
    let office = format!("{NS}/officeDocument");
    let roots = [
        // Two `officeDocument` relationships that target different parts.
        (
            wrap(&format!(
                r#"<Relationship Id="rId1" Type="{office}" Target="xl/workbook.xml"/><Relationship Id="rId2" Type="{office}" Target="xl/other.xml"/>"#
            )),
            false,
        ),
        // An external `officeDocument` relationship, listed first, then an
        // internal one.
        (
            wrap(&format!(
                r#"<Relationship Id="rId1" Type="{office}" Target="https://example.com/" TargetMode="External"/><Relationship Id="rId2" Type="{office}" Target="xl/workbook.xml"/>"#
            )),
            true,
        ),
    ];
    for (root, found) in roots {
        let mut package = crafted(&root, sheets, &rels);
        let (read, read_pushed) = by_read(&mut package);
        let (model, model_pushed) = by_model(&mut package);
        assert_eq!(model_pushed, read_pushed, "{root}");
        assert_eq!(read_pushed.is_empty(), found, "{root}");
        assert_eq!(model.is_some(), found, "{root}");
        assert_eq!(model, read, "{root}");
    }
}

//! The number formats this crate writes, read back by `xlsx-graft-read`: a
//! workbook whose styles part holds the date and time formats as `numFmt`
//! elements, and the built-in formats by id, is read to the kinds of format
//! the formats are meant to have.

use crate::canonical::write::write_number_format;
use crate::number_format::builtin_id;
use crate::{DATE, DATE_TIME, ELAPSED, TIME};
use std::io::{Cursor, Write};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_read::{FormatKind, ReadLimits, Styles, Workbook};
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_write::{DeflateWriter, ZipWriter};

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const TYPES: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// A workbook with no sheet whose styles part is `styles`, written as a zip.
fn workbook(styles: &str) -> Vec<u8> {
    let content_types = concat!(
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        r#"<Default Extension="xml" ContentType="application/xml"/></Types>"#,
    );
    let main = format!(
        r#"<Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{TYPES}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
    );
    let related = format!(
        r#"<Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{TYPES}/styles" Target="styles.xml"/></Relationships>"#
    );
    let book = format!(r#"<workbook xmlns="{NS}"><sheets/></workbook>"#);
    let parts = [
        ("[Content_Types].xml", content_types),
        ("_rels/.rels", main.as_str()),
        ("xl/workbook.xml", book.as_str()),
        ("xl/_rels/workbook.xml.rels", related.as_str()),
        ("xl/styles.xml", styles),
    ];
    let mut spools = MemorySpools;
    let mut zip = ZipWriter::new(Vec::new());
    for (name, xml) in parts {
        let mut content = DeflateWriter::new(&mut spools, 1 << 20);
        content.write_all(xml.as_bytes()).unwrap();
        zip.write_deflated(name.as_bytes(), content.finish().unwrap())
            .unwrap();
    }
    zip.finish().unwrap()
}

/// The kind of each cell format of `styles`.
fn kinds(styles: &str) -> Vec<FormatKind> {
    let mut diagnostics = Diagnostics::new();
    let reader = Cursor::new(workbook(styles));
    let mut package = Package::open(reader, &Limits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    let book = Workbook::find(&package, &mut diagnostics).unwrap();
    let limits = ReadLimits::default();
    match Styles::read(&mut package, &book, &limits, &mut diagnostics).unwrap() {
        Styles::Formats(kinds) => kinds,
        other => panic!("{other:?}"),
    }
}

/// A styles part in the namespace of its prefix, whose `numFmts` define
/// the formats 164 on as `codes`, and whose `cellXfs` hold a cell format for
/// each id of `ids`.
fn styles(prefix: &str, codes: &[&str], ids: &[u32]) -> String {
    let mut formats = Vec::new();
    for (id, code) in (164..).zip(codes) {
        write_number_format(prefix.as_bytes(), id, code, &mut formats);
    }
    let formats = String::from_utf8(formats).unwrap();
    let open = if prefix.is_empty() {
        String::new()
    } else {
        format!("{prefix}:")
    };
    let xfs: String = (ids.iter())
        .map(|id| format!(r#"<{open}xf numFmtId="{id}"/>"#))
        .collect();
    let xmlns = if prefix.is_empty() {
        format!(r#"xmlns="{NS}""#)
    } else {
        format!(r#"xmlns:{prefix}="{NS}""#)
    };
    let count = codes.len();
    format!(
        "<{open}styleSheet {xmlns}><{open}numFmts count=\"{count}\">{formats}</{open}numFmts>\
         <{open}cellXfs>{xfs}</{open}cellXfs></{open}styleSheet>"
    )
}

#[test]
fn the_date_and_time_codes_are_read_as_dates_and_times() {
    let codes = [DATE, TIME, DATE_TIME, ELAPSED];
    let expected = [
        FormatKind::Date,
        FormatKind::Time,
        FormatKind::DateTime,
        FormatKind::Duration,
    ];
    assert_eq!(kinds(&styles("", &codes, &[164, 165, 166, 167])), expected);
    assert_eq!(kinds(&styles("s", &codes, &[164, 165, 166, 167])), expected);
}

#[test]
fn the_built_in_ids_are_read_as_numbers_text_and_general() {
    let codes = [
        ("General", FormatKind::General),
        ("0", FormatKind::Number),
        ("0.00", FormatKind::Number),
        ("#,##0", FormatKind::Number),
        ("#,##0.00", FormatKind::Number),
        ("0%", FormatKind::Number),
        ("0.00%", FormatKind::Number),
        ("0.00E+00", FormatKind::Number),
        ("# ?/?", FormatKind::Number),
        ("# ??/??", FormatKind::Number),
        ("@", FormatKind::Text),
    ];
    let ids: Vec<u32> = codes
        .iter()
        .map(|(code, _)| builtin_id(code).unwrap())
        .collect();
    let expected: Vec<FormatKind> = codes.iter().map(|(_, kind)| *kind).collect();
    assert_eq!(kinds(&styles("", &[], &ids)), expected);
}

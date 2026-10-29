//! What a sheet's rows read to: each type of cell, the gaps between cells,
//! rows and cells without `r`, numbers made dates by their formats, the
//! row buffer, finding a sheet, and readers kept beside the book.

use super::super::zip::Item;
use super::super::{NS, rel};
use super::{
    R, STYLES, Shown, book_of, clean, open, read, read_book, row, shown, workbook, worksheet,
};
use crate::{DateSystem, Row, SheetLookup, Value};
use xlsx_graft_diagnostics::Diagnostics;

#[test]
fn every_type_of_cell_reads_to_its_value() {
    let body = concat!(
        r#"<row r="1">"#,
        r#"<c r="A1" t="n"><v>1.5</v></c>"#,
        r#"<c r="B1" t="s"><v> 1 </v></c>"#,
        r#"<c r="C1" t="str"><f>"x"</f><v>x</v></c>"#,
        r#"<c r="D1" t="inlineStr"><is><r><t>a</t></r><r><rPr/><t xml:space="preserve"> b</t></r>"#,
        r#"<rPh sb="0" eb="1"><t>ア</t></rPh><phoneticPr fontId="1"/></is></c>"#,
        r#"<c r="E1" t="b"><v>1</v></c>"#,
        r#"<c r="F1" t="b"><v>false</v></c>"#,
        r#"<c r="G1" t="e"><v>#N/A</v></c>"#,
        r#"<c r="H1" t="d"><v>2024-01-02</v></c>"#,
        r#"<c r="I1" t="d"><v>2024-01-02T03:04:05</v></c>"#,
        r#"<c r="J1" t="inlineStr"><is><t>plain</t></is></c>"#,
        "</row>",
    );
    assert_eq!(
        clean(body),
        vec![row(
            1,
            &[
                "n:1.5",
                "s:b",
                "s:x",
                "s:a b",
                "b:true",
                "b:false",
                "e:#N/A",
                "date:2024-01-02",
                "dt:2024-01-02T03:04:05.000",
                "s:plain",
            ]
        )]
    );
}

#[test]
fn a_cell_without_its_value_is_empty() {
    let body = concat!(
        r#"<row r="1">"#,
        "<c/>",
        r#"<c t="s"/>"#,
        r#"<c t="str"><f>A1</f></c>"#,
        r#"<c t="inlineStr"/>"#,
        r#"<c t="e"><extLst/></c>"#,
        "</row>",
    );
    assert_eq!(clean(body), vec![row(1, &["", "", "", "", ""])]);
}

#[test]
fn a_value_of_the_other_kind_is_skipped_unread() {
    // An inline string's `v`, and another cell's `is`, are not its value.
    let body = concat!(
        r#"<row><c t="inlineStr"><v>no</v><is><t>yes</t></is></c>"#,
        r#"<c t="str"><v>yes</v><is><t>no</t><x/></is></c></row>"#,
    );
    assert_eq!(clean(body), vec![row(1, &["s:yes", "s:yes"])]);
}

#[test]
fn text_is_read_whole_its_escapes_undone() {
    let body = concat!(
        r#"<row><c t="str"><v>a_x0042_&amp;<![CDATA[<c>]]>&#100;_x005F_x0041_</v></c>"#,
        r#"<c t="inlineStr"><is><t>_x00</t><r><t>41_</t></r></is></c>"#,
        r#"<c t="e"><v>#DIV/0!</v></c><c t="str"><v></v></c></row>"#,
    );
    // An escape does not run across two `t`.
    assert_eq!(
        clean(body),
        vec![row(
            1,
            &["s:aB&<c>d_x0041_", "s:_x0041_", "e:#DIV/0!", "s:"]
        )]
    );
}

#[test]
fn only_the_rows_written_come_each_with_its_number_and_gaps_are_empty() {
    let body = concat!(
        r#"<row r="2"><c r="C2"><v>1</v></c><c><v>2</v></c><c r="F2"><v>3</v></c></row>"#,
        r#"<row><c/><c r="c3"><v>4</v></c></row>"#,
        r#"<row r="10" spans="1:3" ht="20" customHeight="1"/>"#,
        r#"<row r="1048576"><c r="XFD1048576"><v>5</v></c></row>"#,
    );
    let mut last = vec![""; 16_383];
    last.push("n:5");
    assert_eq!(
        clean(body),
        vec![
            row(2, &["", "", "n:1", "n:2", "", "n:3"]),
            row(3, &["", "", "n:4"]),
            row(10, &[]),
            row(1_048_576, &last),
        ]
    );
}

#[test]
fn a_sheet_without_rows_reads_none() {
    assert_eq!(clean(""), vec![]);
    let empty = format!(r#"<worksheet xmlns="{NS}"><sheetData/></worksheet>"#);
    let (rows, seen) = read(book_of(&empty));
    assert_eq!((rows, seen), (vec![], vec![]));
}

/// A styles part whose cell formats are General, then the built-in date,
/// time, date-time and elapsed time formats, then a format it does not
/// define.
fn styles() -> Item {
    let xfs: String = [0, 14, 20, 22, 46, 5]
        .map(|id| format!(r#"<xf numFmtId="{id}"/>"#))
        .concat();
    let xml = format!(r#"<styleSheet xmlns="{NS}"><cellXfs>{xfs}</cellXfs></styleSheet>"#);
    Item::stored("xl/styles.xml", xml.as_bytes())
}

/// The cells of a sheet whose cells, of style 0 to 6 in turn, hold `v`,
/// in a workbook with [`styles`] and `date1904` when it says so.
fn styled(v: &str, date1904: bool) -> Vec<String> {
    let cells: String = (0..=6)
        .map(|s| format!(r#"<c s="{s}"><v>{v}</v></c>"#))
        .collect();
    let sheet = worksheet(&format!("<row>{cells}</row>"));
    let related = rel("rId9", STYLES, "styles.xml");
    let items = workbook(&[("Sheet1", &sheet)], date1904, &related, vec![styles()]);
    let mut diagnostics = Diagnostics::new();
    let book = open(items, &mut diagnostics);
    let expected = if date1904 {
        DateSystem::Base1904
    } else {
        DateSystem::Compatibility1900
    };
    assert_eq!(book.date_system(), expected);
    let (mut rows, seen) = read_book(&book, diagnostics);
    assert_eq!(seen, vec![]);
    rows.remove(0).1
}

#[test]
fn a_number_is_made_the_date_or_time_its_format_says() {
    assert_eq!(
        styled("61.75", false),
        [
            "n:61.75",
            "date:1900-03-01",
            "time:18:00:00.000",
            "dt:1900-03-01T18:00:00.000",
            "el:5335200000",
            // A format it does not define, and one past the formats.
            "n:61.75",
            "n:61.75",
        ]
    );
    assert_eq!(styled("1", true)[1], "date:1904-01-02");
}

#[test]
fn a_number_without_styles_is_general() {
    assert_eq!(
        clean(r#"<row><c s="3"><v>1</v></c></row>"#),
        vec![row(1, &["n:1"])]
    );
}

#[test]
fn a_row_buffer_holds_one_row_at_a_time() {
    let body = concat!(
        r#"<row><c t="str"><v>long text</v></c><c><v>1</v></c></row>"#,
        r#"<row><c t="e"><v>#N/A</v></c></row>"#,
    );
    let mut diagnostics = Diagnostics::new();
    let book = open(book_of(&worksheet(body)), &mut diagnostics);
    let mut rows = book.rows(0).unwrap();
    let mut row = Row::new();
    assert!((row.is_empty(), row.len(), row.number()) == (true, 0, 0));
    assert!(rows.next_row(&mut row, &mut diagnostics).unwrap());
    assert_eq!(row.get(0), Some(Value::String("long text")));
    assert!(rows.next_row(&mut row, &mut diagnostics).unwrap());
    assert_eq!((row.number(), row.len(), row.is_empty()), (2, 1, false));
    assert_eq!((row.get(0), row.get(1)), (Some(Value::Error("#N/A")), None));
    assert!(!rows.next_row(&mut row, &mut diagnostics).unwrap());
    assert!(diagnostics.records().is_empty());
}

#[test]
fn a_sheet_is_found_by_its_name_letter_case_aside_unless_two_have_it() {
    let sheet = worksheet("");
    let sheets = [
        ("Data", &*sheet),
        ("ÄÖ", &*sheet),
        ("äö", &*sheet),
        ("x", &*sheet),
    ];
    let mut diagnostics = Diagnostics::new();
    let book = open(workbook(&sheets, false, "", vec![]), &mut diagnostics);
    assert_eq!(book.sheets().len(), 4);
    assert_eq!(book.sheet("DATA"), SheetLookup::Found(0));
    assert_eq!(book.sheet("X"), SheetLookup::Found(3));
    assert_eq!(book.sheet("äÖ"), SheetLookup::Duplicate);
    assert_eq!(book.sheet("missing"), SheetLookup::Missing);
}

#[test]
fn only_a_worksheet_whose_part_is_known_has_rows() {
    let chart = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet";
    let related = rel("rId8", chart, "chartsheets/sheet1.xml");
    let xml = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}"><sheets><sheet name="A" sheetId="1" r:id="rId1"/><sheet name="C" sheetId="2" r:id="rId8"/><sheet name="M" sheetId="3" r:id="rId7"/></sheets></workbook>"#
    );
    let mut items = workbook(&[("A", &worksheet(""))], false, &related, vec![]);
    items[2] = Item::stored("xl/workbook.xml", xml.as_bytes());
    items.push(Item::stored("xl/chartsheets/sheet1.xml", b"<chartsheet/>"));
    let mut diagnostics = Diagnostics::new();
    let book = open(items, &mut diagnostics);
    assert!(book.rows(0).is_some());
    // A chartsheet, a sheet whose `r:id` leads nowhere, and no sheet.
    assert!(book.rows(1).is_none());
    assert!(book.rows(2).is_none());
    assert!(book.rows(3).is_none());
}

#[test]
fn readers_of_two_sheets_read_in_turns_and_outlive_the_book() {
    let first = worksheet(r#"<row><c><v>1</v></c></row><row><c><v>2</v></c></row>"#);
    let second = worksheet(r#"<row r="5"><c t="s"><v>0</v></c></row><row><c><v>3</v></c></row>"#);
    let related = rel("rId9", super::super::SHARED_STRINGS, "sharedStrings.xml");
    let sst = format!(r#"<sst xmlns="{NS}"><si><t>a</t></si></sst>"#);
    let strings = Item::stored("xl/sharedStrings.xml", sst.as_bytes());
    let items = workbook(
        &[("1", &first), ("2", &second)],
        false,
        &related,
        vec![strings],
    );
    let mut diagnostics = Diagnostics::new();
    let book = open(items, &mut diagnostics);
    let mut one = book.rows(0).unwrap();
    let mut two = book.rows(1).unwrap();
    drop(book);
    let mut row = Row::new();
    let mut read = Vec::new();
    for _ in 0..2 {
        assert!(one.next_row(&mut row, &mut diagnostics).unwrap());
        read.push(shown(&row));
        assert!(two.next_row(&mut row, &mut diagnostics).unwrap());
        read.push(shown(&row));
    }
    assert!(!one.next_row(&mut row, &mut diagnostics).unwrap());
    assert!(!two.next_row(&mut row, &mut diagnostics).unwrap());
    assert_eq!(
        read,
        vec![
            row_of(1, "n:1"),
            row_of(5, "s:a"),
            row_of(2, "n:2"),
            row_of(6, "n:3")
        ]
    );
    assert!(diagnostics.records().is_empty());
}

/// A row of one value.
fn row_of(number: u32, value: &str) -> Shown {
    row(number, &[value])
}

#[test]
fn a_strict_worksheet_reads_as_a_transitional_one() {
    let strict = "http://purl.oclc.org/ooxml/spreadsheetml/main";
    let sheet = format!(
        r#"<x:worksheet xmlns:x="{strict}"><x:sheetData><x:row><x:c><x:v>7</x:v></x:c></x:row></x:sheetData></x:worksheet>"#
    );
    let (rows, seen) = read(book_of(&sheet));
    assert_eq!((rows, seen), (vec![row(1, &["n:7"])], vec![]));
}

#[test]
fn a_declaration_comments_and_processing_instructions_are_passed_over() {
    let sheet = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="{NS}"><!-- a --><sheetData><row><?pi x?><c><v>1</v><!-- b --></c></row></sheetData></worksheet>"#
    );
    let (rows, seen) = read(book_of(&sheet));
    assert_eq!((rows, seen), (vec![row(1, &["n:1"])], vec![]));
}

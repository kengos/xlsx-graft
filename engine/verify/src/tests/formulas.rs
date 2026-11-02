//! The formulas found as the parts are read: every place a workbook keeps
//! one, with where it is, how its text is joined, the formula limit, and
//! the defined names and tables found with them.

use super::{Book, Reader, WORKSHEET};
use crate::collect::FormulaSink;
use crate::{Limits, Rules, read_parts};
use std::io::Cursor;
use xlsx_graft_diagnostics::{Diagnostics, Limit, PartKind};
use xlsx_graft_package::Package;

/// The SpreadsheetML namespace.
const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
/// The worksheet part the helpers add.
const SHEET: &str = "/xl/worksheets/sheet1.xml";

/// A sink that writes down what it is handed, one line each.
#[derive(Default)]
struct Record(Vec<String>);

impl FormulaSink for Record {
    fn formula(&mut self, part: &str, at: &str, text: &str) {
        self.0.push(format!("{part} {at}: {text}"));
    }

    fn markup(&mut self, part: &str, at: &str) {
        self.0.push(format!("{part} {at}: markup"));
    }

    fn too_long(&mut self, part: &str, at: &str, size: u64) {
        self.0.push(format!("{part} {at}: {size} bytes, too long"));
    }

    fn defined_name(&mut self, part: &str, at: &str, name: &str, sheet: Option<&str>, text: &str) {
        let sheet = sheet.unwrap_or("-");
        self.0
            .push(format!("{part} {at}: {name} in {sheet} = {text}"));
    }

    fn table(&mut self, part: &str, names: &[String]) {
        for name in names {
            self.0.push(format!("{part} table {name}"));
        }
    }
}

/// What reading `book` under `limits` hands over, one line each.
fn found_with(book: Book, limits: &Limits) -> Vec<String> {
    found_under(book, limits, &Rules::default())
}

/// What reading `book` under `limits` and `rules` hands over, one line
/// each, the rules allowing all it holds.
fn found_under(book: Book, limits: &Limits, rules: &Rules) -> Vec<String> {
    let reader = Reader {
        inner: Cursor::new(book.zip()),
        poisoned: (0, 0),
    };
    let mut record = Record::default();
    let mut diagnostics = Diagnostics::new();
    let mut package = Package::open(reader, &limits.package, &mut diagnostics)
        .unwrap()
        .unwrap();
    read_parts(&mut package, limits, rules, &mut record, &mut diagnostics).unwrap();
    assert!(diagnostics.finish().is_empty());
    record.0
}

/// What reading `book` under the default limits hands over.
fn found(book: Book) -> Vec<String> {
    found_with(book, &Limits::default())
}

/// A workbook whose worksheet holds `body`.
fn with_sheet_body(body: &str) -> Book {
    let xml = format!(r#"<worksheet xmlns="{MAIN}">{body}</worksheet>"#);
    Book::new()
        .part(SHEET, WORKSHEET, &xml)
        .rel("/xl/workbook.xml", "worksheet", SHEET)
}

/// A workbook whose worksheet holds a cell for each of `cells`, its
/// reference and its formula, XML-escaped.
fn with_cells(cells: &[(&str, &str)]) -> Book {
    let cells: String = (cells.iter())
        .map(|(r, f)| format!(r#"<c r="{r}"><f>{f}</f></c>"#))
        .collect();
    with_sheet_body(&format!(
        r#"<sheetData><row r="1">{cells}</row></sheetData>"#
    ))
}

/// The lines `found` would write for the worksheet's formulas `lines`.
fn in_sheet(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| format!("{SHEET} {line}")).collect()
}

#[test]
fn every_place_a_formula_is_kept_is_found_with_where_it_is() {
    let x14 = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
    let xm = "http://schemas.microsoft.com/office/excel/2006/main";
    let body = format!(
        r#"<sheetData><row r="1"><c r="A1"><f>SUM(1)</f></c><c r="A2"/><c><f>A1</f></c></row></sheetData>
        <conditionalFormatting sqref="C1:C9"><cfRule type="expression"><formula>A1&gt;1</formula></cfRule>
        <cfRule type="colorScale"><colorScale><cfvo type="num" val="0"/><cfvo type="formula" val="$A$1"/></colorScale></cfRule></conditionalFormatting>
        <controls><control><controlPr macro="[0]!OnClick" linkedCell="$F$1" listFillRange="Lists!$A$1:$A$3"/></control></controls>
        <dataValidations><dataValidation sqref="D1:D9"><formula1>ISNUMBER(D1)</formula1><formula2>9</formula2></dataValidation></dataValidations>
        <x><formula>B1</formula></x><dataValidation sqref="G1:G9"/><formula>B2</formula>
        <sheetData><row r="2"><c r="B5"><f>1</f></c><c r="B6"/></row></sheetData><f>B3</f>
        <extLst><ext xmlns:x14="{x14}" xmlns:xm="{xm}"><x14:dataValidations><x14:dataValidation>
        <x14:formula1> <xm:f>Lists!$A$1:$A$3</xm:f> </x14:formula1><xm:sqref>E1</xm:sqref></x14:dataValidation></x14:dataValidations></ext></extLst>"#
    );
    let table = r#"<table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="T" displayName="Sales"><tableColumns>
        <tableColumn name="a"><calculatedColumnFormula>[@b]*2</calculatedColumnFormula><totalsRowFormula>SUBTOTAL(109,[a])</totalsRowFormula></tableColumn></tableColumns></table>"#;
    let pivot = r#"<pivotCacheDefinition><cacheFields><cacheField name="c" formula="b*2"/><cacheField name="d"/></cacheFields>
        <calculatedItems><calculatedItem formula="'x'+'y'"/></calculatedItems></pivotCacheDefinition>"#;
    let drawing = r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"><xdr:sp macro="" textlink="$A$1"/>
        <a14:cameraTool xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main" cellRange="Sheet1!$A$1:$B$2"/><xdr:sp/><xdr:cxnSp macro="A"/><xdr:pic macro="B"/><xdr:graphicFrame macro="C"/></xdr:wsDr>"#;
    let chart = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:val><c:numRef><c:f>Sheet1!$B$1:$B$3</c:f></c:numRef></c:val></c:chartSpace>"#;
    let book = with_sheet_body(&body)
        .related("/xl/tables/table1.xml", table)
        .related("/xl/pivotCache/pivotCacheDefinition1.xml", pivot)
        .related("/xl/charts/chart1.xml", chart)
        .related("/xl/drawings/drawing1.xml", drawing)
        .related(
            "/xl/revisions/revisionLog1.xml",
            "<revisions><rdn><formula>Data!$A$1</formula><oldFormula>Data!$B$1</oldFormula></rdn></revisions>",
        );
    let mut expected = in_sheet(&[
        "A1: SUM(1)",
        "f: A1",
        "formula C1:C9: A1>1",
        "cfvo@val C1:C9: 0",
        "cfvo@val C1:C9: $A$1",
        "controlPr@macro: [0]!OnClick",
        "controlPr@linkedCell: $F$1",
        "controlPr@listFillRange: Lists!$A$1:$A$3",
        "formula1 D1:D9: ISNUMBER(D1)",
        "formula2 D1:D9: 9",
        "formula: B1",
        "formula: B2",
        "B5: 1",
        "f: B3",
        "f: Lists!$A$1:$A$3",
    ]);
    expected.extend(
        [
            "/xl/tables/table1.xml table T",
            "/xl/tables/table1.xml table Sales",
            "/xl/tables/table1.xml calculatedColumnFormula: [@b]*2",
            "/xl/tables/table1.xml totalsRowFormula: SUBTOTAL(109,[a])",
            "/xl/pivotCache/pivotCacheDefinition1.xml cacheField@formula: b*2",
            "/xl/pivotCache/pivotCacheDefinition1.xml calculatedItem@formula: 'x'+'y'",
            "/xl/charts/chart1.xml f: Sheet1!$B$1:$B$3",
            "/xl/drawings/drawing1.xml sp@textlink: $A$1",
            "/xl/drawings/drawing1.xml cameraTool@cellRange: Sheet1!$A$1:$B$2",
            "/xl/drawings/drawing1.xml cxnSp@macro: A",
            "/xl/drawings/drawing1.xml pic@macro: B",
            "/xl/drawings/drawing1.xml graphicFrame@macro: C",
            "/xl/revisions/revisionLog1.xml formula: Data!$A$1",
            "/xl/revisions/revisionLog1.xml oldFormula: Data!$B$1",
        ]
        .map(String::from),
    );
    assert_eq!(found(book), expected);
}

#[test]
fn the_defined_names_of_the_workbook_are_found_with_their_scope() {
    let workbook = format!(
        r#"<workbook xmlns="{MAIN}"><sheets><sheet name="Data" sheetId="1"/><sheet name="it's"/></sheets>
        <definedNames><definedName name="Rate">Data!$A$1</definedName>
        <definedName name="Fetch" localSheetId="1">_xlfn.LAMBDA(_xlpm.u,_xlpm.u)</definedName>
        <definedName>SUM(1)</definedName><definedName name="Marked">1<x/></definedName><definedName name="Blank"> </definedName><definedName name="Empty"/></definedNames></workbook>"#
    );
    let book = Book::new().main("application/xml", &workbook);
    let expected = [
        "/xl/workbook.xml definedName Rate: Rate in - = Data!$A$1",
        "/xl/workbook.xml definedName Fetch: Fetch in 1 = _xlfn.LAMBDA(_xlpm.u,_xlpm.u)",
        "/xl/workbook.xml definedName:  in - = SUM(1)",
        "/xl/workbook.xml definedName Marked: markup",
    ];
    assert_eq!(found(book), expected);
}

#[test]
fn a_formula_is_its_text_and_references_joined_and_markup_inside_is_not_put_together() {
    let book = with_cells(&[
        ("A1", "IF(A2=&quot;x&quot;,1,2)"),
        ("A2", "&#x57;EBSERVICE(1)"),
        ("A3", r#"<![CDATA[SUM("<x>")]]>"#),
        // A formula element nested in another is a formula of its own; the
        // outer one, with an element inside it, is markup.
        ("A4", "WEB<f>SUM(1)</f>SERVICE(1)"),
        ("A5", "WEB<x/>SERVICE(1)"),
        (
            "B1",
            r#"WEB<mc:AlternateContent xmlns:mc="urn:mc"><mc:Choice>SERVICE</mc:Choice><mc:Fallback>X</mc:Fallback></mc:AlternateContent>(1)"#,
        ),
        ("A6", "_x0057_EBSERVICE(1)"),
        ("A9", "WEB<!-- x -->SER<?pi x?>VICE(1)"),
        // Empty, or white space only: no formula.
        ("A7", ""),
        ("A8", " \n "),
        ("A10", " <x/> "),
    ]);
    let expected = in_sheet(&[
        r#"A1: IF(A2="x",1,2)"#,
        "A2: WEBSERVICE(1)",
        r#"A3: SUM("<x>")"#,
        "A4: SUM(1)",
        "A4: markup",
        "A5: markup",
        "B1: markup",
        "A6: _x0057_EBSERVICE(1)",
        "A9: WEBSERVICE(1)",
    ]);
    assert_eq!(found(book), expected);
}

#[test]
fn a_formula_is_found_in_a_part_whatever_its_media_type_says() {
    let image = "/xl/media/image1.png";
    let book = (Book::new())
        .part(image, "image/png", r#"<x><f>WEBSERVICE("x")</f><f>SUM(1)"#)
        .rel("/xl/workbook.xml", "image", image);
    // The formula left open where reading stops is not handed over.
    assert_eq!(found(book), [r#"/xl/media/image1.png f: WEBSERVICE("x")"#]);
}

#[test]
fn a_formula_is_kept_up_to_the_limit_and_counted_past_it() {
    let limits = |max| Limits {
        max_formula_bytes: Limit::default_of(max),
        ..Limits::default()
    };
    // The cell's formula is eight bytes once its reference is read, the
    // attribute's nine, and the defined name's ten.
    let book = || {
        with_cells(&[("A1", "SUM(1&amp;2)")]).related(
            "/xl/x.xml",
            r#"<x><cfvo val="SUM(1&amp;22)"/><definedName name="N">SUM(1)+222</definedName></x>"#,
        )
    };
    let at_limit = found_with(book(), &limits(10));
    assert_eq!(
        at_limit,
        [
            format!("{SHEET} A1: SUM(1&2)"),
            "/xl/x.xml cfvo@val: SUM(1&22)".to_owned(),
            "/xl/x.xml definedName N: N in - = SUM(1)+222".to_owned(),
        ]
    );
    let past = found_with(book(), &limits(8));
    assert_eq!(
        past,
        [
            format!("{SHEET} A1: SUM(1&2)"),
            "/xl/x.xml cfvo@val: 9 bytes, too long".to_owned(),
            "/xl/x.xml definedName N: 10 bytes, too long".to_owned(),
        ]
    );
    let past = found_with(book(), &limits(7));
    assert_eq!(past[0], format!("{SHEET} A1: 8 bytes, too long"));
}

#[test]
fn an_attribute_of_another_namespace_is_no_decoy_for_the_real_one() {
    let xml = r#"<x xmlns:z="urn:z"><cfvo z:val="0" val="WEBSERVICE(1)"/><cfvo z:val="1"/>
        <definedName z:name="Safe" name="Auto_Open">1</definedName></x>"#;
    let book = Book::new().related("/xl/x.xml", xml);
    let expected = [
        "/xl/x.xml cfvo@val: WEBSERVICE(1)",
        "/xl/x.xml definedName Auto_Open: Auto_Open in - = 1",
    ];
    assert_eq!(found(book), expected);
}

#[test]
fn a_place_or_name_is_kept_up_to_1024_bytes() {
    // A three-byte character straddles the 1,024th byte.
    let long = format!("{}あ", "N".repeat(1_022));
    let xml = format!(r#"<x><definedName name="{long}">1</definedName></x>"#);
    let book = Book::new().related("/xl/x.xml", &xml);
    let name = "N".repeat(1_022);
    let expected = format!("/xl/x.xml definedName {name}: {name} in - = 1");
    assert_eq!(found(book), [expected]);
}

#[test]
fn the_formulas_of_a_linked_object_and_of_an_external_workbook_are_found() {
    let link = r#"<externalLink><externalBook><definedNames><definedName name="R" refersTo="Data!$A$1"/></definedNames></externalBook>
        <oleObjects><oleObject progId="Excel.Sheet.12" link="[1]!''''" oleUpdate="OLEUPDATE_ALWAYS"><objectPr macro="[0]!OnOpen"/></oleObject></oleObjects></externalLink>"#;
    let book = Book::new().related("/xl/externalLinks/externalLink1.xml", link);
    let rules = Rules::default()
        .allow_part(PartKind::ExternalLink)
        .allow_part(PartKind::Ole);
    let expected = [
        "/xl/externalLinks/externalLink1.xml definedName@refersTo: Data!$A$1",
        "/xl/externalLinks/externalLink1.xml oleObject@link: [1]!''''",
        "/xl/externalLinks/externalLink1.xml objectPr@macro: [0]!OnOpen",
    ];
    assert_eq!(found_under(book, &Limits::default(), &rules), expected);
}

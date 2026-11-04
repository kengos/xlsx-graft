//! The formulas `verify` judges as it reads the parts: at every place a
//! workbook keeps one, their names resolved against the workbook part's
//! defined names and the table parts' tables, each sheet's own names
//! found by the workbook part's sheets.

use super::zip::Item;
use super::{Book, Reader, Table, VerifyError, WORKSHEET, verify};
use crate::{Limits, Rules, Targets};
use std::io::Cursor;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit};

/// The SpreadsheetML namespace.
const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
/// The namespace of a relationship's `r:id`.
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// A workbook part's media type.
const WORKBOOK: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
/// A table part's media type.
const TABLE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";
/// The parts of the sheets `S1` and `S2`.
const SHEETS: [&str; 2] = ["/xl/worksheets/sheet1.xml", "/xl/worksheets/sheet2.xml"];

/// A workbook of the sheets `S1` and `S2`, its workbook part holding the
/// `definedName` elements `names`, each sheet's part holding a cell for
/// each of its formulas, `A1` on, then `more`.
fn workbook(names: &str, sheets: [&[&str]; 2], more: [&str; 2]) -> Book {
    let xml = format!(
        r#"<workbook xmlns="{MAIN}" xmlns:r="{R}"><sheets><sheet name="S1" sheetId="1" r:id="rId1"/><sheet name="S2" sheetId="2" r:id="rId2"/></sheets><definedNames>{names}</definedNames></workbook>"#
    );
    let mut book = Book::new().main(WORKBOOK, &xml);
    for ((part, formulas), more) in SHEETS.into_iter().zip(sheets).zip(more) {
        let cells: String = (formulas.iter().zip(1..))
            .map(|(f, row)| format!(r#"<c r="A{row}"><f>{f}</f></c>"#))
            .collect();
        let xml = format!(
            r#"<worksheet xmlns="{MAIN}"><sheetData>{cells}</sheetData>{more}</worksheet>"#
        );
        book = book
            .part(part, WORKSHEET, &xml)
            .rel("/xl/workbook.xml", "worksheet", part);
    }
    book
}

impl Book {
    /// This workbook, with the part `name` of `content_type` holding `xml`,
    /// related from `S1` by the type `rel`.
    fn of_s1(self, rel: &str, name: &str, content_type: &str, xml: &str) -> Self {
        (self.part(name, content_type, xml)).rel(SHEETS[0], rel, name)
    }
}

/// [`workbook`] with no more than cells.
fn cells(names: &str, sheets: [&[&str]; 2]) -> Book {
    workbook(names, sheets, ["", ""])
}

/// A table part named `name`, of the column `column`, with `xml` inside
/// the column.
fn table(name: &str, column: &str, xml: &str) -> String {
    format!(
        r#"<table xmlns="{MAIN}" name="{name}" displayName="{name}"><tableColumns><tableColumn name="{column}">{xml}</tableColumn></tableColumns></table>"#
    )
}

/// What verifying `book` under `limits` and `rules` finds, one line each:
/// where, through which defined name, and what.
fn judged_under(book: Book, limits: &Limits, rules: &Rules) -> Vec<String> {
    judged_against(book, limits, rules, &Targets::default())
}

/// [`judged_under`], the functions checked against `targets`.
fn judged_against(book: Book, limits: &Limits, rules: &Rules, targets: &Targets) -> Vec<String> {
    let reader = Reader {
        inner: Cursor::new(book.zip()),
        poisoned: (0, 0),
    };
    let mut diagnostics = Diagnostics::new();
    verify(reader, limits, rules, &Table, targets, &mut diagnostics).unwrap();
    (diagnostics.finish().into_iter())
        .map(|record| {
            let location = record.location;
            let part = location
                .part
                .map(|p| p.as_str().to_owned())
                .unwrap_or_default();
            let place = location
                .place
                .map(|p| p.as_str().to_owned())
                .unwrap_or_default();
            let via = (location.defined_name)
                .map_or_else(String::new, |n| format!(" via {}", n.as_str()));
            format!("{part} {place}{via}: {}", brief(&record.diagnostic))
        })
        .collect()
}

/// [`judged_under`] the default limits and rules.
fn judged(book: Book) -> Vec<String> {
    judged_under(book, &Limits::default(), &Rules::default())
}

/// What `diagnostic` says, briefly.
fn brief(diagnostic: &Diagnostic) -> String {
    match diagnostic {
        Diagnostic::FunctionDangerous { name, count, .. } => format!("dangerous {name} x{count}"),
        Diagnostic::FunctionDenied {
            name,
            denial,
            count,
        } => format!("{denial} {name} x{count}"),
        Diagnostic::FormulaDenied { kind, count } => format!("{kind} x{count}"),
        Diagnostic::FormulaMalformed { reason, count } => format!("{reason} x{count}"),
        Diagnostic::FormulaTooLong { size, count, .. } => format!("{size} bytes x{count}"),
        Diagnostic::TargetFunctionUnsupported {
            name,
            app,
            unsupported,
            unverified,
            count,
            ..
        } => format!("targets {name} {app} [{unsupported}] [{unverified}] x{count}"),
        Diagnostic::BestEffortFunctionUnsupported {
            name,
            app,
            unsupported,
            unverified,
            count,
            ..
        } => format!("bestEffort {name} {app} [{unsupported}] [{unverified}] x{count}"),
        other => format!("{other:?}"),
    }
}

#[test]
fn every_place_a_workbook_keeps_a_formula_is_judged() {
    let names = r#"<definedName name="Fetch">WEBSERVICE("u")</definedName>"#;
    let sheet = r#"<conditionalFormatting sqref="C1:C9"><cfRule type="colorScale"><colorScale><cfvo type="formula" val="WEBSERVICE(1)"/></colorScale></cfRule></conditionalFormatting><dataValidations><dataValidation sqref="B1:B9"><formula1>RTD("x",,"y")</formula1></dataValidation></dataValidations>"#;
    let chart = r#"<c:chartSpace xmlns:c="urn:c"><c:f>S1!$A$1:$A$3</c:f><c:f>_xludf.EVIL(1)</c:f></c:chartSpace>"#;
    let drawing = r#"<xdr:wsDr xmlns:xdr="urn:x"><xdr:sp macro="Run" textlink="S2!A1"/><xdr:pic macro="[1]!Run"/></xdr:wsDr>"#;
    let book = workbook(
        names,
        [&[r#"WEBSERVICE("u")"#], &["cmd|' /C calc'!A0"]],
        [sheet, ""],
    )
    .of_s1(
        "table",
        "/xl/tables/table1.xml",
        TABLE,
        &table(
            "T",
            "c",
            "<calculatedColumnFormula>INDIRECT(\"A1\")</calculatedColumnFormula>",
        ),
    )
    .of_s1("chart", "/xl/charts/chart1.xml", "application/xml", chart)
    .of_s1(
        "drawing",
        "/xl/drawings/drawing1.xml",
        "application/xml",
        drawing,
    );
    let expected = [
        "/xl/workbook.xml definedName Fetch: dangerous WEBSERVICE x1",
        "/xl/worksheets/sheet1.xml A1: dangerous WEBSERVICE x2",
        "/xl/worksheets/sheet1.xml formula1 B1:B9: unreviewed RTD x1",
        "/xl/worksheets/sheet2.xml A1: dde x1",
        "/xl/tables/table1.xml calculatedColumnFormula: unknown INDIRECT x1",
        "/xl/charts/chart1.xml f: unknown _XLUDF.EVIL x1",
        "/xl/drawings/drawing1.xml sp@macro: unknown RUN x2",
        "/xl/drawings/drawing1.xml pic@macro: external_link x1",
    ];
    assert_eq!(judged(book), expected);
}

#[test]
fn what_excel_writes_with_tables_names_and_sheets_is_not_refused() {
    let names = r#"<definedName name="Rate"><![CDATA[S1!]]>&#36;B<!-- the row -->&#36;1</definedName><definedName name="MyLambda">_xlfn.LAMBDA(_xlpm.x,SUM(_xlpm.x,Rate))</definedName>"#;
    let one: &[&str] = &[
        "SUM(Sales[WEBSERVICE])",
        "SUM(Sales)+ROWS_ALIAS",
        "RTD!A1+'WEBSERVICE'!A1",
        "MyLambda(A1)+SUM(Rate)",
        "_xlfn.LAMBDA(_xlpm.x,_xlpm.x+1)(A1)",
    ];
    let book = cells(names, [one, &[]])
        .of_s1(
            "table",
            "/xl/tables/table1.xml",
            TABLE,
            &table("Sales", "WEBSERVICE", ""),
        )
        .of_s1(
            "table",
            "/xl/tables/table2.xml",
            TABLE,
            &table("ROWS_ALIAS", "c", ""),
        );
    assert_eq!(judged(book), Vec::<String>::new());
}

#[test]
fn a_lambda_defined_name_is_judged_by_its_formula_in_its_sheet() {
    let fetch = "_xlfn.LAMBDA(_xlpm.u,WEBSERVICE(_xlpm.u))";
    let names = format!(
        r#"<definedName name="Fetch">{fetch}</definedName><definedName name="Outer">Fetch("u")</definedName><definedName name="Loop">Loop+1</definedName><definedName name="Pick" localSheetId="1">{fetch}</definedName><definedName name="Pick">_xlfn.LAMBDA(_xlpm.u,SUM(_xlpm.u))</definedName>"#
    );
    let book = cells(&names, [&["Loop+Pick(1)", "Outer"], &["Pick(1)"]]);
    let expected = [
        // Pick, scoped to S2, is judged at its own place too, counted
        // with Fetch: one diagnostic per part and finding.
        "/xl/workbook.xml definedName Fetch: dangerous WEBSERVICE x2",
        "/xl/workbook.xml definedName Outer via Fetch: dangerous WEBSERVICE x1",
        "/xl/worksheets/sheet2.xml A1 via Pick: dangerous WEBSERVICE x1",
    ];
    assert_eq!(judged(book), expected);
}

#[test]
fn only_the_workbook_part_and_the_table_parts_name_what_a_word_means() {
    let mc = "http://schemas.openxmlformats.org/markup-compatibility/2006";
    let names = format!(
        r#"<f>SUM(1)</f><definedName name="Marked">1<x/></definedName><definedName name="Odd" localSheetId="x">SUM(1)</definedName><definedName name="Spaced" localSheetId=" 0 ">SUM(1)</definedName><definedName name="Blank"> </definedName><definedName name="Bare"/><z:definedName xmlns:z="urn:z" name="Other">SUM(1)</z:definedName><extLst><ext><definedName name="Ext">SUM(1)</definedName></ext></extLst><mc:AlternateContent xmlns:mc="{mc}"><mc:Choice Requires="x"><definedName name="Alt">SUM(1)</definedName></mc:Choice></mc:AlternateContent>"#
    );
    let decoy = r#"<x><definedName name="Safe">WEBSERVICE(1)</definedName><table name="Tbl"/></x>"#;
    let inside = r#"<calculatedColumnFormula>1<x/></calculatedColumnFormula><definedName name="InTable">SUM(1)</definedName><table name="Extra"/>"#;
    let one: &[&str] = &[
        "Safe(1)+SUM(Tbl)",
        "Odd+Spaced",
        "Marked+Blank+Bare+Other+Ext+Alt",
        "SUM(Real)+SUM(Extra)+InTable",
    ];
    let book = cells(&names, [one, &["Spaced"]])
        .related("/xl/x.xml", decoy)
        .of_s1(
            "table",
            "/xl/tables/table1.xml",
            TABLE,
            &table("Real", "c", inside),
        );
    let expected = [
        "/xl/worksheets/sheet1.xml A1: unknown SAFE x1",
        "/xl/worksheets/sheet1.xml A1: unknown TBL x1",
        "/xl/worksheets/sheet1.xml A2: unknown ODD x1",
        "/xl/worksheets/sheet1.xml A3: unknown MARKED x1",
        "/xl/worksheets/sheet1.xml A3: unknown BLANK x1",
        "/xl/worksheets/sheet1.xml A3: unknown BARE x1",
        "/xl/worksheets/sheet1.xml A3: unknown OTHER x1",
        "/xl/worksheets/sheet1.xml A3: unknown EXT x1",
        "/xl/worksheets/sheet1.xml A3: unknown ALT x1",
        "/xl/worksheets/sheet1.xml A4: unknown EXTRA x1",
        "/xl/worksheets/sheet1.xml A4: unknown INTABLE x1",
        // Spaced is S1's alone: S2 does not see it.
        "/xl/worksheets/sheet2.xml A1: unknown SPACED x1",
        "/xl/x.xml definedName Safe: dangerous WEBSERVICE x1",
        "/xl/workbook.xml definedName Marked: markup x1",
        "/xl/tables/table1.xml calculatedColumnFormula: markup x1",
    ];
    assert_eq!(judged(book), expected);
}

#[test]
fn a_formula_too_long_or_with_markup_is_refused_unread_once_per_part() {
    let book = cells(
        r#"<definedName name="Long">SUM(1,2,3)</definedName>"#,
        [
            &["SUM(1,2,3)", "1<x/>", "SUM(4,5,6,7)", "2<y/>"],
            &["SUM(7,8,9)"],
        ],
    )
    .of_s1(
        "table",
        "/xl/tables/table1.xml",
        TABLE,
        &table(
            "T",
            "c",
            "<calculatedColumnFormula>SUM(1,2,3)</calculatedColumnFormula>",
        ),
    );
    let limits = Limits {
        max_formula_bytes: Limit::default_of(9),
        ..Limits::default()
    };
    // Long is no name, nor a formula judged: Excel would not read it either.
    let expected = [
        "/xl/workbook.xml definedName Long: 10 bytes x1",
        "/xl/worksheets/sheet1.xml A1: 12 bytes x2",
        "/xl/worksheets/sheet1.xml A2: markup x2",
        "/xl/worksheets/sheet2.xml A1: 10 bytes x1",
        "/xl/tables/table1.xml calculatedColumnFormula: 10 bytes x1",
    ];
    assert_eq!(judged_under(book, &limits, &Rules::default()), expected);
}

#[test]
fn a_local_sheet_id_counts_every_sheet_and_a_part_two_sheets_name_is_neither() {
    let xml = format!(
        r#"<workbook xmlns="{MAIN}" xmlns:r="{R}"><sheets><sheet name="C" sheetId="1" r:id="rId1"/><sheet name="S1" sheetId="2" r:id="rId2"/><sheet name="S2" sheetId="3" r:id="rId3"/><sheet name="Again" sheetId="4" r:id="rId3"/><sheet name="Lost" sheetId="5" r:id="rId9"/></sheets><definedNames><definedName name="Bar" localSheetId="1">SUM(1)</definedName><definedName name="Foo" localSheetId="2">SUM(1)</definedName></definedNames></workbook>"#
    );
    let sheet = |f: &str| {
        format!(
            r#"<worksheet xmlns="{MAIN}"><sheetData><c r="A1"><f>{f}</f></c></sheetData></worksheet>"#
        )
    };
    let chart = "/xl/chartsheets/sheet1.xml";
    let book = Book::new()
        .main(WORKBOOK, &xml)
        .part(
            chart,
            "application/xml",
            &format!(r#"<chartsheet xmlns="{MAIN}"/>"#),
        )
        .rel("/xl/workbook.xml", "chartsheet", chart)
        .part(SHEETS[0], WORKSHEET, &sheet("Bar(1)"))
        .rel("/xl/workbook.xml", "worksheet", SHEETS[0])
        .part(SHEETS[1], WORKSHEET, &sheet("Foo(1)"))
        .rel("/xl/workbook.xml", "worksheet", SHEETS[1]);
    // Bar is S1's, the sheet at 1 after the chartsheet; Foo is S2's, but
    // S2's part is Again's too, so it is no sheet's. Lost has no part.
    // What the workbook reader reports of the sheets is reported too.
    assert_eq!(
        judged(book),
        [
            r#"/xl/workbook.xml : SheetPartDuplicate { name: Echo("Again"), other: Echo("S2") }"#,
            r#"/xl/workbook.xml : SheetRelationshipMissing { relationship_id: Echo("rId9") }"#,
            "/xl/worksheets/sheet2.xml A1: unknown FOO x1"
        ]
    );
}

#[test]
fn a_workbook_part_that_cannot_be_opened_names_nothing() {
    let names = r#"<definedName name="Rate">1</definedName>"#;
    let xml =
        format!(r#"<workbook xmlns="{MAIN}"><definedNames>{names}</definedNames></workbook>"#);
    let item = Item::stored("xl/workbook.xml", xml.as_bytes()).with_method(99);
    let book = cells(names, [&["SUM(Rate)"], &[]]).replace(item);
    let seen = judged(book);
    assert!(
        seen.contains(&"/xl/worksheets/sheet1.xml A1: unknown RATE x1".to_owned()),
        "{seen:?}"
    );
}

#[test]
fn the_policy_forbids_a_function_the_table_finds_safe() {
    let book = cells("", [&["SUM(1)"], &[]]);
    let rules = Rules::default().forbid_function("SUM", "no sums");
    let seen = judged_under(book, &Limits::default(), &rules);
    assert_eq!(seen.len(), 1);
    assert!(
        seen[0].starts_with("/xl/worksheets/sheet1.xml A1: FunctionForbidden"),
        "{seen:?}"
    );
}

#[test]
fn a_workbook_part_that_breaks_its_schema_names_nothing() {
    let names = r#"<definedName name="Pick" localSheetId="0">WEBSERVICE(1)</definedName>"#;
    // No `sheets`: no part is a sheet, and no defined name is believed.
    let xml =
        format!(r#"<workbook xmlns="{MAIN}"><definedNames>{names}</definedNames></workbook>"#);
    let book = cells(names, [&["Pick"], &[]]).main(WORKBOOK, &xml);
    // So the word is a function, while the name's formula is judged at
    // its own place all the same.
    let expected = [
        "/xl/workbook.xml definedName Pick: dangerous WEBSERVICE x1",
        "/xl/worksheets/sheet1.xml A1: unknown PICK x1",
    ];
    assert_eq!(judged(book), expected);
    // A part that is not well-formed names nothing at all.
    let book = cells(names, [&["Pick"], &[]]).main(WORKBOOK, &xml.replace("</workbook>", ""));
    let seen = judged(book);
    assert!(
        seen.contains(&"/xl/worksheets/sheet1.xml A1: unknown PICK x1".to_owned()),
        "{seen:?}"
    );
    assert!(
        !seen.iter().any(|line| line.contains("via Pick")),
        "{seen:?}"
    );
}

#[test]
fn reading_the_workbook_part_for_its_names_fails_as_the_host_does() {
    let zip = cells(r#"<definedName name="N">MARK</definedName>"#, [&[], &[]]).zip();
    let mark = zip.windows(4).position(|w| w == b"MARK").unwrap();
    let start = zip[..mark]
        .windows(9)
        .rposition(|w| w == b"<workbook")
        .unwrap() as u64;
    let reader = Reader {
        inner: Cursor::new(zip),
        poisoned: (start, start + 1),
    };
    let mut diagnostics = Diagnostics::new();
    let error = verify(
        reader,
        &Limits::default(),
        &Rules::default(),
        &Table,
        &Targets::default(),
        &mut diagnostics,
    );
    assert!(matches!(error, Err(VerifyError::Package(_))), "{error:?}");
}

#[test]
fn a_package_with_no_workbook_part_has_its_formulas_judged_all_the_same() {
    let mut book = cells("", [&["WEBSERVICE(1)"], &[]]);
    book.relationships.retain(|(source, _)| source != "/");
    let seen = judged(book);
    assert!(
        seen.contains(&"/xl/worksheets/sheet1.xml A1: dangerous WEBSERVICE x1".to_owned()),
        "{seen:?}"
    );
}

#[test]
fn the_defined_names_are_kept_up_to_the_limit_and_one_past_it_is_refused() {
    let limits = Limits {
        max_defined_names: Limit::default_of(2),
        ..Limits::default()
    };
    let name = |n: &str| format!(r#"<definedName name="{n}">SUM(1)</definedName>"#);
    let two = [name("A"), name("B")].concat();
    let book = cells(&two, [&["A+B"], &[]]);
    assert_eq!(
        judged_under(book, &limits, &Rules::default()),
        Vec::<String>::new()
    );
    let three = [two, name("C")].concat();
    let book = cells(&three, [&["A+B+C"], &[]]);
    let expected = [
        "/xl/workbook.xml : DefinedNamesTooMany { count: 3, limit: Limit { value: 2, source: Default } }",
        "/xl/worksheets/sheet1.xml A1: unknown C x1",
    ];
    assert_eq!(judged_under(book, &limits, &Rules::default()), expected);
}

#[test]
fn a_damaged_workbook_or_table_part_reports_its_damage_once() {
    let names = r#"<definedName name="Fetch">WEBSERVICE(1)</definedName>"#;
    let book = cells(names, [&[], &[]]);
    let xml = book.parts[0].2.clone();
    let workbook = Item::stored("xl/workbook.xml", &xml).damaged(|data| data[1] = b'W');
    let column = "<calculatedColumnFormula>WEBSERVICE(1)</calculatedColumnFormula>";
    let table = table("T", "c", column);
    let damaged = Item::stored("xl/tables/table1.xml", table.as_bytes()).damaged(|data| {
        // Still well-formed, so the stream reads to the end of the entry.
        let at = data.windows(10).position(|w| w == b"WEBSERVICE").unwrap();
        data[at] = b'X';
    });
    let book = book
        .of_s1("table", "/xl/tables/table1.xml", TABLE, &table)
        .replace(workbook)
        .replace(damaged);
    let seen = judged(book);
    let damage: Vec<&String> = seen
        .iter()
        .filter(|line| line.contains("ZipCrcMismatch"))
        .collect();
    assert_eq!(damage.len(), 2, "{seen:?}");
}

#[test]
fn a_workbook_part_too_large_to_read_in_memory_is_refused_for_it() {
    let mut limits = Limits::default();
    limits.package.xml.max_bytes = Limit::default_of(4_000);
    let names = format!(
        r#"<definedName name="Rate">1</definedName><!-- {} -->"#,
        "x".repeat(5_000)
    );
    let book = cells(&names, [&["SUM(Rate)"], &[]]);
    let seen = judged_under(book, &limits, &Rules::default());
    let expected = [
        "/xl/workbook.xml : XmlTooLarge { size: 5344, limit: Limit { value: 4000, source: Default } }",
        "/xl/worksheets/sheet1.xml A1: unknown RATE x1",
    ];
    assert_eq!(seen, expected);
}

#[test]
fn only_the_first_table_of_a_table_part_names_a_table() {
    let xml = format!(
        r#"<table xmlns="{MAIN}" displayName="T"><extLst><ext><x:table xmlns:x="urn:x" name="NewFn" displayName="NewFn"/></ext></extLst></table>"#
    );
    let book =
        cells("", [&["SUM(T)+NewFn"], &[]]).of_s1("table", "/xl/tables/table1.xml", TABLE, &xml);
    assert_eq!(
        judged(book),
        ["/xl/worksheets/sheet1.xml A1: unknown NEWFN x1"]
    );
}

#[test]
fn a_function_the_targets_lack_is_an_error_and_one_best_effort_lacks_a_warning() {
    let names = r#"<definedName name="Pick">_xlfn.LAMBDA(_xlpm.x,_xlfn.XLOOKUP(_xlpm.x,A1:A3,B1:B3))</definedName>"#;
    let book = || {
        cells(
            names,
            [
                &[
                    "_xlfn.XLOOKUP(1,A1:A3,B1:B3)",
                    "_xlfn.XLOOKUP(2,A1:A3,B1:B3)+Pick(3)",
                ],
                &["_xlfn._xlws.FILTER(A1:A3,A1:A3>1)"],
            ],
        )
    };
    let rules = Rules::default()
        .allow_function("XLOOKUP")
        .allow_function("FILTER");
    let mut diagnostics = Diagnostics::new();
    let targets = Targets::parse(
        &["excel >= 2019"],
        &["libreoffice <= 24.8"],
        &mut diagnostics,
    );
    let found = judged_against(book(), &Limits::default(), &rules, &targets.unwrap());
    assert_eq!(
        found,
        [
            "/xl/workbook.xml definedName Pick: targets LAMBDA excel [2019, 2021] [] x1",
            "/xl/workbook.xml definedName Pick: bestEffort LAMBDA libreoffice [24.2, 24.8] [] x1",
            "/xl/workbook.xml definedName Pick: targets XLOOKUP excel [2019] [] x1",
            "/xl/workbook.xml definedName Pick: bestEffort XLOOKUP libreoffice [24.2] [] x1",
            "/xl/worksheets/sheet1.xml A1: targets XLOOKUP excel [2019] [] x2",
            "/xl/worksheets/sheet1.xml A1: bestEffort XLOOKUP libreoffice [24.2] [] x2",
            "/xl/worksheets/sheet1.xml A2 via Pick: targets LAMBDA excel [2019, 2021] [] x1",
            "/xl/worksheets/sheet1.xml A2 via Pick: bestEffort LAMBDA libreoffice [24.2, 24.8] [] x1",
            "/xl/worksheets/sheet1.xml A2 via Pick: targets XLOOKUP excel [2019] [] x1",
            "/xl/worksheets/sheet1.xml A2 via Pick: bestEffort XLOOKUP libreoffice [24.2] [] x1",
            "/xl/worksheets/sheet2.xml A1: targets FILTER excel [2019] [] x1",
            "/xl/worksheets/sheet2.xml A1: bestEffort FILTER libreoffice [24.2] [24.8] x1",
        ]
    );
    assert_eq!(
        judged_under(book(), &Limits::default(), &rules),
        Vec::<String>::new()
    );
}

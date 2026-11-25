//! How `apply` rewrites the formulas of the parts other than the workbook
//! part, and the report it returns: the places each of a rename and a delete
//! rewrites, the ones it only reports, what is kept as it is (a formula that
//! is malformed, one with markup, a table or a pivot source deleted), the
//! parts it does not read or rewrite, a part it cannot read, and `#REF!`.

use super::{NS, R, TABLE, applied_with_report, defined, open_zip, package, package_raw, zip};
use crate::structure::{KeptReason, StructureEdit, StructureReport, sheet_prefix};
use crate::{PackageEdit, WriteLimits};
use std::io::Cursor;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::Package;

pub(super) const CHART: &str = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";
pub(super) const WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
const PIVOT: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotCacheDefinition+xml";
const DRAWING: &str = "application/vnd.openxmlformats-officedocument.drawing+xml";
const REVISIONS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.revisionLog+xml";
const EXTERNAL: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml";
pub(super) const CHART_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";

/// What a report keeps, each as (part, place, at, sheet, reason).
pub(super) type Kept<'a> = Vec<(
    &'a str,
    &'a str,
    Option<&'a str>,
    Option<&'a str>,
    KeptReason,
)>;

pub(super) fn kept(report: &StructureReport) -> Kept<'_> {
    (report.kept().iter())
        .map(|k| (k.part(), k.place(), k.at(), k.sheet(), k.reason()))
        .collect()
}

/// The sheets `Alpha` and `Beta`, the workbook part's `after`, and the parts
/// `extra`.
pub(super) fn book(after: &str, extra: &[(&str, &str, &str)]) -> Package<Cursor<Vec<u8>>> {
    package("", &[("Alpha", ""), ("Beta", "")], after, extra)
}

/// Beta's worksheet with `prefix` before each reference.
fn beta(prefix: &str) -> String {
    format!(
        r#"<worksheet xmlns="{NS}"><sheetData><row r="1"><c r="A1"><f>{prefix}A1+1</f><v>2</v></c><c r="B1"><f t="shared" ref="B1:B2" si="0">SUM({prefix}A1:A3)</f></c></row><row r="2"><c r="B2"><f t="shared" si="0"/></c></row></sheetData><dataValidations count="1"><dataValidation sqref="C1:C9" type="list"><formula1>{prefix}$A$1:$A$3</formula1><formula2>{prefix}$B$1</formula2></dataValidation></dataValidations><conditionalFormatting sqref="D1:D9"><cfRule type="expression" priority="1"><formula>{prefix}A1&gt;0</formula></cfRule></conditionalFormatting><extLst><ext><x14:conditionalFormattings xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"><x14:conditionalFormatting><x14:cfRule type="expression" id="{{1}}"><xm:f>{prefix}A1</xm:f></x14:cfRule><xm:sqref>E1</xm:sqref></x14:conditionalFormatting></x14:conditionalFormattings></ext></extLst></worksheet>"#
    )
}

/// A chart of one series from `Alpha`.
fn chart(prefix: &str) -> String {
    format!(
        r#"<c:chartSpace xmlns:c="{CHART_NS}"><c:chart><c:ser><c:val><c:numRef><c:f>{prefix}$B$2:$B$5</c:f></c:numRef></c:val></c:ser></c:chart></c:chartSpace>"#
    )
}

/// A table with a calculated column and a totals row on `Alpha`.
fn table(prefix: &str) -> String {
    format!(
        r#"<table xmlns="{NS}" name="T1" displayName="T1" ref="A1:B3"><tableColumns count="2"><tableColumn id="1" name="a"><calculatedColumnFormula>{prefix}A1+[@a]</calculatedColumnFormula></tableColumn><tableColumn id="2" name="b"><totalsRowFormula>SUM({prefix}B1:B2)</totalsRowFormula></tableColumn></tableColumns></table>"#
    )
}

/// `text` with each reference the `Alpha!` prefix qualifies (`Alpha!A1`,
/// `Alpha!A1:A3`, `Alpha!$B$1`) replaced by `#REF!` as a whole.
fn ref_for_alpha(text: &str) -> String {
    [
        "Alpha!$A$1:$A$3",
        "Alpha!$B$2:$B$5",
        "Alpha!A1:A3",
        "Alpha!B1:B2",
        "Alpha!$B$1",
        "Alpha!A1",
    ]
    .iter()
    .fold(text.to_owned(), |text, old| text.replace(old, "#REF!"))
}

#[test]
fn a_rename_rewrites_the_formulas_of_every_place_the_edit_rewrites() {
    let (sheet, graph, grid) = (beta("Alpha!"), chart("Alpha!"), table("Alpha!"));
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, sheet.as_str()),
        ("xl/charts/chart1.xml", CHART, graph.as_str()),
        ("xl/tables/table1.xml", TABLE, grid.as_str()),
    ];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("alpha", "New Name").unwrap();
    });
    for (name, expected) in [
        ("/xl/worksheets/sheet2.xml", beta("'New Name'!")),
        ("/xl/charts/chart1.xml", chart("'New Name'!")),
        ("/xl/tables/table1.xml", table("'New Name'!")),
    ] {
        assert_eq!(
            output.part(name).as_deref(),
            Some(expected.as_str()),
            "{name}"
        );
    }
    assert_eq!(kept(&report), []);
}

#[test]
fn a_delete_writes_ref_in_the_formulas_and_the_workbook_is_calculated_on_load() {
    let (sheet, graph, grid) = (beta("Alpha!"), chart("Alpha!"), table("Alpha!"));
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, sheet.as_str()),
        ("xl/charts/chart1.xml", CHART, graph.as_str()),
        ("xl/tables/table1.xml", TABLE, grid.as_str()),
    ];
    let after = r#"<calcPr calcId="1"/>"#;
    let (output, report) = applied_with_report(book(after, &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    for (name, expected) in [
        ("/xl/worksheets/sheet2.xml", ref_for_alpha(&beta("Alpha!"))),
        ("/xl/charts/chart1.xml", ref_for_alpha(&chart("Alpha!"))),
        ("/xl/tables/table1.xml", ref_for_alpha(&table("Alpha!"))),
    ] {
        assert_eq!(
            output.part(name).as_deref(),
            Some(expected.as_str()),
            "{name}"
        );
    }
    let workbook = output.part("/xl/workbook.xml").unwrap();
    assert!(workbook.contains(r#"<calcPr calcId="1" fullCalcOnLoad="1"/>"#));
    assert_eq!(kept(&report), []);
}

#[test]
fn a_rename_does_not_ask_for_a_recalculation_and_a_delete_that_writes_no_ref_does_not_either() {
    let sheet = format!(r#"<worksheet xmlns="{NS}"><c r="A1"><f>Beta!A1</f></c></worksheet>"#);
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let after = r#"<calcPr calcId="1"/>"#;
    let (output, _) = applied_with_report(book(after, &extra), |edit| {
        edit.rename_sheet("Alpha", "Gamma").unwrap();
    });
    let workbook = output.part("/xl/workbook.xml").unwrap();
    assert!(workbook.contains(r#"<calcPr calcId="1"/>"#));
    let (output, _) = applied_with_report(book(after, &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    let workbook = output.part("/xl/workbook.xml").unwrap();
    assert!(workbook.contains(r#"<calcPr calcId="1"/>"#));
}

#[test]
fn a_formula_is_read_across_cdata_references_comments_and_escapes() {
    let sheet = format!(
        r#"<worksheet xmlns="{NS}"><c r="A1"><f><![CDATA[Alpha!A1&B1]]></f></c><c r="A2"><f>Alpha!<!--c-->A2</f></c><c r="A3"><f>_x0041_lpha!A3</f></c><c r="A4"><f> </f></c><c r="A5"><f/></c></worksheet>"#
    );
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    let expected = format!(
        r#"<worksheet xmlns="{NS}"><c r="A1"><f>Delta!A1&amp;B1</f></c><c r="A2"><f>Delta!A2</f></c><c r="A3"><f>Delta!A3</f></c><c r="A4"><f> </f></c><c r="A5"><f/></c></worksheet>"#
    );
    assert_eq!(output.part("/xl/worksheets/sheet2.xml"), Some(expected));
    assert_eq!(kept(&report), []);
}

#[test]
fn a_formula_element_inside_another_is_a_formula_of_its_own() {
    let sheet = format!(
        r#"<worksheet xmlns="{NS}" xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"><dataValidation sqref="A1"><formula1><xm:f>Alpha!A1</xm:f></formula1></dataValidation></worksheet>"#
    );
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert_eq!(
        output.part("/xl/worksheets/sheet2.xml"),
        Some(sheet.replace("Alpha!", "Delta!"))
    );
    assert_eq!(kept(&report), []);
}

#[test]
fn a_pivot_cache_source_is_renamed_and_one_that_is_not_this_workbooks_sheet_is_not() {
    let cache = |sheet: &str| {
        format!(
            r#"<pivotCacheDefinition xmlns="{NS}" xmlns:r="{R}"><cacheSource type="worksheet"><worksheetSource ref="A1:C9" sheet="{sheet}"/></cacheSource><cacheSource><worksheetSource r:id="rId1" sheet="Alpha"/></cacheSource><cacheSource><worksheetSource id="1" r:x="2" sheet="{sheet}"/></cacheSource><cacheSource><worksheetSource name="Table1"/></cacheSource><cacheSource><worksheetSource ref="A1" sheet="Other"/></cacheSource></pivotCacheDefinition>"#
        )
    };
    let input = cache("alpha");
    let extra = [("xl/pivotCache/pivotCacheDefinition1.xml", PIVOT, &input[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Q&A").unwrap();
    });
    assert_eq!(
        output.part("/xl/pivotCache/pivotCacheDefinition1.xml"),
        Some(cache("Q&amp;A"))
    );
    assert_eq!(kept(&report), []);
}

#[test]
fn a_pivot_cache_source_of_a_deleted_sheet_is_kept_as_it_is_and_reported() {
    let cache = format!(
        r#"<pivotCacheDefinition xmlns="{NS}"><cacheSource><worksheetSource ref="A1:C9" sheet="ALPHA"/></cacheSource></pivotCacheDefinition>"#
    );
    let extra = [("xl/pivotCache/pivotCacheDefinition1.xml", PIVOT, &cache[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    assert!(output.holds(&cache));
    assert_eq!(
        kept(&report),
        [(
            "/xl/pivotCache/pivotCacheDefinition1.xml",
            "worksheetSource@sheet",
            None,
            Some("Alpha"),
            KeptReason::DeletedSource,
        )]
    );
}

#[test]
fn the_places_outside_the_list_are_kept_as_they_are_and_reported_when_they_name_the_sheet() {
    let sheet = format!(
        r#"<worksheet xmlns="{NS}"><conditionalFormatting sqref="F1:F3"><cfRule type="dataBar"><dataBar><cfvo type="formula" val="Alpha!$A$1"/><cfvo type="num" val="5"/></dataBar></cfRule></conditionalFormatting></worksheet>"#
    );
    let drawing = r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"><xdr:sp textlink="Alpha!$A$1"/><xdr:sp textlink="Beta!$A$1"/><xdr:pic macro="Alpha!M"/></xdr:wsDr>"#;
    let revisions = format!(
        r#"<revisions xmlns="{NS}"><rcc rId="1" sId="1"><oc r="A1"><oldFormula>Alpha!A1</oldFormula></oc><nc r="A1"><oldFormula>Beta!A1</oldFormula></nc></rcc></revisions>"#
    );
    let external = format!(
        r#"<externalLink xmlns="{NS}"><externalBook><definedNames><definedName name="X" refersTo="Alpha!$A$1"/></definedNames></externalBook></externalLink>"#
    );
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..]),
        ("xl/drawings/drawing1.xml", DRAWING, drawing),
        ("xl/revisions/revisionLog1.xml", REVISIONS, &revisions[..]),
        (
            "xl/externalLinks/externalLink1.xml",
            EXTERNAL,
            &external[..],
        ),
    ];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    for text in [
        sheet.as_str(),
        drawing,
        revisions.as_str(),
        external.as_str(),
    ] {
        assert!(output.holds(text), "{text}");
    }
    fn outside<'a>(
        part: &'a str,
        place: &'a str,
        at: Option<&'a str>,
    ) -> (
        &'a str,
        &'a str,
        Option<&'a str>,
        Option<&'a str>,
        KeptReason,
    ) {
        (part, place, at, Some("Alpha"), KeptReason::Outside)
    }
    assert_eq!(
        kept(&report),
        [
            outside("/xl/worksheets/sheet2.xml", "cfvo@val", Some("F1:F3")),
            outside("/xl/drawings/drawing1.xml", "sp@textlink", None),
            outside("/xl/drawings/drawing1.xml", "pic@macro", None),
            outside("/xl/revisions/revisionLog1.xml", "oldFormula", None),
        ]
    );
}

#[test]
fn a_formula_that_cannot_be_read_whole_or_has_markup_is_kept_when_it_holds_the_renamed_sheets_name()
{
    let sheet = format!(
        r#"<worksheet xmlns="{NS}"><c r="A1"><f>Alpha!A1+"x</f></c><c r="B1"><f>Other!A1+"x</f></c><c r="C1"><f>Alpha!A1<b/>+1</f></c><c r="D1"><f>1+<b/>1</f></c><definedNames><definedName name="Bad">ALPHA!"x</definedName><definedName>Alpha!"x</definedName></definedNames></worksheet>"#
    );
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert!(output.holds(&sheet));
    let part = "/xl/worksheets/sheet2.xml";
    let at = |at: &'static str, reason| (part, "f", Some(at), Some("Alpha"), reason);
    assert_eq!(
        kept(&report),
        [
            at("A1", KeptReason::Malformed),
            at("C1", KeptReason::Markup),
            (
                part,
                "definedName",
                Some("definedName Bad"),
                Some("Alpha"),
                KeptReason::Malformed
            ),
            (
                part,
                "definedName",
                None,
                Some("Alpha"),
                KeptReason::Malformed
            ),
        ]
    );
}

#[test]
fn the_cell_and_the_range_of_a_formula_are_those_of_the_element_it_is_inside() {
    let sheet = format!(
        r#"<worksheet xmlns="{NS}"><c r="A1"><f>1</f></c><f>Alpha!"1</f><dataValidation sqref="B1"><formula1>1</formula1></dataValidation><formula1>Alpha!"2</formula1><c r="Z9"/><f>Alpha!"3</f><dataValidation sqref="Y9"/><formula2>Alpha!"4</formula2><conditionalFormatting sqref="D1:D9"><cfRule><f>Alpha!"5</f></cfRule></conditionalFormatting></worksheet>"#
    );
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (_, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    let part = "/xl/worksheets/sheet2.xml";
    let malformed = |place, at| (part, place, at, Some("Alpha"), KeptReason::Malformed);
    assert_eq!(
        kept(&report),
        [
            malformed("f", None),
            malformed("formula1", None),
            malformed("f", None),
            malformed("formula2", None),
            malformed("f", Some("D1:D9")),
        ]
    );
}

#[test]
fn a_structured_reference_to_a_table_of_a_deleted_sheet_is_kept_and_reported() {
    let sales =
        format!(r#"<table xmlns="{NS}" name="Sales" displayName="SalesTable" ref="A1:B3"/>"#);
    let sheet = format!(
        r#"<worksheet xmlns="{NS}"><c r="C2"><f>SUM(Sales[Amount])+SUM(sales[Tax])</f></c><c r="C3"><f>SUM(Other[x])</f></c><c r="C4"><f>SalesTable[@a]</f></c></worksheet>"#
    );
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{R}/table" Target="../tables/table1.xml"/></Relationships>"#
    );
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..]),
        ("xl/worksheets/_rels/sheet1.xml.rels", "", &rels[..]),
        ("xl/tables/table1.xml", TABLE, &sales[..]),
    ];
    let names = defined(&[("Tot", "", "Sales[Amount]"), ("Fine", "", "Other[x]")]);
    let (output, report) = applied_with_report(book(&names, &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    assert!(output.holds(&sheet));
    let part = "/xl/worksheets/sheet2.xml";
    let table = KeptReason::DeletedTable;
    assert_eq!(
        kept(&report),
        [
            (part, "f", Some("C2"), None, table),
            (part, "f", Some("C4"), None, table),
            (
                "/xl/workbook.xml",
                "definedName",
                Some("definedName Tot"),
                None,
                table,
            ),
        ]
    );
    // Each names the table as its part writes it, whatever the formula spells.
    let tables: Vec<Option<&str>> = report.kept().iter().map(|k| k.table()).collect();
    assert_eq!(tables, [Some("Sales"), Some("SalesTable"), Some("Sales")]);
}

#[test]
fn a_defined_name_of_the_workbook_that_cannot_be_read_whole_is_reported() {
    let names = defined(&[
        ("Bad", "", r#"Alpha!A1+"x"#),
        ("Other", "", r#"Beta!A1+"x"#),
    ]);
    let (_, report) = applied_with_report(book(&names, &[]), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert_eq!(
        kept(&report),
        [(
            "/xl/workbook.xml",
            "definedName",
            Some("definedName Bad"),
            Some("Alpha"),
            KeptReason::Malformed,
        )]
    );
}

#[test]
fn a_rename_then_a_delete_is_applied_in_order_to_a_formula_and_a_report() {
    let sheet = format!(
        r#"<worksheet xmlns="{NS}"><c r="A1"><f>Alpha!A1+Gamma!A1</f></c><c r="A2"><f>Alpha!A2</f></c><conditionalFormatting sqref="B1"><cfRule><dataBar><cfvo val="Alpha!A1"/></dataBar></cfRule></conditionalFormatting></worksheet>"#
    );
    let cache = format!(
        r#"<pivotCacheDefinition xmlns="{NS}"><cacheSource><worksheetSource sheet="Alpha"/></cacheSource></pivotCacheDefinition>"#
    );
    let sheets = [("Alpha", ""), ("Beta", ""), ("Gamma", "")];
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..]),
        ("xl/pivotCache/pivotCacheDefinition1.xml", PIVOT, &cache[..]),
    ];
    let (output, report) = applied_with_report(package("", &sheets, "", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
        edit.delete_sheet("Delta").unwrap();
    });
    let expected = sheet
        .replace("Alpha!A1+Gamma!A1", "#REF!+Gamma!A1")
        .replace("Alpha!A2", "#REF!");
    assert_eq!(output.part("/xl/worksheets/sheet2.xml"), Some(expected));
    assert_eq!(
        kept(&report),
        [
            (
                "/xl/worksheets/sheet2.xml",
                "cfvo@val",
                Some("B1"),
                Some("Alpha"),
                KeptReason::Outside,
            ),
            (
                "/xl/pivotCache/pivotCacheDefinition1.xml",
                "worksheetSource@sheet",
                None,
                Some("Alpha"),
                KeptReason::DeletedSource,
            ),
        ]
    );
}

#[test]
fn a_part_with_no_formula_changed_is_not_written_again() {
    let (untouched, touched) = (beta("Beta!"), chart("Alpha!"));
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, &untouched[..]),
        ("xl/charts/chart1.xml", CHART, &touched[..]),
    ];
    let (output, _) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert!(output.holds(&untouched));
    assert!(!output.holds(&touched));
}

#[test]
fn the_parts_are_read_only_when_a_sheet_was_renamed_or_deleted() {
    let broken = r#"<c:chartSpace xmlns:c="x"><c:f>Alpha!A1"#;
    let extra = [("xl/charts/chart1.xml", CHART, broken)];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.add_sheet("Gamma", None, None).unwrap();
        edit.move_sheet("Beta", 0).unwrap();
    });
    assert!(output.holds(broken));
    assert_eq!(kept(&report), []);
}

#[test]
fn only_the_parts_whose_media_type_is_xml_and_that_are_not_removed_are_read() {
    let sheet = format!(r#"<worksheet xmlns="{NS}"><c r="A1"><f>Alpha!A1</f></c></worksheet>"#);
    let cell = sheet.clone();
    let broken = "<worksheet";
    let extra = [
        // The deleted sheet's part.
        ("xl/worksheets/sheet1.xml", WORKSHEET, broken),
        ("xl/data/a.bin", "application/octet-stream", broken),
        ("xl/data/b.png", "image/png", broken),
        ("xl/data/c.xml", "", &cell[..]),
        ("xl/data/d.xml", "text/xml", &cell[..]),
        ("xl/data/e.xml", "application/vnd.x+xml", &cell[..]),
    ];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    // A part of another kind is read, and its formulas are not written.
    assert!(output.holds(broken) && output.holds(&sheet));
    // The part of the sheet deleted is not read, its XML being broken.
    let (_, report_deleted) = applied_with_report(book("", &extra[..1]), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    assert_eq!(kept(&report_deleted), []);
    // Renamed, it is read and cannot be.
    let outside = |part| (part, "f", Some("A1"), Some("Alpha"), KeptReason::Outside);
    assert_eq!(
        kept(&report),
        [
            (
                "/xl/worksheets/sheet1.xml",
                "",
                None,
                None,
                KeptReason::Unreadable
            ),
            outside("/xl/data/c.xml"),
            outside("/xl/data/d.xml"),
            outside("/xl/data/e.xml"),
        ]
    );
}

/// `zip` with `from` replaced by `to`, of the same length.
fn corrupt(mut zip: Vec<u8>, from: &str, to: &str) -> Vec<u8> {
    let at = zip
        .windows(from.len())
        .position(|window| window == from.as_bytes())
        .unwrap();
    zip[at..at + to.len()].copy_from_slice(to.as_bytes());
    zip
}

/// `zip` with the method of the entry named `name` changed to 12 in both
/// headers.
fn with_method_12(mut zip: Vec<u8>, name: &str) -> Vec<u8> {
    let found = |signature: &[u8], back: usize| {
        let at = (zip.windows(name.len()).enumerate())
            .filter(|(at, window)| *window == name.as_bytes() && *at >= back)
            .map(|(at, _)| at - back)
            .find(|at| zip[*at..].starts_with(signature));
        at.unwrap()
    };
    let local = found(b"PK\x03\x04", 30);
    let central = found(b"PK\x01\x02", 46);
    zip[local + 8] = 12;
    zip[central + 10] = 12;
    zip
}

/// The report `apply` returns for the package of `zip`, the edit not
/// written.
pub(super) fn reported(
    zip: Vec<u8>,
    act: impl FnOnce(&mut StructureEdit<'static>),
) -> StructureReport {
    let mut package = open_zip(zip);
    let limits = WriteLimits::default();
    let mut diagnostics = Diagnostics::new();
    let mut edit = StructureEdit::open(&mut package, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    act(&mut edit);
    let mut package_edit = PackageEdit::new(package);
    edit.apply(&mut package_edit).unwrap()
}

#[test]
fn a_part_that_cannot_be_read_as_xml_to_its_end_is_reported_and_left_as_it_is() {
    let good = chart("Alpha!");
    let cases = [
        // XML that is not well formed, after a formula to rewrite.
        format!("{good}<"),
        // A zip entry whose data does not match its CRC-32.
        format!("{good}<!--MARKER_ONE-->"),
        // A zip entry whose method is not supported.
        good.clone(),
    ];
    for (n, content) in cases.iter().enumerate() {
        let extra = [("xl/charts/chart1.xml", CHART, &content[..])];
        let mut bytes = zip("", &[("Alpha", ""), ("Beta", "")], "", &extra);
        bytes = match n {
            1 => corrupt(bytes, "MARKER_ONE", "MARKER_TWO"),
            2 => with_method_12(bytes, "xl/charts/chart1.xml"),
            _ => bytes,
        };
        let report = reported(bytes, |edit| edit.rename_sheet("Alpha", "Delta").unwrap());
        assert_eq!(
            kept(&report),
            [(
                "/xl/charts/chart1.xml",
                "",
                None,
                None,
                KeptReason::Unreadable
            )],
            "case {n}"
        );
    }
}

#[test]
fn a_defined_name_with_an_empty_name_is_at_the_range_or_nowhere() {
    let sheet = format!(
        r#"<worksheet xmlns="{NS}"><conditionalFormatting sqref="D1"><definedName name="">Alpha!"x</definedName></conditionalFormatting><definedName name="">Alpha!"y</definedName><definedName name="N">Alpha!"z</definedName></worksheet>"#
    );
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (_, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    let part = "/xl/worksheets/sheet2.xml";
    let at = |at| {
        (
            part,
            "definedName",
            at,
            Some("Alpha"),
            KeptReason::Malformed,
        )
    };
    assert_eq!(
        kept(&report),
        [at(Some("D1")), at(None), at(Some("definedName N"))]
    );
}

#[test]
fn a_reported_place_is_kept_when_it_is_malformed_and_names_the_sheet_or_a_deleted_table() {
    let sales = format!(r#"<table xmlns="{NS}" name="Sales" displayName="Sales" ref="A1:B3"/>"#);
    let sheet = format!(
        r#"<worksheet xmlns="{NS}"><conditionalFormatting sqref="F1"><cfRule><dataBar><cfvo val="ALPHA!A1+&quot;x"/><cfvo val="Beta!A1+&quot;x"/><cfvo val="SUM(Sales[Amount])"/><cfvo val="SUM(Other[Amount])"/></dataBar></cfRule></conditionalFormatting></worksheet>"#
    );
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{R}/table" Target="../tables/table1.xml"/></Relationships>"#
    );
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..]),
        ("xl/worksheets/_rels/sheet1.xml.rels", "", &rels[..]),
        ("xl/tables/table1.xml", TABLE, &sales[..]),
    ];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    assert!(output.holds(&sheet));
    let part = "/xl/worksheets/sheet2.xml";
    assert_eq!(
        kept(&report),
        [
            (
                part,
                "cfvo@val",
                Some("F1"),
                Some("Alpha"),
                KeptReason::Malformed
            ),
            (part, "cfvo@val", Some("F1"), None, KeptReason::DeletedTable,),
        ]
    );
}

#[test]
fn a_delete_reports_the_places_outside_the_list_that_name_the_sheet() {
    let drawing = r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"><xdr:sp textlink="Alpha!$A$1"/></xdr:wsDr>"#;
    let revisions = format!(
        r#"<revisions xmlns="{NS}"><rcc rId="1" sId="1"><oc r="A1"><oldFormula>Alpha!A1</oldFormula></oc></rcc></revisions>"#
    );
    let extra = [
        ("xl/drawings/drawing1.xml", DRAWING, drawing),
        ("xl/revisions/revisionLog1.xml", REVISIONS, &revisions[..]),
    ];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    assert!(output.holds(drawing) && output.holds(&revisions));
    let outside = |part, place| (part, place, None, Some("Alpha"), KeptReason::Outside);
    assert_eq!(
        kept(&report),
        [
            outside("/xl/drawings/drawing1.xml", "sp@textlink"),
            outside("/xl/revisions/revisionLog1.xml", "oldFormula"),
        ]
    );
}

#[test]
fn a_pivot_source_of_another_workbook_is_not_reported_when_its_sheet_name_is_deleted() {
    let cache = format!(
        r#"<pivotCacheDefinition xmlns="{NS}" xmlns:r="{R}"><cacheSource><worksheetSource r:id="rId1" sheet="Alpha"/></cacheSource></pivotCacheDefinition>"#
    );
    let extra = [("xl/pivotCache/pivotCacheDefinition1.xml", PIVOT, &cache[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    assert!(output.holds(&cache));
    assert_eq!(kept(&report), []);
}

#[test]
fn character_references_are_kept_in_a_formula_that_does_not_change_and_written_as_text_in_one_that_does()
 {
    let same = r#"IF(A1&gt;0,&quot;x&quot;,1)"#;
    let sheet = |third: &str| {
        format!(
            r#"<worksheet xmlns="{NS}"><c r="A1"><f>{same}</f></c><c r="A2"><f>{third}</f></c></worksheet>"#
        )
    };
    let input = sheet("IF(A1&gt;0,&quot;x&quot;,Alpha!B1)");
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &input[..])];
    let (output, _) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    // The text is written with `&`, `<` and `>` as references and a quote as it is.
    let expected = sheet(r#"IF(A1&gt;0,"x",Delta!B1)"#);
    assert_eq!(output.part("/xl/worksheets/sheet2.xml"), Some(expected));
    let kept_part = sheet("1+1");
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &kept_part[..])];
    let (output, _) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert!(output.holds(same));
}

#[test]
fn a_new_name_that_reads_as_an_escape_or_holds_markup_characters_is_written_escaped_twice() {
    let sheet = format!(r#"<worksheet xmlns="{NS}"><c r="A1"><f>Alpha!A1</f></c></worksheet>"#);
    for name in ["a_x0041_b", "R&D", "A<B>C"] {
        let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
        let (output, _) = applied_with_report(book("", &extra), |edit| {
            edit.rename_sheet("Alpha", name).unwrap();
        });
        // The ST_Xstring escape first, then the XML's.
        let written = format!("{}!A1", sheet_prefix(name, None))
            .replace("_x0041_", "_x005F_x0041_")
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let expected = sheet.replace("Alpha!A1", &written);
        assert_eq!(
            output.part("/xl/worksheets/sheet2.xml"),
            Some(expected),
            "{name}"
        );
    }
}

#[test]
fn a_prefix_of_another_workbook_or_with_a_colon_in_its_last_sheet_is_not_rewritten() {
    let sheet = format!(
        r#"<worksheet xmlns="{NS}"><c r="A1"><f>'Alpha:b:c'!A1</f></c><c r="A2"><f>[1]Alpha!A1</f></c></worksheet>"#
    );
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert!(output.holds(&sheet));
    assert_eq!(kept(&report), []);
}

/// A formula of exactly `len` bytes that names `Alpha`.
pub(super) fn formula_of(len: usize) -> String {
    format!("Alpha!A1{}", "+1".repeat((len - 8) / 2))
}

pub(super) fn beta_with(content: &str) -> String {
    format!(r#"<worksheet xmlns="{NS}">{content}</worksheet>"#)
}

#[test]
fn a_formula_element_outside_the_kinds_of_part_that_are_rewritten_is_reported_not_written() {
    let custom = r#"<root><formula>Alpha!A1</formula></root>"#;
    let revisions = format!(
        r#"<revisions xmlns="{NS}"><rcc rId="1" sId="1"><oc r="A1"><f>Alpha!A1</f></oc></rcc></revisions>"#
    );
    let extra = [
        ("customXml/item1.xml", "", custom),
        ("xl/revisions/revisionLog1.xml", REVISIONS, &revisions[..]),
    ];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    assert!(output.holds(custom) && output.holds(&revisions));
    let outside = |part, place| (part, place, None, Some("Alpha"), KeptReason::Outside);
    assert_eq!(
        kept(&report),
        [
            outside("/customXml/item1.xml", "formula"),
            outside("/xl/revisions/revisionLog1.xml", "f"),
        ]
    );
    let workbook = output.part("/xl/workbook.xml").unwrap();
    assert!(!workbook.contains("fullCalcOnLoad"));
}

#[test]
fn the_formulas_of_the_macrosheet_and_the_dialogsheet_are_rewritten() {
    let sheet = |prefix: &str| beta_with(&format!(r#"<c r="A1"><f>{prefix}A1</f></c>"#));
    let (macros, dialogs) = (
        "application/vnd.ms-excel.macrosheet+xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml",
    );
    let input = sheet("Alpha!");
    let extra = [
        ("xl/macrosheets/sheet1.xml", macros, &input[..]),
        ("xl/dialogsheets/sheet1.xml", dialogs, &input[..]),
    ];
    let (output, _) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    for name in ["/xl/macrosheets/sheet1.xml", "/xl/dialogsheets/sheet1.xml"] {
        assert_eq!(output.part(name), Some(sheet("Delta!")), "{name}");
    }
}

#[test]
fn a_pivot_source_of_another_sheet_is_kept_and_not_reported_and_one_in_another_part_is_not_read() {
    let cache = |sheet: &str| {
        format!(
            r#"<pivotCacheDefinition xmlns="{NS}"><cacheSource><worksheetSource ref="A1" sheet="{sheet}"/></cacheSource></pivotCacheDefinition>"#
        )
    };
    let beta = cache("Beta");
    let custom = r#"<root><worksheetSource sheet="Alpha"/></root>"#;
    let extra = [
        ("xl/pivotCache/pivotCacheDefinition1.xml", PIVOT, &beta[..]),
        ("customXml/item1.xml", "", custom),
    ];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
    });
    assert!(output.holds(&beta) && output.holds(custom));
    assert_eq!(kept(&report), []);
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert!(output.holds(&beta) && output.holds(custom));
    assert_eq!(kept(&report), []);
}

#[test]
fn the_app_part_is_not_read() {
    let app = format!(r#"<Properties xmlns="{NS}"><f>Alpha!A1</f></Properties>"#);
    let workbook = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}"><sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/><sheet name="Beta" sheetId="2" r:id="rId2"/></sheets></workbook>"#
    );
    let rels: String = (1..=2)
        .map(|n| {
            format!(
                r#"<Relationship Id="rId{n}" Type="{R}/worksheet" Target="worksheets/sheet{n}.xml"/>"#
            )
        })
        .collect();
    let app_media = "application/vnd.openxmlformats-officedocument.extended-properties+xml";
    let package = package_raw(
        &workbook,
        &rels,
        2,
        &[("docProps/app.xml", app_media, &app[..])],
        &format!(
            r#"<Relationship Id="rId9" Type="{R}/extended-properties" Target="docProps/app.xml"/>"#
        ),
    );
    let (_, report) = applied_with_report(package, |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert_eq!(kept(&report), []);
}

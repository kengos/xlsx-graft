//! What a workbook part is read to: its sheets, their parts and kinds, and
//! its date system.

use super::super::{NS, STRICT_NS, rel};
use super::{R, WORKSHEET, book_with, clean, read, read_with, sheets, worksheet};
use crate::{DateSystem, ReadLimits, SheetKind, SheetState};

/// The markup compatibility namespace.
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

#[test]
fn a_workbook_as_excel_writes_it_reads_its_sheets_in_order() {
    // The shape of a workbook part Excel 365 writes, with what it adds
    // under `mc:Ignorable` and the `mc:AlternateContent` it writes when it
    // knows where the file was saved.
    let xml = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}" xmlns:mc="{MC}" mc:Ignorable="x15 xr" xmlns:x15="urn:x15" xmlns:xr="urn:xr">
  <fileVersion appName="xl" lastEdited="7" lowestEdited="4" rupBuild="11011"/>
  <workbookPr defaultThemeVersion="124226"/>
  <mc:AlternateContent><mc:Choice Requires="x15"><x15ac:absPath xmlns:x15ac="urn:x15ac" url="C:\"/></mc:Choice></mc:AlternateContent>
  <xr:revisionPtr revIDLastSave="0"/>
  <bookViews><workbookView xWindow="240" activeTab="1"/></bookViews>
  <sheets>
    <sheet name="集計" sheetId="3" r:id="rId2"/>
    <sheet name="Data" sheetId="1" state="hidden" r:id="rId1"/>
  </sheets>
  <definedNames><definedName name="_xlnm.Print_Titles" localSheetId="0">集計!$4:$4</definedName></definedNames>
  <calcPr calcId="191029"/>
  <extLst><ext uri="{{B58B0392}}"><x:any xmlns:x="urn:x"/></ext></extLst>
</workbook>"#
    );
    let related = [
        rel("rId1", WORKSHEET, "worksheets/sheet1.xml"),
        rel("rId2", WORKSHEET, "/xl/worksheets/sheet2.xml"),
    ]
    .concat();
    let items = book_with(&xml, &related, vec![worksheet(1), worksheet(2)]);
    let (part, seen, names) = read_with(items, &ReadLimits::default());
    assert_eq!(seen, vec![]);
    let part = part.unwrap();
    assert_eq!(part.date_system(), DateSystem::Compatibility1900);
    let listed: Vec<_> = (part.sheets().iter())
        .map(|sheet| {
            let at = sheet.part().unwrap();
            let name = names[at.index()].as_str();
            (
                sheet.name(),
                sheet.sheet_id(),
                sheet.state(),
                name,
                at.kind(),
            )
        })
        .collect();
    let worksheet = SheetKind::Worksheet;
    assert_eq!(
        listed,
        vec![
            (
                "集計",
                3,
                SheetState::Visible,
                "/xl/worksheets/sheet2.xml",
                worksheet
            ),
            (
                "Data",
                1,
                SheetState::Hidden,
                "/xl/worksheets/sheet1.xml",
                worksheet
            ),
        ]
    );
}

#[test]
fn a_sheet_has_each_state_and_its_escapes_undone() {
    let body = concat!(
        "<sheets>",
        r#"<sheet name="_x0041_b_x005F_x0041_" sheetId=" 7 " state="veryHidden" r:id="rId1"/>"#,
        r#"<sheet name="" sheetId="4294967295" state="visible" r:id="rId2"/>"#,
        "</sheets>",
    );
    let part = clean(body);
    let [first, second] = part.sheets() else {
        panic!("{part:?}");
    };
    assert_eq!(
        (first.name(), first.sheet_id(), first.state()),
        ("Ab_x0041_", 7, SheetState::VeryHidden)
    );
    assert_eq!(
        (second.name(), second.sheet_id(), second.state()),
        ("", u32::MAX, SheetState::Visible)
    );
}

#[test]
fn each_kind_is_its_relationship_type() {
    let kinds = [
        (
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet",
            SheetKind::Worksheet,
        ),
        (
            "http://purl.oclc.org/ooxml/officeDocument/relationships/worksheet",
            SheetKind::Worksheet,
        ),
        (
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet",
            SheetKind::Chartsheet,
        ),
        (
            "http://purl.oclc.org/ooxml/officeDocument/relationships/chartsheet",
            SheetKind::Chartsheet,
        ),
        (
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/dialogsheet",
            SheetKind::Dialogsheet,
        ),
        (
            "http://purl.oclc.org/ooxml/officeDocument/relationships/dialogsheet",
            SheetKind::Dialogsheet,
        ),
        (
            "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet",
            SheetKind::Macrosheet,
        ),
        (
            "http://schemas.microsoft.com/office/2006/relationships/xlIntlMacrosheet",
            SheetKind::Macrosheet,
        ),
    ];
    for (rel_type, kind) in kinds {
        let xml = format!(
            r#"<workbook xmlns="{NS}" xmlns:r="{R}">{}</workbook>"#,
            sheets(&[("A", "rId1")])
        );
        let related = rel("rId1", rel_type, "worksheets/sheet1.xml");
        let (part, seen) = read(book_with(&xml, &related, vec![worksheet(1)]));
        assert_eq!(seen, vec![], "{rel_type}");
        let part = part.unwrap();
        let sheet = &part.sheets()[0];
        assert_eq!(
            sheet.part().map(|part| part.kind()),
            Some(kind),
            "{rel_type}"
        );
    }
}

#[test]
fn a_strict_workbook_reads_its_own_relationships_namespace() {
    let strict_r = "http://purl.oclc.org/ooxml/officeDocument/relationships";
    let xml = format!(
        // `dateCompatibility` is taken in Strict too, and not read.
        r#"<workbook xmlns="{STRICT_NS}" xmlns:r="{strict_r}" conformance="strict"><workbookPr dateCompatibility="1"/>{}</workbook>"#,
        sheets(&[("A", "rId1")])
    );
    let related = rel(
        "rId1",
        &format!("{strict_r}/worksheet"),
        "worksheets/sheet1.xml",
    );
    let (part, seen) = read(book_with(&xml, &related, vec![worksheet(1)]));
    assert_eq!(seen, vec![]);
    let part = part.unwrap();
    assert_eq!(part.date_system(), DateSystem::Base1900);
    assert_eq!(
        part.sheets()[0].part().map(|part| part.kind()),
        Some(SheetKind::Worksheet)
    );
}

#[test]
fn the_date_system_is_date1904_then_conformance() {
    let a = sheets(&[("A", "rId1")]);
    let cases = [
        ("", "", DateSystem::Compatibility1900),
        (
            "",
            r#"<workbookPr date1904="false"/>"#,
            DateSystem::Compatibility1900,
        ),
        (
            "",
            r#"<workbookPr date1904=" 0 " dateCompatibility="0"/>"#,
            DateSystem::Compatibility1900,
        ),
        ("", r#"<workbookPr date1904="1"/>"#, DateSystem::Base1904),
        (
            "",
            r#"<workbookPr date1904=" true"></workbookPr>"#,
            DateSystem::Base1904,
        ),
        (
            r#" conformance="transitional""#,
            "",
            DateSystem::Compatibility1900,
        ),
        (
            r#" conformance="strict""#,
            "<workbookPr/>",
            DateSystem::Base1900,
        ),
        (
            r#" conformance="strict""#,
            r#"<workbookPr date1904="1"/>"#,
            DateSystem::Base1904,
        ),
    ];
    for (conformance, properties, system) in cases {
        let xml = format!(
            r#"<workbook xmlns="{NS}" xmlns:r="{R}"{conformance}>{properties}{a}</workbook>"#
        );
        let related = rel("rId1", WORKSHEET, "worksheets/sheet1.xml");
        let (part, seen) = read(book_with(&xml, &related, vec![worksheet(1)]));
        assert_eq!(seen, vec![], "{xml}");
        assert_eq!(part.unwrap().date_system(), system, "{xml}");
    }
}

#[test]
fn every_child_of_workbook_may_stand_in_its_order() {
    let body = concat!(
        "<fileVersion/><fileSharing/><workbookPr/><workbookProtection/>",
        "<bookViews/><sheets><sheet name='A' sheetId='1' r:id='rId1'/></sheets>",
        "<functionGroups/><externalReferences/><definedNames/><calcPr/>",
        "<oleSize/><customWorkbookViews/><pivotCaches/><smartTagPr/>",
        "<smartTagTypes/><webPublishing/><fileRecoveryPr/><fileRecoveryPr/>",
        "<webPublishObjects/><extLst/>",
    );
    assert_eq!(clean(body).sheets().len(), 1);
}

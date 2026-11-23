//! `Workbook::read` on workbooks it reads: the sheets, the defined names,
//! the views, `calcPr`, the relationships and the tables. The helpers that
//! build a package in memory are shared with `model_unreadable`.

use crate::WriteLimits;
use crate::structure::model::{BookView, SheetKind, SheetState, Workbook};
use crate::tests::stored;
use std::io::Cursor;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics};
use xlsx_graft_package::{Limits, Package};

/// SpreadsheetML, Transitional.
pub(super) const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
/// The namespace of `r:id`, Transitional.
pub(super) const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// SpreadsheetML, Strict.
const STRICT_NS: &str = "http://purl.oclc.org/ooxml/spreadsheetml/main";
/// The namespace of `r:id`, Strict.
const STRICT_R: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships";
/// The Transitional relationship types start with this.
const TYPE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// The media type of a table part.
pub(super) const TABLE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";
/// The name of the workbook part.
pub(super) const WORKBOOK: &str = "/xl/workbook.xml";
/// The name of the workbook part's Relationships part.
pub(super) const WORKBOOK_RELS: &str = "/xl/_rels/workbook.xml.rels";

/// A relationship of the workbook part to the sheet part `sheet<n>.xml`,
/// of the type `rel_type` (`worksheet` of the Transitional types, else the
/// full type).
pub(super) fn rel(id: &str, rel_type: &str, n: u32) -> String {
    let rel_type = if rel_type.contains("://") {
        rel_type.to_owned()
    } else {
        format!("{TYPE}/{rel_type}")
    };
    format!(r#"<Relationship Id="{id}" Type="{rel_type}" Target="worksheets/sheet{n}.xml"/>"#)
}

/// The Relationships part of the workbook part: `rId1` to `rId8` are
/// worksheets, targeting `sheet1.xml` to `sheet8.xml`.
pub(super) fn rels() -> String {
    let body: String = (1..=8)
        .map(|n| rel(&format!("rId{n}"), "worksheet", n))
        .collect();
    relationships(&body)
}

/// A Relationships part of `body`.
pub(super) fn relationships(body: &str) -> String {
    format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{body}</Relationships>"#
    )
}

/// A Transitional workbook part of `body`.
pub(super) fn workbook(body: &str) -> String {
    format!(r#"<workbook xmlns="{NS}" xmlns:r="{R}">{body}</workbook>"#)
}

/// A workbook part of the one sheet `Sheet1`, then `after`.
pub(super) fn one_sheet(after: &str) -> String {
    workbook(&format!(
        r#"<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>{after}"#
    ))
}

/// The package relationships: `officeDocument`.
pub(super) fn root_rels() -> String {
    relationships(&format!(
        r#"<Relationship Id="rId1" Type="{TYPE}/officeDocument" Target="xl/workbook.xml"/>"#
    ))
}

/// A package, in memory, of the package relationships `root_rels`, the
/// workbook part `workbook`, its Relationships part `rels`, eight
/// worksheets and the parts `extra` (name, media type, content: no
/// override when the media type is empty), read with the diagnostics of
/// the package model left out.
pub(super) fn package(
    root_rels: &str,
    workbook: &str,
    rels: &str,
    extra: &[(&str, &str, &str)],
) -> Package<Cursor<Vec<u8>>> {
    open(bytes(root_rels, workbook, rels, extra))
}

/// The bytes of the zip [`package`] reads.
pub(super) fn bytes(
    root_rels: &str,
    workbook: &str,
    rels: &str,
    extra: &[(&str, &str, &str)],
) -> Vec<u8> {
    let sheets: Vec<(String, String)> = (1..=8)
        .map(|n| {
            (
                format!("xl/worksheets/sheet{n}.xml"),
                "<worksheet/>".to_owned(),
            )
        })
        .collect();
    let overrides: String = (sheets
        .iter()
        .map(|(name, _)| (name.as_str(), "application/vnd.ms-excel.worksheet+xml")))
    .chain(extra.iter().map(|(name, kind, _)| (*name, *kind)))
    .filter(|(_, kind)| !kind.is_empty())
    .map(|(name, kind)| format!(r#"<Override PartName="/{name}" ContentType="{kind}"/>"#))
    .collect();
    let types = format!(
        concat!(
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
            r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
            r#"<Default Extension="xml" ContentType="application/xml"/>"#,
            r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.ms-excel.sheet.main+xml"/>{}"#,
            r#"</Types>"#,
        ),
        overrides
    );
    // The docProps parts are written before the workbook part, so the workbook
    // part's index in Package::parts is above 1.
    let mut entries: Vec<(String, String)> = vec![
        ("[Content_Types].xml".to_owned(), types),
        ("_rels/.rels".to_owned(), root_rels.to_owned()),
        ("docProps/core.xml".to_owned(), "<core/>".to_owned()),
        ("docProps/app.xml".to_owned(), "<Properties/>".to_owned()),
        ("xl/workbook.xml".to_owned(), workbook.to_owned()),
        ("xl/_rels/workbook.xml.rels".to_owned(), rels.to_owned()),
    ];
    entries.extend(sheets);
    entries.extend(
        extra
            .iter()
            .map(|(name, _, content)| ((*name).to_owned(), (*content).to_owned())),
    );
    zip(&entries)
}

/// `entries` as the bytes of a zip, each stored.
pub(super) fn zip(entries: &[(String, String)]) -> Vec<u8> {
    let pairs: Vec<(&[u8], &[u8])> = (entries.iter())
        .map(|(name, content)| (name.as_bytes(), content.as_bytes()))
        .collect();
    stored(&pairs)
}

/// The package in `bytes`, whatever the diagnostics the package model
/// pushes.
pub(super) fn open(bytes: Vec<u8>) -> Package<Cursor<Vec<u8>>> {
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(Cursor::new(bytes), &Limits::default(), &mut diagnostics);
    package.unwrap().unwrap()
}

/// The workbook of the package of `workbook` and the default
/// relationships, and the diagnostics the reading pushed.
pub(super) fn read_workbook(workbook: &str) -> (Option<Workbook>, Vec<String>) {
    let mut package = package(&root_rels(), workbook, &rels(), &[]);
    read(&mut package, &WriteLimits::default())
}

/// What `Workbook::read` gives for `package` under `limits`, and the
/// diagnostics it pushed.
pub(super) fn read(
    package: &mut Package<Cursor<Vec<u8>>>,
    limits: &WriteLimits,
) -> (Option<Workbook>, Vec<String>) {
    let mut diagnostics = Diagnostics::new();
    let workbook = Workbook::read(package, limits, &mut diagnostics).unwrap();
    (workbook, found(&diagnostics))
}

/// The diagnostics' codes with where each points, as
/// `code <entry, else part, else Relationships part>#<relationshipId>`, and
/// the reason and name of a schema problem after it.
pub(super) fn found(diagnostics: &Diagnostics) -> Vec<String> {
    (diagnostics.records().iter())
        .map(|record| {
            let location = &record.location;
            let at = (location.entry.as_ref())
                .or(location.part.as_ref())
                .or(location.rels_part.as_ref())
                .map_or("", |echo| echo.as_str());
            let id = (location.relationship_id.as_ref())
                .map_or(String::new(), |id| format!("#{}", id.as_str()));
            let detail = match &record.diagnostic {
                Diagnostic::XmlSchemaInvalid { reason, name, .. } => {
                    format!(" {reason} {}", name.as_str())
                }
                _ => String::new(),
            };
            format!("{} {at}{id}{detail}", record.diagnostic.code())
        })
        .collect()
}

/// The workbook `workbook` read, which must be read without a diagnostic.
fn clean(workbook: &str) -> Workbook {
    let (workbook, found) = read_workbook(workbook);
    assert_eq!(found, Vec::<String>::new());
    workbook.unwrap()
}

/// The index of the part `name` of `package`.
fn index(package: &Package<Cursor<Vec<u8>>>, name: &str) -> usize {
    (package.parts().iter())
        .position(|part| part.name().as_str() == name)
        .unwrap()
}

#[test]
fn the_sheets_are_read_in_order_with_their_values_part_and_kind() {
    let types = [
        ("worksheet", SheetKind::Worksheet),
        (
            "http://purl.oclc.org/ooxml/officeDocument/relationships/worksheet",
            SheetKind::Worksheet,
        ),
        ("chartsheet", SheetKind::Chartsheet),
        (
            "http://purl.oclc.org/ooxml/officeDocument/relationships/chartsheet",
            SheetKind::Chartsheet,
        ),
        ("dialogsheet", SheetKind::Dialogsheet),
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
    let rels: String = (types.iter().zip(1..))
        .map(|((rel_type, _), n)| rel(&format!("rId{n}"), rel_type, n))
        .collect();
    // The sheets in the reverse of their parts' order, with the ids and
    // states written in each way.
    let body = r#"<sheets>
        <sheet name="H" sheetId="80" r:id="rId8"/>
        <sheet name="G" sheetId=" 7 " state="hidden" r:id="rId7"/>
        <sheet name="F" sheetId="6" state="veryHidden" r:id="rId6"/>
        <sheet name="E" sheetId="5" state="visible" r:id="rId5"/>
        <sheet name="D" sheetId="4" r:id="rId4"/>
        <sheet name="C" sheetId="3" r:id="rId3"/>
        <sheet name="B" sheetId="2" r:id="rId2"/>
        <sheet name="A" sheetId="1" r:id="rId1"/>
    </sheets>"#;
    let mut package = package(&root_rels(), &workbook(body), &relationships(&rels), &[]);
    let (read, found) = read(&mut package, &WriteLimits::default());
    assert_eq!(found, Vec::<String>::new());
    let read = read.unwrap();
    let got: Vec<_> = (read.sheets().iter())
        .map(|sheet| {
            (
                sheet.name().to_owned(),
                sheet.sheet_id(),
                sheet.state(),
                sheet.relationship_id().to_owned(),
                sheet.part(),
                sheet.kind(),
            )
        })
        .collect();
    let part = |n: u32| index(&package, &format!("/xl/worksheets/sheet{n}.xml"));
    let state = [
        SheetState::Visible,
        SheetState::Hidden,
        SheetState::VeryHidden,
        SheetState::Visible,
        SheetState::Visible,
        SheetState::Visible,
        SheetState::Visible,
        SheetState::Visible,
    ];
    let ids = [80, 7, 6, 5, 4, 3, 2, 1];
    let want: Vec<_> = (0..8_usize)
        .map(|i| {
            let n = 8 - u32::try_from(i).unwrap();
            let (_, kind) = types[usize::try_from(n).unwrap() - 1];
            let name = char::from(b'A' + u8::try_from(n).unwrap() - 1).to_string();
            (name, ids[i], state[i], format!("rId{n}"), part(n), kind)
        })
        .collect();
    assert_eq!(got, want);
    assert_eq!(read.part(), index(&package, WORKBOOK));
    assert!(read.part() > 1);
    assert!(!read.strict());
}

#[test]
fn a_sheet_name_has_its_escapes_and_references_undone() {
    let body = r#"<sheets>
        <sheet name="_x0041_b" sheetId="1" r:id="rId1"/>
        <sheet name="a_x005F_x0041_" sheetId="2" r:id="rId2"/>
        <sheet name="R&amp;D&#x20;&#65;" sheetId="3" r:id="rId3"/>
        <sheet name="_xZZZZ_" sheetId="4" r:id="rId4"/>
    </sheets>"#;
    let read = clean(&workbook(body));
    let names: Vec<_> = read.sheets().iter().map(|sheet| sheet.name()).collect();
    assert_eq!(names, ["Ab", "a_x0041_", "R&D A", "_xZZZZ_"]);
}

#[test]
fn a_strict_workbook_is_read_with_its_own_namespaces() {
    let body = format!(
        r#"<workbook xmlns="{STRICT_NS}" xmlns:r="{STRICT_R}">
            <sheets><sheet name="S" sheetId="1" r:id="rId1"/></sheets>
            <definedNames><definedName name="N">S!$A$1</definedName></definedNames>
            <calcPr/>
        </workbook>"#
    );
    let read = clean(&body);
    assert!(read.strict());
    assert_eq!(read.sheets().len(), 1);
    assert_eq!(read.defined_names().len(), 1);
    assert!(read.calc_pr());
}

#[test]
fn a_defined_name_has_every_attribute_in_order_and_its_text_as_written() {
    let body = r#"<sheets><sheet name="S" sheetId="1" r:id="rId1"/></sheets>
        <definedNames xmlns:x="urn:x">
            <definedName name="_x0041_n" localSheetId=" 2 " hidden="1" comment="a &amp; b" x:extra="1" xml:space="preserve" xmlns:y="urn:y" xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">S!$A$1</definedName>
            <definedName name="Empty"/>
            <definedName name="Ref">&lt;&amp;&#x41;<![CDATA[ x<y ]]>z</definedName>
            <x:definedName name="other"/>
            <unknown><definedName name="inside"/></unknown>
            <definedName name="Last">2</definedName>
        </definedNames>
        <x:definedNames xmlns:x="urn:x"><definedName name="foreign"/></x:definedNames>
        <mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">
            <mc:Choice Requires="x"><definedNames><definedName name="alt"/></definedNames></mc:Choice>
        </mc:AlternateContent>"#;
    let read = clean(&workbook(body));
    let names = read.defined_names();
    assert_eq!(names.len(), 4);
    assert_eq!(
        names
            .iter()
            .map(|n| (n.name(), n.local_sheet_id(), n.text()))
            .collect::<Vec<_>>(),
        [
            ("_x0041_n", Some(2), "S!$A$1"),
            ("Empty", None, ""),
            ("Ref", None, "<&A x<y z"),
            ("Last", None, "2"),
        ]
    );
    let attributes: Vec<(&str, &str)> = (names[0].attributes().iter())
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    assert_eq!(
        attributes,
        [
            ("name", "_x0041_n"),
            ("localSheetId", " 2 "),
            ("hidden", "1"),
            ("comment", "a & b"),
            ("x:extra", "1"),
            ("xml:space", "preserve"),
        ]
    );
    assert_eq!(
        names[1].attributes(),
        [("name".to_owned(), "Empty".to_owned())]
    );
}

#[test]
fn the_workbook_views_are_read_with_the_attributes_written() {
    let body = r#"<bookViews>
            <workbookView/>
            <workbookView activeTab="2"/>
            <workbookView firstSheet=" 1 " activeTab="0" xWindow="5"/>
        </bookViews>"#;
    let read = clean(&workbook(&format!(
        r#"{body}<sheets><sheet name="S" sheetId="1" r:id="rId1"/></sheets>"#
    )));
    let views: Vec<(Option<u32>, Option<u32>)> = (read.views().iter())
        .map(|view: &BookView| (view.active_tab(), view.first_sheet()))
        .collect();
    assert_eq!(views, [(None, None), (Some(2), None), (Some(0), Some(1))]);
    assert!(clean(&one_sheet("")).views().is_empty());
}

#[test]
fn calc_pr_is_true_only_for_a_calc_pr_child_of_the_root() {
    assert!(clean(&one_sheet("<calcPr calcId=\"1\"/>")).calc_pr());
    assert!(clean(&one_sheet("<calcPr></calcPr>")).calc_pr());
    for other in [
        "",
        "<workbookPr/><fileVersion/><extLst><ext/></extLst>",
        "<x:calcPr xmlns:x=\"urn:x\"/>",
        "<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\"><mc:Choice Requires=\"r\"><calcPr/></mc:Choice></mc:AlternateContent>",
        "<definedNames><calcPr/></definedNames>",
    ] {
        assert!(!clean(&one_sheet(other)).calc_pr(), "{other}");
    }
}

#[test]
fn the_relationships_are_those_of_the_workbook_part() {
    let rels = relationships(&format!(
        r#"{}<Relationship Id="rId9" Type="{TYPE}/hyperlink" Target="https://example.com/" TargetMode="External"/>"#,
        rel("rId1", "worksheet", 1)
    ));
    let mut package = package(&root_rels(), &one_sheet(""), &rels, &[]);
    let (read, found) = read(&mut package, &WriteLimits::default());
    assert_eq!(found, Vec::<String>::new());
    let read = read.unwrap();
    let part = package.part(WORKBOOK).unwrap();
    assert_eq!(read.relationships(), part.relationships());
    let ids: Vec<&str> = read.relationships().iter().map(|r| r.id()).collect();
    assert_eq!(ids, ["rId1", "rId9"]);
}

#[test]
fn the_package_may_name_the_workbook_part_twice_or_list_an_external_office_document_relationship_first()
 {
    let root = relationships(&format!(
        concat!(
            r#"<Relationship Id="rId1" Type="{t}/officeDocument" Target="https://example.com/" TargetMode="External"/>"#,
            r#"<Relationship Id="rId2" Type="{t}/officeDocument" Target="xl/workbook.xml"/>"#,
            r#"<Relationship Id="rId3" Type="http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument" Target="/xl/workbook.xml"/>"#,
        ),
        t = TYPE
    ));
    let mut package = package(&root, &one_sheet(""), &rels(), &[]);
    let (read, found) = read(&mut package, &WriteLimits::default());
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(read.unwrap().part(), index(&package, WORKBOOK));
}

#[test]
fn every_element_named_table_in_a_table_part_gives_a_table_with_its_names_as_written() {
    let extra = [
        (
            "xl/tables/table1.xml",
            TABLE,
            r#"<table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="Sales" displayName="SalesTable" id="1"><tableColumns/></table>"#,
        ),
        (
            "xl/tables/table2.xml",
            TABLE,
            r#"<table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="OnlyName"/>"#,
        ),
        (
            "xl/tables/table3.xml",
            TABLE,
            r#"<!-- c --><table xmlns="http://purl.oclc.org/ooxml/spreadsheetml/main" displayName="OnlyDisplay"></table>"#,
        ),
        (
            "xl/tables/table4.xml",
            TABLE,
            r#"<other xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="HasNoTableElement"/>"#,
        ),
        (
            "xl/tables/table5.xml",
            TABLE,
            r#"<table xmlns="urn:other" name="NotSpreadsheetML"/>"#,
        ),
        (
            "xl/tables/table8.xml",
            TABLE,
            r#"<table name="NoNamespace"/>"#,
        ),
        (
            "xl/tables/table9.xml",
            TABLE,
            r#"<x:table xmlns:x="urn:x" name="Prefixed" displayName="P"/>"#,
        ),
        (
            "xl/tables/table10.xml",
            TABLE,
            r#"<other><inner><table name="Inner"/></inner><table name="Second" displayName="S"/></other>"#,
        ),
        (
            "xl/tables/table6.xml",
            "application/xml",
            r#"<table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="NotATablePart"/>"#,
        ),
        (
            "xl/tables/table7.xml",
            TABLE,
            r#"<table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="a&amp;b" displayName="c"/>"#,
        ),
    ];
    let mut package = package(&root_rels(), &one_sheet(""), &rels(), &extra);
    let (read, found) = read(&mut package, &WriteLimits::default());
    assert_eq!(found, Vec::<String>::new());
    let read = read.unwrap();
    let got: Vec<(usize, Option<&str>, Option<&str>)> = (read.tables().iter())
        .map(|table| (table.part(), table.name(), table.display_name()))
        .collect();
    let part = |n: u32| index(&package, &format!("/xl/tables/table{n}.xml"));
    assert_eq!(
        got,
        [
            (part(1), Some("Sales"), Some("SalesTable")),
            (part(2), Some("OnlyName"), None),
            (part(3), None, Some("OnlyDisplay")),
            (part(5), Some("NotSpreadsheetML"), None),
            (part(8), Some("NoNamespace"), None),
            (part(9), Some("Prefixed"), Some("P")),
            (part(10), Some("Inner"), None),
            (part(10), Some("Second"), Some("S")),
            (part(7), Some("a&b"), Some("c")),
        ]
    );
    assert!(clean(&one_sheet("")).tables().is_empty());
}

#[test]
fn elements_inside_a_sheet_a_workbook_view_or_calc_pr_are_skipped() {
    let body = r#"<sheets><sheet name="S" sheetId="1" r:id="rId1"><sheet name="in" sheetId="2" r:id="rId2"/></sheet></sheets>
        <bookViews><workbookView activeTab="1"><workbookView activeTab="9"/></workbookView></bookViews>
        <calcPr><calcPr/></calcPr>"#;
    let read = clean(&workbook(body));
    assert_eq!(read.sheets().len(), 1);
    assert_eq!(read.views().len(), 1);
    assert_eq!(read.views()[0].active_tab(), Some(1));
    assert!(read.calc_pr());
}

#[test]
fn as_many_defined_names_as_the_limit_allows_are_read() {
    let names = r#"<definedName name="a"/>"#.repeat(65_536);
    let read = clean(&one_sheet(&format!("<definedNames>{names}</definedNames>")));
    assert_eq!(read.defined_names().len(), 65_536);
}

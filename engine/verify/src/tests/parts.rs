//! The kinds of content a part holds, by its media type, the relationships
//! that target it and its elements, and how the rules judge them.

use super::zip::Item;
use super::{Book, MICROSOFT, WORKSHEET, check, check_with, denied};
use crate::Rules;
use xlsx_graft_diagnostics::{Diagnostic, Echo, EncodingReason, PartKind};

/// A worksheet, `/xl/worksheets/sheet1.xml`, holding `body`, related from
/// the workbook.
fn with_sheet(body: &str) -> Book {
    let ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let xml = format!(r#"<worksheet xmlns="{ns}"><sheetData/>{body}</worksheet>"#);
    Book::new()
        .part("/xl/worksheets/sheet1.xml", WORKSHEET, &xml)
        .rel("/xl/workbook.xml", "worksheet", "/xl/worksheets/sheet1.xml")
}

/// A workbook with a VBA project, as Excel saves an `.xlsm`.
fn with_vba() -> Book {
    Book::new()
        .part(
            "/xl/vbaProject.bin",
            "application/vnd.ms-office.vbaProject",
            "VBA",
        )
        .rel(
            "/xl/workbook.xml",
            &format!("{MICROSOFT}vbaProject"),
            "/xl/vbaProject.bin",
        )
}

#[test]
fn a_workbook_with_nothing_denied_passes() {
    let book = with_sheet("").part(
        "/xl/styles.xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml",
        "<styleSheet/>",
    );
    let book = book.rel("/xl/workbook.xml", "styles", "/xl/styles.xml");
    assert_eq!(check(book), vec![]);
}

#[test]
fn a_vba_project_named_by_its_media_type_and_relationship_is_denied_once() {
    assert_eq!(
        check(with_vba()),
        vec![denied(PartKind::Macro, "/xl/vbaProject.bin")]
    );
}

#[test]
fn a_vba_project_is_denied_by_its_relationship_whatever_its_media_type() {
    let book = Book::new()
        .part("/xl/media/image1.xml", "application/xml", "<x/>")
        .rel(
            "/xl/workbook.xml",
            &format!("{MICROSOFT}vbaProjectSignature"),
            "/xl/media/image1.xml",
        );
    assert_eq!(
        check(book),
        vec![denied(PartKind::Macro, "/xl/media/image1.xml")]
    );
}

#[test]
fn a_vba_project_is_denied_by_its_media_type_whatever_relates_to_it() {
    let book = Book::new()
        .part(
            "/xl/media/image1.png",
            "application/vnd.ms-office.vbaProject",
            "VBA",
        )
        .rel("/xl/workbook.xml", "image", "/xl/media/image1.png");
    assert_eq!(
        check(book),
        vec![denied(PartKind::Macro, "/xl/media/image1.png")]
    );
}

#[test]
fn a_macro_enabled_workbook_and_its_macro_and_dialog_sheets_are_denied() {
    let main = "application/vnd.ms-excel.sheet.macroEnabled.main+xml";
    let book = Book::new()
        .main(main, "<workbook/>")
        .part(
            "/xl/macrosheets/sheet1.xml",
            "application/vnd.ms-excel.macrosheet+xml",
            "<macrosheet/>",
        )
        .rel(
            "/xl/workbook.xml",
            &format!("{MICROSOFT}xlMacrosheet"),
            "/xl/macrosheets/sheet1.xml",
        )
        .part(
            "/xl/macrosheets/intl1.xml",
            "application/vnd.ms-excel.intlmacrosheet+xml",
            "<macrosheet/>",
        )
        .rel(
            "/xl/workbook.xml",
            &format!("{MICROSOFT}xlIntlMacrosheet"),
            "/xl/macrosheets/intl1.xml",
        )
        .part(
            "/xl/dialogsheets/sheet1.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml",
            "<dialogsheet/>",
        )
        .rel(
            "/xl/workbook.xml",
            "dialogsheet",
            "/xl/dialogsheets/sheet1.xml",
        );
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::Macro, "/xl/workbook.xml"),
            denied(PartKind::Macro, "/xl/macrosheets/sheet1.xml"),
            denied(PartKind::Macro, "/xl/macrosheets/intl1.xml"),
            denied(PartKind::Macro, "/xl/dialogsheets/sheet1.xml"),
        ]
    );
}

#[test]
fn an_external_workbook_reference_is_one_external_link_with_its_path() {
    let link = "/xl/externalLinks/externalLink1.xml";
    let book = Book::new()
        .part(
            link,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml",
            r#"<externalLink><externalBook r:id="rId1" xmlns:r="urn:r"/></externalLink>"#,
        )
        .rel("/xl/workbook.xml", "externalLink", link)
        .external(link, "externalLinkPath", "other.xlsx");
    assert_eq!(check(book), vec![denied(PartKind::ExternalLink, link)]);
}

#[test]
fn an_external_reference_holding_a_dde_or_ole_link_is_that_link_alone() {
    let media = "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml";
    let dde = "/xl/externalLinks/externalLink1.xml";
    let ole = "/xl/externalLinks/externalLink2.xml";
    let book = Book::new()
        .part(
            dde,
            media,
            r#"<externalLink><ddeLink ddeService="excel" ddeTopic="x"/></externalLink>"#,
        )
        .rel("/xl/workbook.xml", "externalLink", dde)
        .part(
            ole,
            media,
            r#"<x:externalLink xmlns:x="urn:x"><x:oleLink progId="P"/></x:externalLink>"#,
        )
        .rel("/xl/workbook.xml", "externalLink", ole)
        .external(ole, "oleObject", "file:///C:/object.bin");
    let rels = "/xl/externalLinks/_rels/externalLink2.xml.rels#rId1";
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::Dde, dde),
            denied(PartKind::Ole, ole),
            denied(PartKind::ExternalLink, rels),
        ]
    );
}

#[test]
fn an_external_book_beside_a_dde_link_is_dde_with_its_path_an_external_link() {
    let link = "/xl/externalLinks/externalLink1.xml";
    let book = Book::new()
        .part(
            link,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml",
            r#"<externalLink><externalBook r:id="rId1" xmlns:r="urn:r"/><ddeLink ddeService="cmd" ddeTopic="x"/></externalLink>"#,
        )
        .rel("/xl/workbook.xml", "externalLink", link)
        .external(link, "externalLinkPath", "other.xlsx");
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::Dde, link),
            denied(
                PartKind::ExternalLink,
                "/xl/externalLinks/_rels/externalLink1.xml.rels#rId1"
            ),
        ]
    );
}

#[test]
fn a_worksheet_that_embeds_an_object_holds_ole_whatever_the_object_claims() {
    let book = with_sheet(
        r#"<oleObjects><oleObject progId="Package" r:id="rId1" xmlns:r="urn:r"/></oleObjects>"#,
    )
    .part("/xl/embeddings/x.bin", "application/octet-stream", "BIN")
    .rel(
        "/xl/worksheets/sheet1.xml",
        "urn:misspelled/oleObject",
        "/xl/embeddings/x.bin",
    );
    assert_eq!(
        check(book),
        vec![denied(PartKind::Ole, "/xl/worksheets/sheet1.xml")]
    );
}

#[test]
fn an_external_reference_an_ole_relationship_targets_is_still_an_external_link() {
    let link = "/xl/externalLinks/externalLink1.xml";
    let book = with_sheet("")
        .part(
            link,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml",
            "<externalLink/>",
        )
        .rel("/xl/workbook.xml", "externalLink", link)
        .rel("/xl/worksheets/sheet1.xml", "oleObject", link);
    let rules = Rules::default().allow_part(PartKind::Ole);
    assert_eq!(
        check_with(book, &rules),
        vec![denied(PartKind::ExternalLink, link)]
    );
}

#[test]
fn embedded_objects_and_packages_are_ole() {
    let book = with_sheet("")
        .part(
            "/xl/embeddings/oleObject1.bin",
            "application/vnd.openxmlformats-officedocument.oleObject",
            "OLE",
        )
        .rel(
            "/xl/worksheets/sheet1.xml",
            "oleObject",
            "/xl/embeddings/oleObject1.bin",
        )
        .part(
            "/xl/embeddings/doc.docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "DOCX",
        )
        .rel(
            "/xl/worksheets/sheet1.xml",
            "package",
            "/xl/embeddings/doc.docx",
        )
        .part("/xl/embeddings/other.bin", "application/x-other", "BIN")
        .rel(
            "/xl/worksheets/sheet1.xml",
            "http://purl.oclc.org/ooxml/officeDocument/relationships/oleObject",
            "/xl/embeddings/other.bin",
        );
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::Ole, "/xl/embeddings/oleObject1.bin"),
            denied(PartKind::Ole, "/xl/embeddings/doc.docx"),
            denied(PartKind::Ole, "/xl/embeddings/other.bin"),
        ]
    );
}

#[test]
fn an_activex_control_and_its_binary_are_denied_each() {
    let control = "/xl/activeX/activeX1.xml";
    let binary = "/xl/activeX/activeX1.bin";
    let book = with_sheet("")
        .part(control, "application/vnd.ms-office.activeX+xml", "<ocx/>")
        .rel("/xl/worksheets/sheet1.xml", "control", control)
        .part(binary, "application/vnd.ms-office.activeX", "BIN")
        .rel(control, &format!("{MICROSOFT}activeXControlBinary"), binary);
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::Activex, control),
            denied(PartKind::Activex, binary)
        ]
    );
}

#[test]
fn connections_query_tables_and_power_query_are_data_connections() {
    let book = with_sheet("")
        .part(
            "/xl/connections.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.connections+xml",
            "<connections/>",
        )
        .rel("/xl/workbook.xml", "connections", "/xl/connections.xml")
        .part(
            "/xl/queryTables/queryTable1.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.queryTable+xml",
            "<queryTable/>",
        )
        .rel(
            "/xl/worksheets/sheet1.xml",
            "queryTable",
            "/xl/queryTables/queryTable1.xml",
        )
        .part(
            "/customXml/item1.xml",
            "application/xml",
            r#"<DataMashup xmlns="http://schemas.microsoft.com/DataMashup">AAAA</DataMashup>"#,
        )
        .rel("/xl/workbook.xml", "customXml", "/customXml/item1.xml")
        .part(
            "/xl/xmlMaps.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.xmlMaps+xml",
            r#"<MapInfo><Map ID="1"><DataBinding FileBinding="true" ConnectionID="1"/></Map></MapInfo>"#,
        )
        .rel("/xl/workbook.xml", "xmlMaps", "/xl/xmlMaps.xml");
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::DataConnection, "/xl/connections.xml"),
            denied(PartKind::DataConnection, "/xl/queryTables/queryTable1.xml"),
            denied(PartKind::DataConnection, "/customXml/item1.xml"),
            denied(PartKind::DataConnection, "/xl/xmlMaps.xml"),
        ]
    );
}

#[test]
fn a_power_query_definition_as_excel_writes_it_in_utf_16_is_found() {
    let xml = r#"<?xml version="1.0" encoding="utf-16"?><DataMashup xmlns="http://schemas.microsoft.com/DataMashup">AAAA</DataMashup>"#;
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    let book = Book::new()
        .part("/customXml/item1.xml", "application/xml", "")
        .rel("/xl/workbook.xml", "customXml", "/customXml/item1.xml")
        .replace(Item::stored("customXml/item1.xml", &bytes));
    assert_eq!(
        check(book),
        vec![denied(PartKind::DataConnection, "/customXml/item1.xml")]
    );
}

#[test]
fn a_part_is_read_as_xml_whatever_its_media_type() {
    let padded = format!(
        "{}<worksheet><webPublishItems><webPublishItem/></webPublishItems></worksheet>",
        " ".repeat(9 * 1024)
    );
    let book = with_sheet("")
        .part(
            "/xl/worksheets/sheet2.bin",
            "application/octet-stream",
            &padded,
        )
        .rel("/xl/workbook.xml", "worksheet", "/xl/worksheets/sheet2.bin")
        .part(
            "/xl/worksheets/sheet3.bin",
            "application/octet-stream",
            "<worksheet><webPublishItem/><broken></worksheet>",
        )
        .rel("/xl/workbook.xml", "worksheet", "/xl/worksheets/sheet3.bin");
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::WebPublish, "/xl/worksheets/sheet2.bin"),
            denied(PartKind::WebPublish, "/xl/worksheets/sheet3.bin"),
        ]
    );
}

#[test]
fn a_part_that_is_not_xml_raises_nothing_unless_it_shows_it_is() {
    let vml = r#"<xml xmlns:o="urn:schemas-microsoft-com:office:office"><o:OLEObject/><x:ClientData><br></x:ClientData></xml>"#;
    let book = with_sheet("")
        .part(
            "/xl/media/image1.png",
            "image/png",
            "\u{89}PNG <webPublishItem/>",
        )
        .rel("/xl/worksheets/sheet1.xml", "image", "/xl/media/image1.png")
        .part(
            "/xl/drawings/vmlDrawing1.vml",
            "application/vnd.openxmlformats-officedocument.vmlDrawing",
            vml,
        )
        .rel(
            "/xl/worksheets/sheet1.xml",
            "vmlDrawing",
            "/xl/drawings/vmlDrawing1.vml",
        )
        .part(
            "/xl/media/image2.bin",
            "application/octet-stream",
            r#"<!DOCTYPE x [<!ENTITY e SYSTEM "file:///etc/passwd">]><x/>"#,
        )
        .rel("/xl/worksheets/sheet1.xml", "image", "/xl/media/image2.bin")
        .part(
            "/customXml/item1.bin",
            "application/octet-stream",
            r#"<?xml version="1.0" encoding="windows-1252"?><DataMashup/>"#,
        )
        .rel("/xl/workbook.xml", "customXml", "/customXml/item1.bin")
        .part("/xl/media/image3.emf", "image/x-emf", "\u{1}\0\0\0 EMF")
        .rel("/xl/worksheets/sheet1.xml", "image", "/xl/media/image3.emf")
        .part("/xl/media/image4.png", "image/png", "")
        .rel("/xl/worksheets/sheet1.xml", "image", "/xl/media/image4.png")
        .replace(Item::stored(
            "xl/media/image4.png",
            b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR<webPublishItem/>",
        ))
        .part("/xl/vbaData.bin", "application/octet-stream", "")
        .rel("/xl/worksheets/sheet1.xml", "image", "/xl/vbaData.bin")
        .replace(Item::stored(
            "xl/vbaData.bin",
            b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1",
        ));
    let seen = check(book);
    assert_eq!(seen.len(), 2);
    assert!(matches!(seen[0].0, Diagnostic::XmlDoctype { offset: 0 }));
    assert_eq!(seen[0].1, "/xl/media/image2.bin");
    assert!(matches!(
        seen[1].0,
        Diagnostic::XmlEncodingUnsupported {
            reason: EncodingReason::OtherEncoding,
            ..
        }
    ));
    assert_eq!(seen[1].1, "/customXml/item1.bin");
}

#[test]
fn external_references_and_custom_xml_are_read_whatever_their_media_type() {
    let book = Book::new()
        .part(
            "/xl/externalLinks/externalLink1.xml",
            "application/octet-stream",
            r#"<externalLink><ddeLink ddeService="cmd" ddeTopic="/C calc"/></externalLink>"#,
        )
        .rel(
            "/xl/workbook.xml",
            "externalLink",
            "/xl/externalLinks/externalLink1.xml",
        )
        .part(
            "/customXml/item1.bin",
            "application/octet-stream",
            "<DataMashup>AAAA</DataMashup>",
        )
        .rel(
            "/xl/workbook.xml",
            "http://purl.oclc.org/ooxml/officeDocument/relationships/customXml",
            "/customXml/item1.bin",
        );
    let rules = Rules::default().allow_part(PartKind::ExternalLink);
    assert_eq!(
        check_with(book, &rules),
        vec![
            denied(PartKind::Dde, "/xl/externalLinks/externalLink1.xml"),
            denied(PartKind::DataConnection, "/customXml/item1.bin"),
        ]
    );
}

#[test]
fn web_publishing_is_denied_once_per_part_in_any_namespace() {
    let items = concat!(
        r#"<webPublishItems count="2">"#,
        r#"<webPublishItem id="1" divId="a" sourceType="sheet" destinationFile="\\server\a.htm" autoRepublish="1"/>"#,
        r#"<webPublishItem id="2" divId="b" sourceType="sheet" destinationFile="\\server\b.htm"/>"#,
        "</webPublishItems>",
    );
    let book = with_sheet(items).main(
        "application/xml",
        r#"<w:workbook xmlns:w="urn:w"><w:webPublishObjects><w:webPublishObject id="1" divId="c" destinationFile="c.htm"/></w:webPublishObjects></w:workbook>"#,
    );
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::WebPublish, "/xl/workbook.xml"),
            denied(PartKind::WebPublish, "/xl/worksheets/sheet1.xml"),
        ]
    );
}

#[test]
fn a_part_of_several_kinds_is_reported_once_for_each() {
    let book = with_vba().rel("/xl/workbook.xml", "connections", "/xl/vbaProject.bin");
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::Macro, "/xl/vbaProject.bin"),
            denied(PartKind::DataConnection, "/xl/vbaProject.bin"),
        ]
    );
}

#[test]
fn the_template_allows_a_kind_and_the_policy_forbids_over_it() {
    let allowed = Rules::default().allow_part(PartKind::Macro);
    assert_eq!(check_with(with_vba(), &allowed), vec![]);
    let forbidden = allowed.forbid_part(PartKind::Macro, "no macros here");
    assert_eq!(
        check_with(with_vba(), &forbidden),
        vec![(
            Diagnostic::PartForbidden {
                kind: PartKind::Macro,
                reason: Echo::new("no macros here"),
            },
            String::from("/xl/vbaProject.bin"),
        )]
    );
}

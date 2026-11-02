//! The relationships that point outside the package: a hyperlink to a Web
//! page or a mail address passes, and any other is denied once per part
//! that declares it, at the first one.

use super::{Book, WORKSHEET, check, check_with, denied};
use crate::Rules;
use xlsx_graft_diagnostics::PartKind;

/// The worksheet the links are declared for.
const SHEET: &str = "/xl/worksheets/sheet1.xml";
/// Its Relationships part.
const SHEET_RELS: &str = "/xl/worksheets/_rels/sheet1.xml.rels";

/// A workbook whose worksheet declares a hyperlink to each of `targets`,
/// as `rId1` on, and holds a `hyperlink` element for `rId1` to `rId9`.
fn with_hyperlinks(targets: &[&str]) -> Book {
    sheet_with(targets, "")
}

/// [`with_hyperlinks`], the worksheet holding `more` after its hyperlinks.
fn sheet_with(targets: &[&str], more: &str) -> Book {
    let links: String = (1..=9)
        .map(|n| format!(r#"<hyperlink ref="A{n}" r:id="rId{n}"/>"#))
        .collect();
    let xml =
        format!(r#"<worksheet xmlns:r="urn:r"><hyperlinks>{links}</hyperlinks>{more}</worksheet>"#);
    let book = Book::new()
        .part(SHEET, WORKSHEET, &xml)
        .rel("/xl/workbook.xml", "worksheet", SHEET);
    (targets.iter()).fold(book, |book, target| {
        book.external(SHEET, "hyperlink", target)
    })
}

#[test]
fn hyperlinks_to_web_pages_and_mail_addresses_pass_in_any_letter_case() {
    let book = with_hyperlinks(&[
        "http://example.com/",
        "https://example.com/a?b=c",
        "mailto:someone@example.com",
        "HTTPS://EXAMPLE.COM/",
        "MailTo:someone@example.com",
    ])
    .external(
        SHEET,
        "http://purl.oclc.org/ooxml/officeDocument/relationships/hyperlink",
        "https://example.com/",
    );
    assert_eq!(check(book), vec![]);
}

#[test]
fn a_hyperlink_to_anything_else_is_one_external_link_at_the_first() {
    let book = with_hyperlinks(&[
        "https://example.com/",
        "file:///C:/secret.xlsx",
        r"\\server\share\book.xlsx",
        "ftp://example.com/",
        "http:example.com",
    ]);
    assert_eq!(
        check(book),
        vec![denied(
            PartKind::ExternalLink,
            &format!("{SHEET_RELS}#rId2")
        )]
    );
}

#[test]
fn an_external_relationship_of_another_type_is_an_external_link() {
    let book = Book::new()
        .external("/", "image", "https://example.com/a.png")
        .external("/", "image", "https://example.com/b.png");
    assert_eq!(
        check(book),
        vec![denied(PartKind::ExternalLink, "/_rels/.rels#rId2")]
    );
}

#[test]
fn an_external_relationship_takes_the_kind_its_type_names() {
    let book = with_hyperlinks(&["file:///C:/a.htm"])
        .external(SHEET, "oleObject", "file:///C:/object.xls")
        .external(SHEET, "oleObject", "https://example.com/object.xls");
    assert_eq!(
        check(book),
        vec![
            denied(PartKind::ExternalLink, &format!("{SHEET_RELS}#rId1")),
            denied(PartKind::Ole, &format!("{SHEET_RELS}#rId2")),
        ]
    );
}

#[test]
fn allowing_ole_does_not_let_an_ole_link_reach_outside() {
    let object = "/xl/embeddings/oleObject1.bin";
    let book = with_hyperlinks(&[])
        .part(
            object,
            "application/vnd.openxmlformats-officedocument.oleObject",
            "OLE",
        )
        .rel(SHEET, "oleObject", object)
        .external(object, "oleObject", r"file://\\attacker\share\x.xls")
        .external(SHEET, "oleObject", "file:///C:/linked.xls");
    let rules = Rules::default().allow_part(PartKind::Ole);
    assert_eq!(
        check_with(book, &rules),
        vec![
            denied(PartKind::ExternalLink, &format!("{SHEET_RELS}#rId2")),
            denied(
                PartKind::ExternalLink,
                "/xl/embeddings/_rels/oleObject1.bin.rels#rId1"
            ),
        ]
    );
    assert_eq!(
        check(with_hyperlinks(&[]).external(SHEET, "oleObject", "file:///C:/linked.xls")),
        vec![
            denied(PartKind::ExternalLink, &format!("{SHEET_RELS}#rId1")),
            denied(PartKind::Ole, &format!("{SHEET_RELS}#rId1")),
        ]
    );
}

#[test]
fn a_hyperlink_type_in_another_letter_case_is_no_hyperlink() {
    let book = with_hyperlinks(&[]).external(
        SHEET,
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/HYPERLINK",
        "https://example.com/",
    );
    assert_eq!(
        check(book),
        vec![denied(
            PartKind::ExternalLink,
            &format!("{SHEET_RELS}#rId1")
        )]
    );
}

#[test]
fn a_hyperlink_no_hyperlink_element_uses_is_an_external_link() {
    let book = Book::new()
        .part(
            SHEET,
            WORKSHEET,
            r#"<worksheet><hyperlinks><hyperlink ref="A1" id="rId2"/></hyperlinks></worksheet>"#,
        )
        .rel("/xl/workbook.xml", "worksheet", SHEET)
        .external(SHEET, "hyperlink", "https://example.com/")
        .external(SHEET, "hyperlink", "https://example.com/b");
    assert_eq!(
        check(book),
        vec![denied(
            PartKind::ExternalLink,
            &format!("{SHEET_RELS}#rId1")
        )]
    );
}

#[test]
fn an_ole_link_cannot_borrow_a_hyperlink() {
    let book = sheet_with(
        &["https://example.com/x.doc"],
        r#"<oleObjects><oleObject progId="Word.Document" r:id="rId1"/></oleObjects>"#,
    );
    let rules = Rules::default().allow_part(PartKind::Ole);
    assert_eq!(
        check_with(book, &rules),
        vec![denied(
            PartKind::ExternalLink,
            &format!("{SHEET_RELS}#rId1")
        )]
    );
    let link = "/xl/externalLinks/externalLink1.xml";
    let book = Book::new()
        .part(
            link,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml",
            r#"<externalLink xmlns:r="urn:r"><oleLink r:id="rId1" progId="P"/></externalLink>"#,
        )
        .rel("/xl/workbook.xml", "externalLink", link)
        .external(link, "hyperlink", "http://example.com/x.doc");
    assert_eq!(
        check_with(book, &rules),
        vec![denied(
            PartKind::ExternalLink,
            "/xl/externalLinks/_rels/externalLink1.xml.rels#rId1"
        )]
    );
}

#[test]
fn a_drawing_hyperlink_to_a_web_page_passes() {
    let drawing = "/xl/drawings/drawing1.xml";
    let book = with_hyperlinks(&[])
        .part(
            drawing,
            "application/vnd.openxmlformats-officedocument.drawing+xml",
            r#"<xdr:wsDr xmlns:xdr="urn:x" xmlns:a="urn:a" xmlns:r="urn:r"><a:hlinkClick r:id="rId1"/><a:hlinkHover r:id="rId2"/></xdr:wsDr>"#,
        )
        .rel(SHEET, "drawing", drawing)
        .external(drawing, "hyperlink", "https://example.com/")
        .external(drawing, "hyperlink", "mailto:a@example.com");
    assert_eq!(check(book), vec![]);
}

#[test]
fn a_hyperlink_used_for_anything_else_or_in_a_part_read_part_way_is_an_external_link() {
    let drawing = "/xl/drawings/drawing1.xml";
    let book = with_hyperlinks(&[])
        .part(
            drawing,
            "application/vnd.openxmlformats-officedocument.drawing+xml",
            r#"<xdr:wsDr xmlns:xdr="urn:x" xmlns:a="urn:a" xmlns:r="urn:r"><a:hlinkClick r:id="rId1"/><a:blip r:link="rId1"/></xdr:wsDr>"#,
        )
        .rel(SHEET, "drawing", drawing)
        .external(drawing, "hyperlink", "https://example.com/x.png")
        .part(
            "/xl/other.bin",
            "application/octet-stream",
            r#"<x xmlns:r="urn:r"><hyperlink r:id="rId1"/><broken></x>"#,
        )
        .rel(SHEET, "image", "/xl/other.bin")
        .external("/xl/other.bin", "hyperlink", "https://example.com/");
    assert_eq!(
        check(book),
        vec![
            denied(
                PartKind::ExternalLink,
                "/xl/drawings/_rels/drawing1.xml.rels#rId1"
            ),
            denied(PartKind::ExternalLink, "/xl/_rels/other.bin.rels#rId1"),
        ]
    );
}

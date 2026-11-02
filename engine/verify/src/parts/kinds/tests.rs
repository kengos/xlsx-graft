//! Unit tests for the tables of kinds: letter case, prefixes and what
//! names no kind.

use super::{element, media_type, relationship, web_or_mail_link};
use xlsx_graft_diagnostics::PartKind;

#[test]
fn a_media_type_names_a_kind_by_its_lowered_essence_or_prefix() {
    let cases = [
        (
            "application/vnd.ms-office.vbaprojectsignature",
            Some(PartKind::Macro),
        ),
        (
            "application/vnd.ms-excel.addin.macroenabled.main+xml",
            Some(PartKind::Macro),
        ),
        ("application/vnd.ms-office.activex", Some(PartKind::Activex)),
        (
            "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
            None,
        ),
        (
            "application/vnd.openxmlformats-officedocument.oleobject+xml",
            None,
        ),
        ("image/png", None),
    ];
    for (essence, kind) in cases {
        assert_eq!(media_type(essence), kind, "{essence}");
    }
}

#[test]
fn a_relationship_type_names_a_kind_with_letter_case_aside() {
    let cases = [
        (
            "HTTP://SCHEMAS.OPENXMLFORMATS.ORG/officeDocument/2006/relationships/OLEOBJECT",
            Some(PartKind::Ole),
        ),
        (
            "http://purl.oclc.org/ooxml/officeDocument/relationships/externalLinkPath",
            Some(PartKind::ExternalLink),
        ),
        (
            "http://schemas.microsoft.com/office/2006/relationships/VBAPROJECT",
            Some(PartKind::Macro),
        ),
        (
            "http://schemas.microsoft.com/office/2006/relationships/xlIntlMacrosheet",
            Some(PartKind::Macro),
        ),
        (
            "http://schemas.microsoft.com/office/2006/relationships/ui/extensibility",
            None,
        ),
        (
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
            None,
        ),
        ("urn:other/oleObject", None),
        ("", None),
    ];
    for (rel_type, kind) in cases {
        assert_eq!(relationship(rel_type), kind, "{rel_type}");
    }
}

#[test]
fn an_element_names_a_kind_by_its_local_name_in_its_own_letter_case() {
    assert_eq!(element("webPublishObject"), Some(PartKind::WebPublish));
    assert_eq!(element("WEBPUBLISHITEM"), None);
    assert_eq!(element("worksheet"), None);
}

#[test]
fn only_a_hyperlink_to_a_web_page_or_a_mail_address_is_admitted() {
    let hyperlink = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
    let image = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
    assert!(web_or_mail_link(hyperlink, "https://example.com/"));
    assert!(!web_or_mail_link(hyperlink, " https://example.com/"));
    assert!(!web_or_mail_link(hyperlink, "javascript:alert(1)"));
    assert!(!web_or_mail_link(hyperlink, "http"));
    assert!(!web_or_mail_link(image, "https://example.com/a.png"));
    let shouted = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/Hyperlink";
    assert!(!web_or_mail_link(shouted, "https://example.com/"));
}

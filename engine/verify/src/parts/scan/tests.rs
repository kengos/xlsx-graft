//! Unit tests for which problems say only that a part is not XML.

use super::not_xml;
use xlsx_graft_diagnostics::{Diagnostic, Echo, EncodingReason, Limit};

#[test]
fn malformed_xml_and_a_foreign_encoding_say_a_part_is_not_xml() {
    let malformed = Diagnostic::XmlMalformed {
        offset: 0,
        detail: Echo::new("x"),
    };
    let encoding = Diagnostic::XmlEncodingUnsupported {
        encoding: Echo::new("UTF-16"),
        reason: EncodingReason::Utf16WithoutBom,
    };
    assert!(not_xml(&malformed));
    assert!(not_xml(&encoding));
    for reason in [EncodingReason::InvalidUtf8, EncodingReason::InvalidUtf16] {
        let undecoded = Diagnostic::XmlEncodingUnsupported {
            encoding: Echo::new("UTF-8"),
            reason,
        };
        assert!(not_xml(&undecoded));
    }
}

#[test]
fn an_encoding_named_or_declared_wrongly_says_the_part_is_xml() {
    for reason in [
        EncodingReason::OtherEncoding,
        EncodingReason::DeclarationMismatch,
    ] {
        let encoding = Diagnostic::XmlEncodingUnsupported {
            encoding: Echo::new("windows-1252"),
            reason,
        };
        assert!(!not_xml(&encoding));
    }
    let utf32 = Diagnostic::XmlEncodingUnsupported {
        encoding: Echo::new("UTF-32"),
        reason: EncodingReason::OtherEncoding,
    };
    assert!(not_xml(&utf32));
    let declared = Diagnostic::XmlEncodingUnsupported {
        encoding: Echo::new("utf-32"),
        reason: EncodingReason::OtherEncoding,
    };
    assert!(not_xml(&declared));
}

#[test]
fn a_doctype_and_a_limit_say_more() {
    let large = Diagnostic::XmlEventTooLarge {
        offset: 0,
        size: 17,
        limit: Limit::default_of(16),
    };
    assert!(!not_xml(&large));
    let deep = Diagnostic::XmlTooDeep {
        offset: 0,
        depth: 3,
        limit: Limit::default_of(2),
    };
    assert!(!not_xml(&Diagnostic::XmlDoctype { offset: 0 }));
    assert!(!not_xml(&deep));
}

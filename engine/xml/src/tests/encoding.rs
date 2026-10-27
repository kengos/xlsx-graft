//! Encodings: UTF-8 with or without a byte order mark and UTF-16 with one
//! are read; anything else, a declaration that disagrees with the bytes and
//! bytes that do not decode are `xml_encoding_unsupported`.

use super::{malformed_at, read, utf16};
use xlsx_graft_diagnostics::{Diagnostic, Echo, EncodingReason};

/// The `xml_encoding_unsupported` reading `bytes` ends in.
fn unsupported(bytes: &[u8]) -> (String, EncodingReason) {
    match read(bytes) {
        Err(Diagnostic::XmlEncodingUnsupported { encoding, reason }) => {
            (encoding.as_str().to_owned(), reason)
        }
        other => panic!("expected xml_encoding_unsupported, got {other:?}"),
    }
}

fn expect(encoding: &str, reason: EncodingReason) -> (String, EncodingReason) {
    (Echo::new(encoding).as_str().to_owned(), reason)
}

#[test]
fn utf8_reads_with_or_without_a_byte_order_mark() {
    assert_eq!(read(b"<a/>"), Ok(1));
    assert_eq!(read(b"\xef\xbb\xbf<a/>"), Ok(1));
    assert_eq!(
        read(b"\xef\xbb\xbf<?xml version=\"1.0\" encoding=\"UTF-8\"?><a/>"),
        Ok(2)
    );
}

#[test]
fn utf16_reads_in_either_byte_order_with_a_byte_order_mark() {
    for big in [false, true] {
        assert_eq!(read(&utf16("<a>é</a>", big)), Ok(3));
        let declared = r#"<?xml version="1.0" encoding="UTF-16"?><a/>"#;
        assert_eq!(read(&utf16(declared, big)), Ok(2));
    }
}

#[test]
fn a_declaration_names_its_encoding_in_any_letter_case() {
    assert_eq!(
        read(b"<?xml version=\"1.0\" encoding=\"utf-8\"?><a/>"),
        Ok(2)
    );
    assert_eq!(read(b"<?xml version=\"1.0\"?><a/>"), Ok(2));
}

#[test]
fn a_declaration_naming_another_encoding_is_refused() {
    let utf7 = b"<?xml version=\"1.0\" encoding=\"UTF-7\"?><a/>";
    assert_eq!(
        unsupported(utf7),
        expect("UTF-7", EncodingReason::OtherEncoding)
    );
    let latin1 = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><a/>";
    assert_eq!(
        unsupported(latin1),
        expect("ISO-8859-1", EncodingReason::OtherEncoding)
    );
}

#[test]
fn a_declaration_naming_a_byte_order_is_another_encoding() {
    for (name, big) in [("UTF-16LE", false), ("utf-16be", true)] {
        let bytes = utf16(
            &format!(r#"<?xml version="1.0" encoding="{name}"?><a/>"#),
            big,
        );
        assert_eq!(
            unsupported(&bytes),
            expect(name, EncodingReason::OtherEncoding)
        );
    }
}

#[test]
fn a_declaration_that_disagrees_with_the_bytes_is_refused() {
    let utf8_as_16 = b"<?xml version=\"1.0\" encoding=\"UTF-16\"?><a/>";
    assert_eq!(
        unsupported(utf8_as_16),
        expect("UTF-16", EncodingReason::DeclarationMismatch)
    );
    let utf16_as_8 = utf16(r#"<?xml version="1.0" encoding="UTF-8"?><a/>"#, false);
    assert_eq!(
        unsupported(&utf16_as_8),
        expect("UTF-8", EncodingReason::DeclarationMismatch)
    );
}

#[test]
fn utf32_with_or_without_a_byte_order_mark_is_another_encoding() {
    let starts: [&[u8]; 8] = [
        &[0x00, 0x00, 0xfe, 0xff],
        &[0x00, 0x00, 0xff, 0xfe],
        &[0x00, 0x00, 0x3c, 0x00],
        &[0xff, 0xfe, 0x00, 0x00],
        &[0xfe, 0xff, 0x00, 0x00],
        &[0x00, 0x3c, 0x00, 0x00],
        &[0x00, 0x00, 0x00, 0x3c],
        &[0x3c, 0x00, 0x00, 0x00],
    ];
    for bom in starts {
        let mut bytes = bom.to_vec();
        bytes.extend(b"<\0\0\0");
        assert_eq!(
            unsupported(&bytes),
            expect("UTF-32", EncodingReason::OtherEncoding)
        );
    }
}

#[test]
fn utf16_without_a_byte_order_mark_is_refused() {
    let le: Vec<u8> = "<a/>".encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert_eq!(
        unsupported(&le),
        expect("UTF-16LE", EncodingReason::Utf16WithoutBom)
    );
    let be: Vec<u8> = "<a/>".encode_utf16().flat_map(u16::to_be_bytes).collect();
    assert_eq!(
        unsupported(&be),
        expect("UTF-16BE", EncodingReason::Utf16WithoutBom)
    );
}

#[test]
fn bytes_that_do_not_decode_are_refused() {
    assert_eq!(
        unsupported(b"<a>\xff</a>"),
        expect("UTF-8", EncodingReason::InvalidUtf8)
    );
    let mut odd = utf16("<a/>", false);
    odd.push(0x00);
    assert_eq!(
        unsupported(&odd),
        expect("UTF-16LE", EncodingReason::InvalidUtf16)
    );
    let mut lone = utf16("<a>", true);
    lone.extend([0xd8, 0x00]);
    lone.extend(&utf16("</a>", true)[2..]);
    assert_eq!(
        unsupported(&lone),
        expect("UTF-16BE", EncodingReason::InvalidUtf16)
    );
}

#[test]
fn an_unreadable_encoding_in_the_declaration_is_malformed_at_its_start() {
    let (offset, _) = malformed_at(b"<?xml version=\"1.0\" encoding=UTF-8?><a/>");
    assert_eq!(offset, 0);
}

#[test]
fn a_declaration_after_white_space_is_malformed_whatever_it_names() {
    let (offset, detail) =
        super::malformed_at(b" <?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><a/>");
    assert_eq!(
        (offset, detail.as_str()),
        (1, "the XML declaration is not at the start")
    );
}

#[test]
fn a_second_byte_order_mark_is_content_before_the_root() {
    let outside = "the XML has content outside the root element";
    let (offset, detail) = super::malformed_at(b"\xef\xbb\xbf\xef\xbb\xbf<a/>");
    assert_eq!((offset, detail.as_str()), (0, outside));
    let (offset, detail) = super::malformed_at(&utf16("\u{feff}<a/>", false));
    assert_eq!((offset, detail.as_str()), (0, outside));
}

#[test]
fn characters_of_several_bytes_or_units_read_in_every_encoding() {
    for big in [false, true] {
        assert_eq!(read(&utf16("<a b=\"𝄞\">é𝄞あ</a>", big)), Ok(3));
    }
    assert_eq!(read("<a b=\"𝄞\">é𝄞あ</a>".as_bytes()), Ok(3));
}

#[test]
fn a_character_cut_off_at_the_end_does_not_decode() {
    let mut utf8 = b"<a/>".to_vec();
    utf8.extend(&"あ".as_bytes()[..2]);
    assert_eq!(
        unsupported(&utf8),
        expect("UTF-8", EncodingReason::InvalidUtf8)
    );
    let mut high = utf16("<a/>", false);
    high.extend([0x34, 0xd8]);
    assert_eq!(
        unsupported(&high),
        expect("UTF-16LE", EncodingReason::InvalidUtf16)
    );
}

#[test]
fn a_declaration_read_whole_is_judged_before_the_bytes_after_it() {
    let latin1 = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><a>\xe9</a>";
    assert_eq!(
        unsupported(latin1),
        expect("ISO-8859-1", EncodingReason::OtherEncoding)
    );
    let mut mislabelled = utf16(r#"<?xml version="1.0" encoding="UTF-8"?><a>"#, true);
    mislabelled.extend([0xd8, 0x00]);
    assert_eq!(
        unsupported(&mislabelled),
        expect("UTF-8", EncodingReason::DeclarationMismatch)
    );
}

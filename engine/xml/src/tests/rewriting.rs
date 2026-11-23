//! `XmlRewriter`: the events of a part, written as `next` returned them, give
//! the part's bytes; a UTF-16 part is written as UTF-8.

use super::{chunked, utf16};
use crate::events::{BytesDecl, BytesStart, Event};
use crate::{XmlLimits, XmlRewriter, XmlStreamError, XmlStreamReader};
use std::io::{self, Write};
use xlsx_graft_diagnostics::{Diagnostic, Limit};

/// A part exercising what a rewrite must keep: the declaration's spaces and
/// its single quotes, a newline between attributes, single-quoted values,
/// both empty-element forms, a comment, CDATA, a processing instruction,
/// character and entity references, and whitespace outside the root.
const PART: &str = "<?xml  version='1.0'   encoding=\"UTF-8\"\tstandalone=\"yes\" ?>\r\n\
<!-- a comment -->\n\
<?pi some data?>\n\
<root xmlns=\"urn:r\"\n      b='2'  a=\"1\">\n\
  <x />\n\
  <y/>\n\
  <z a = \"1\"  b='&lt;'>t &amp; &#65; &#x42; &quot;&apos;&gt;</z>\n\
  <![CDATA[<raw> & stuff]]>\n\
  <!--c--><w></w>\n\
</root>\n\n";

/// Reads `bytes` to the end, writing every event as received.
fn rewrite(bytes: &[u8]) -> Vec<u8> {
    let mut rewriter = XmlRewriter::new(bytes, Vec::new(), &XmlLimits::default());
    let mut buf = Vec::new();
    while let Some(event) = rewriter.next(&mut buf).unwrap() {
        rewriter.write(event).unwrap();
    }
    rewriter.into_inner()
}

#[test]
fn a_part_written_as_read_is_the_same_bytes() {
    assert_eq!(rewrite(PART.as_bytes()), PART.as_bytes());
}

#[test]
fn a_utf8_byte_order_mark_is_written_back() {
    let with_bom = [b"\xef\xbb\xbf".as_slice(), PART.as_bytes()].concat();
    assert_eq!(rewrite(&with_bom), with_bom);
    // Without a declaration, the mark is still the first thing written.
    assert_eq!(rewrite(b"\xef\xbb\xbf<a/>"), b"\xef\xbb\xbf<a/>");
    assert_eq!(rewrite(b"<a/>"), b"<a/>");
    // The mark is known only once four bytes are read, so a part handed out
    // a byte at a time is sniffed after several reads.
    for bytes in [with_bom.as_slice(), b"\xef\xbb\xbf<a/>"] {
        let mut rewriter = XmlRewriter::new(chunked(bytes, 1), Vec::new(), &XmlLimits::default());
        let mut buf = Vec::new();
        while let Some(event) = rewriter.next(&mut buf).unwrap() {
            rewriter.write(event).unwrap();
        }
        assert_eq!(rewriter.into_inner(), bytes);
    }
}

#[test]
fn a_utf16_part_is_written_as_utf8_with_a_utf8_declaration() {
    let part = "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"yes\"?>\n<a b=\"é𝄞\"/>";
    let expected = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<a b=\"é𝄞\"/>";
    for big in [false, true] {
        assert_eq!(
            rewrite(&utf16(part, big)),
            expected.as_bytes(),
            "big: {big}"
        );
    }
    // `utf-16` in lower case is also written as `UTF-8`.
    let lower = utf16("<?xml version=\"1.0\" encoding=\"utf-16\"?><a/>", false);
    assert_eq!(
        rewrite(&lower),
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><a/>"
    );
}

#[test]
fn a_utf8_declaration_is_written_as_it_was_in_any_letter_case() {
    let part = b"<?xml version='1.0'  encoding='utf-8'?><a/>";
    assert_eq!(rewrite(part), part);
}

#[test]
fn a_declaration_naming_no_encoding_in_a_utf16_part_is_written_unchanged() {
    // A declaration that names no encoding, with or without a version, is
    // right for the UTF-8 output, so it is written as it was.
    for part in [
        "<?xml  version=\"1.0\" ?><a/>",
        "<?xml standalone=\"yes\"?><a/>",
    ] {
        assert_eq!(rewrite(&utf16(part, false)), part.as_bytes(), "{part}");
    }
}

#[test]
fn a_declaration_in_a_utf16_part_gets_only_the_value_of_its_encoding_replaced() {
    // Single quotes, a pseudo-attribute, and spaces; no version; values with
    // a double quote, which `BytesDecl::new` would write inside double quotes
    // without escaping.
    for (declaration, expected) in [
        (
            r#"<?xml version='1.0'  encoding='utf-16' standalone='yes' foo="bar" ?>"#,
            r#"<?xml version='1.0'  encoding='UTF-8' standalone='yes' foo="bar" ?>"#,
        ),
        (
            r#"<?xml encoding="UTF-16"?>"#,
            r#"<?xml encoding="UTF-8"?>"#,
        ),
        (
            r#"<?xml version='1"0' encoding='UTF-16' standalone='y"s'?>"#,
            r#"<?xml version='1"0' encoding='UTF-8' standalone='y"s'?>"#,
        ),
    ] {
        let part = utf16(&format!("{declaration}<a/>"), false);
        let written = rewrite(&part);
        assert_eq!(written, format!("{expected}<a/>").as_bytes());
        // What is written is a part the reader reads to its end.
        let mut reader = XmlStreamReader::new(written.as_slice(), &XmlLimits::default());
        let mut buf = Vec::new();
        while !matches!(reader.read_event_into(&mut buf).unwrap(), Event::Eof) {}
    }
}

#[test]
fn a_declaration_the_caller_built_with_an_unreadable_attribute_is_refused_by_write() {
    // An encoding whose value is not quoted, and an attribute before the
    // encoding whose value is not quoted.
    for content in [
        "xml version=\"1.0\" encoding=UTF-16",
        "xml version=1.0 encoding=\"UTF-16\"",
    ] {
        let start = BytesStart::from_content(content, 3);
        let mut rewriter = XmlRewriter::new(b"".as_slice(), Vec::new(), &XmlLimits::default());
        let error = rewriter
            .write(Event::Decl(BytesDecl::from_start(start)))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(rewriter.into_inner().is_empty());
    }
}

#[test]
fn replacing_one_element_leaves_the_rest_byte_for_byte() {
    let part = "<?xml version=\"1.0\"?>\n<r>\n  <a x='1'\n     y=\"2\"/>\n  <b />\n</r>\n";
    let mut rewriter = XmlRewriter::new(part.as_bytes(), Vec::new(), &XmlLimits::default());
    let mut buf = Vec::new();
    while let Some(event) = rewriter.next(&mut buf).unwrap() {
        match event {
            Event::Empty(element) if element.name().as_ref() == "a" => {
                let mut replacement = BytesStart::new("c");
                replacement.push_attribute(("k", "v"));
                rewriter.write(Event::Empty(replacement)).unwrap();
            }
            event => rewriter.write(event).unwrap(),
        }
    }
    let expected = "<?xml version=\"1.0\"?>\n<r>\n  <c k=\"v\"/>\n  <b />\n</r>\n";
    assert_eq!(rewriter.into_inner(), expected.as_bytes());
}

#[test]
fn space_after_the_name_in_a_closing_tag_is_not_written() {
    // quick-xml's reader trims it from the name, so the one form the
    // rewrite does not keep is `</a >`.
    assert_eq!(rewrite(b"<a>t</a >"), b"<a>t</a>");
    assert_eq!(rewrite(b"<a>t</a\n>"), b"<a>t</a>");
}

#[test]
fn a_problem_in_the_part_is_returned_by_next_and_by_every_later_call() {
    let mut rewriter = XmlRewriter::new(
        b"<!DOCTYPE a><a/>".as_slice(),
        Vec::new(),
        &XmlLimits::default(),
    );
    let mut buf = Vec::new();
    for _ in 0..2 {
        let Err(XmlStreamError::Invalid(diagnostic)) = rewriter.next(&mut buf) else {
            panic!("expected the DOCTYPE to be refused");
        };
        assert!(matches!(diagnostic, Diagnostic::XmlDoctype { .. }));
    }
    assert!(rewriter.into_inner().is_empty());
}

#[test]
fn the_limits_of_the_reader_apply() {
    let limits = XmlLimits {
        max_depth: Limit::default_of(2),
        ..XmlLimits::default()
    };
    let mut rewriter = XmlRewriter::new(b"<a><b><c/></b></a>".as_slice(), Vec::new(), &limits);
    let mut buf = Vec::new();
    let mut seen = 0;
    let error = loop {
        match rewriter.next(&mut buf) {
            Ok(Some(event)) => {
                rewriter.write(event).unwrap();
                seen += 1;
            }
            Ok(None) => panic!("expected xml_too_deep"),
            Err(error) => break error,
        }
    };
    assert!(matches!(
        error,
        XmlStreamError::Invalid(Diagnostic::XmlTooDeep { .. })
    ));
    assert_eq!(seen, 2);
    assert_eq!(rewriter.into_inner(), b"<a><b>");
}

/// A writer that fails every write.
struct Failing;

impl Write for Failing {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("the host failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_failure_of_the_output_is_returned_by_write() {
    let mut rewriter = XmlRewriter::new(b"<a/>".as_slice(), Failing, &XmlLimits::default());
    let mut buf = Vec::new();
    let event = rewriter.next(&mut buf).unwrap().unwrap();
    assert_eq!(
        rewriter.write(event).unwrap_err().to_string(),
        "the host failed"
    );
}

/// A writer that fails its first write and accepts every later one.
struct FailsFirst(bool);

impl Write for FailsFirst {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0 {
            return Ok(bytes.len());
        }
        self.0 = true;
        Err(io::Error::other("the host failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_failure_to_write_the_byte_order_mark_is_returned_by_write() {
    // The first write is the mark; the event's own write would succeed.
    let part = b"\xef\xbb\xbf<a/>".as_slice();
    let mut rewriter = XmlRewriter::new(part, FailsFirst(false), &XmlLimits::default());
    let mut buf = Vec::new();
    let event = rewriter.next(&mut buf).unwrap().unwrap();
    assert_eq!(
        rewriter.write(event).unwrap_err().to_string(),
        "the host failed"
    );
}

#[test]
fn a_utf8_declaration_without_a_version_is_read_and_written_unchanged() {
    let part = b"<?xml encoding=\"UTF-8\"?><a/>";
    assert_eq!(rewrite(part), part);
}

#[test]
fn get_ref_gives_the_source_that_failed() {
    let mut source = chunked(b"<a/>", 1);
    source.get_mut().fail_at = Some(1);
    let mut rewriter = XmlRewriter::new(source, Vec::new(), &XmlLimits::default());
    let mut buf = Vec::new();
    assert!(matches!(
        rewriter.next(&mut buf),
        Err(XmlStreamError::Io(_))
    ));
    assert!(rewriter.get_ref().get_ref().reads > 0);
}

#[test]
fn the_names_resolve_and_the_offset_is_the_readers() {
    let part = b"<p:a xmlns:p=\"urn:p\" p:k=\"v\"><p:b/></p:a>";
    let mut rewriter = XmlRewriter::new(part.as_slice(), Vec::new(), &XmlLimits::default());
    let mut buf = Vec::new();
    let Some(Event::Start(a)) = rewriter.next(&mut buf).unwrap() else {
        panic!("expected a start tag");
    };
    let (namespace, local) = rewriter.resolve_element(a.name());
    assert_eq!(
        namespace,
        crate::name::ResolveResult::Bound(crate::name::Namespace("urn:p"))
    );
    assert_eq!(local.as_ref(), "a");
    let (namespace, local) = rewriter.resolve_attribute(crate::name::QName("p:k"));
    assert_eq!(
        namespace,
        crate::name::ResolveResult::Bound(crate::name::Namespace("urn:p"))
    );
    assert_eq!(local.as_ref(), "k");
    assert_eq!(rewriter.resolve_prefix("p"), Some("urn:p"));
    assert_eq!(rewriter.resolve_prefix("q"), None);
    assert_eq!(rewriter.offset(), 0);
    let event = rewriter.next(&mut buf).unwrap().unwrap();
    rewriter.write(event).unwrap();
    assert_eq!(rewriter.offset(), 29);
}

#[test]
fn a_rewriter_prints_its_reader_and_whether_it_has_written() {
    let rewriter = XmlRewriter::new(b"<a/>".as_slice(), Vec::new(), &XmlLimits::default());
    let printed = format!("{rewriter:?}");
    assert!(printed.starts_with("XmlRewriter { reader: "), "{printed}");
    assert!(printed.contains("started: false"), "{printed}");
}

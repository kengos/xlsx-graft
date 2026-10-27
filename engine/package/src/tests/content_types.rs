//! Media types from the content types stream: `Default` and `Override`,
//! which takes precedence, refused and duplicate mappings, parts with no
//! type or the wrong kind, the media type grammar, and a stream that is
//! missing or cannot be read.

use super::builder::Item;
use super::opc::{
    DEFAULTS, RELS_TYPE, at, clean, found, package_rels, part, read_with, rel, types,
};
use super::{Reader, limit};
use crate::{ContentType, Limits, Package, PackageError, ZipArchive, ZipLimits};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, PartNameReason, SchemaReason};

/// The content types stream with `DEFAULTS` and `more`.
fn types_with(more: &str) -> Item {
    types(&format!("{DEFAULTS}{more}"))
}

/// A package of `stream` as its content types, with the package
/// relationships, the workbook and `more`.
fn package(stream: Item, more: Vec<Item>) -> Vec<Item> {
    let mut items = vec![stream, package_rels(""), part("xl/workbook.xml")];
    items.extend(more);
    items
}

/// Where a diagnostic about the content types stream points.
const STREAM: &str = "[Content_Types].xml";

/// `xml_schema_invalid` at `offset`.
fn schema(offset: u64, reason: SchemaReason, name: &str) -> Diagnostic {
    Diagnostic::XmlSchemaInvalid {
        offset,
        reason,
        name: Echo::new(name),
    }
}

#[test]
fn an_override_takes_precedence_over_a_default_matched_in_any_case() {
    let stream = types(&format!(
        r#"<Default Extension="XmL" ContentType="application/xml"/>
           <Default Extension="RELS" ContentType="{RELS_TYPE}"/>
           <Override PartName="/XL/WORKBOOK.xml" ContentType="Application/Vnd.Main+XML; v=1"/>"#
    ));
    let package = clean(package(stream, vec![]));
    let workbook = package
        .part("/xl/workbook.xml")
        .unwrap()
        .content_type()
        .unwrap();
    assert_eq!(workbook.as_str(), "Application/Vnd.Main+XML; v=1");
    assert_eq!(workbook.essence(), "application/vnd.main+xml");
    let rels = package
        .part("/_rels/.rels")
        .unwrap()
        .content_type()
        .unwrap();
    assert_eq!(rels.essence(), RELS_TYPE);
}

#[test]
fn a_part_with_no_type_is_reported_but_a_relationships_part_is_not() {
    let stream = types(r#"<Default Extension="xml" ContentType="application/xml"/>"#);
    let seen = found(package(stream, vec![part("a.bin"), part("noext")]));
    let missing = Diagnostic::PartContentTypeMissing {};
    let unreferenced = Diagnostic::PartUnreferenced {};
    assert_eq!(
        seen,
        vec![
            at(missing.clone(), "/a.bin"),
            at(missing, "/noext"),
            at(unreferenced.clone(), "/a.bin"),
            at(unreferenced, "/noext"),
        ]
    );
}

#[test]
fn a_type_that_does_not_fit_the_name_is_a_mismatch_either_way() {
    let stream = types_with(&format!(
        r#"<Override PartName="/_rels/.rels" ContentType="application/xml"/>
           <Override PartName="/xl/workbook.xml" ContentType="{RELS_TYPE}"/>"#
    ));
    let mismatch = |content_type: &str| Diagnostic::PartContentTypeMismatch {
        content_type: Echo::new(content_type),
    };
    assert_eq!(
        found(package(stream, vec![part("x.bin").with_method(12)])),
        vec![
            at(
                Diagnostic::ZipUnsupportedCompression { method: 12 },
                "x.bin"
            ),
            at(mismatch("application/xml"), "/_rels/.rels"),
            at(mismatch(RELS_TYPE), "/xl/workbook.xml"),
        ]
    );
}

#[test]
fn duplicate_and_refused_mappings_are_reported_once_and_type_nothing() {
    let stream = types_with(
        r#"<Default Extension="XML" ContentType="text/xml"/>
           <Override PartName="/a.bin" ContentType="a/b"/>
           <Override PartName="/A.BIN" ContentType="a/b"/>
           <Override PartName="/c.bin" ContentType="a /b"/>
           <Override PartName="c.bin" ContentType="a/b"/>
           <Override PartName="/%41.bin" ContentType="a/b"/>"#,
    );
    let seen = found(package(stream, vec![part("a.bin"), part("c.bin")]));
    let unreferenced = Diagnostic::PartUnreferenced {};
    assert_eq!(
        seen,
        vec![
            at(
                Diagnostic::ContentTypesDuplicateDefault {
                    extension: Echo::new("XML"),
                },
                STREAM
            ),
            at(
                Diagnostic::ContentTypesDuplicateOverride {
                    part_name: Echo::new("/A.BIN"),
                },
                STREAM
            ),
            at(
                Diagnostic::ContentTypeInvalid {
                    content_type: Echo::new("a /b"),
                },
                STREAM
            ),
            at(
                Diagnostic::ContentTypesPartNameInvalid {
                    part_name: Echo::new("c.bin"),
                    reason: PartNameReason::MissingLeadingSlash,
                },
                STREAM
            ),
            at(
                Diagnostic::ContentTypesPartNameInvalid {
                    part_name: Echo::new("/%41.bin"),
                    reason: PartNameReason::EncodedUnreserved,
                },
                STREAM
            ),
            at(unreferenced.clone(), "/a.bin"),
            at(unreferenced, "/c.bin"),
        ]
    );
}

#[test]
fn a_mapping_missing_an_attribute_or_with_a_bad_extension_is_left_out() {
    let stream = types_with(
        r#"<Default Extension="bin"/>
           <Override ContentType="a/b"/>
           <Default Extension=".bin" ContentType="a/b"/>
           <Default Extension="" ContentType="a/b"/>
           <Default Extension="b%4" ContentType="a/b"/>
           <Default Extension="!$&amp;'()*+,:=@-_~%4a" ContentType="a/b"/>"#,
    );
    let seen = found(package(stream, vec![part("a.bin")]));
    let offsets: Vec<u64> = seen
        .iter()
        .filter_map(|(d, _)| match d {
            Diagnostic::XmlSchemaInvalid { offset, .. } => Some(*offset),
            _ => None,
        })
        .collect();
    assert_eq!(
        seen,
        vec![
            at(
                schema(offsets[0], SchemaReason::MissingAttribute, "ContentType"),
                STREAM
            ),
            at(
                schema(offsets[1], SchemaReason::MissingAttribute, "PartName"),
                STREAM
            ),
            at(
                schema(offsets[2], SchemaReason::InvalidValue, "Extension"),
                STREAM
            ),
            at(
                schema(offsets[3], SchemaReason::InvalidValue, "Extension"),
                STREAM
            ),
            at(
                schema(offsets[4], SchemaReason::InvalidValue, "Extension"),
                STREAM
            ),
            // The mapping missing its `PartName` may have typed it.
            at(Diagnostic::PartUnreferenced {}, "/a.bin"),
        ]
    );
    assert!(offsets.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn media_types_follow_rfc_7231_without_white_space_or_comments() {
    for value in [
        "a/b",
        "application/vnd.ms-excel.sheet.macroEnabled.main+xml",
        "a/b;c=d",
        "a/b ; c=d;\te=\"f \\\" g\u{e9}\"",
        "!#$%&'*+-.^_`|~0/z",
    ] {
        assert!(ContentType::parse(value).is_some(), "{value}");
    }
    for value in [
        "",
        "a",
        "a/",
        "/b",
        "a /b",
        "a/ b",
        "a/b ",
        " a/b",
        "a/b;",
        "a/b;c",
        "a/b;c =d",
        "a/b;c= d",
        "a/b;c=\"d",
        "a/b;c=\"\x7f\"",
        "a/b;c=\"\\\x7f\"",
        "a/b;c=\"\\",
        "a/b(comment)",
        "a/b;c=d e",
        "é/b",
        "a/b;c=\"中\"",
    ] {
        assert!(ContentType::parse(value).is_none(), "{value:?}");
    }
}

#[test]
fn a_missing_stream_is_reported_once_and_types_no_part() {
    let seen = found(vec![
        package_rels(""),
        part("xl/workbook.xml"),
        part("a.bin"),
    ]);
    assert_eq!(
        seen,
        vec![
            at(Diagnostic::ContentTypesMissing {}, ""),
            at(Diagnostic::PartUnreferenced {}, "/a.bin"),
        ]
    );
}

#[test]
fn a_stream_that_cannot_be_read_types_no_part_and_reports_nothing_more() {
    let malformed = found(package(
        Item::stored(STREAM, b"<Types"),
        vec![part("a.bin")],
    ));
    assert!(matches!(&malformed[0], (Diagnostic::XmlMalformed { .. }, at) if at == STREAM));
    assert_eq!(malformed.len(), 2);
    let refused = found(package(
        types(DEFAULTS).with_method(12),
        vec![part("a.bin")],
    ));
    assert_eq!(
        refused[0],
        at(Diagnostic::ZipUnsupportedCompression { method: 12 }, STREAM)
    );
    assert_eq!(refused.len(), 2);
    let mut damaged = types(DEFAULTS);
    damaged.local.crc32 = 0;
    damaged.central.crc32 = 0;
    let damaged = found(package(damaged, vec![part("a.bin")]));
    assert!(matches!(&damaged[0], (Diagnostic::ZipCrcMismatch { .. }, at) if at == STREAM));
    assert_eq!(damaged.len(), 2);
}

#[test]
fn an_xml_stream_is_held_to_the_size_limit_before_it_is_read() {
    let stream = types_with("");
    let rels = package_rels(&rel("rId2", "xl/workbook.xml"));
    let (size, rels_size) = (stream.central.size, rels.central.size);
    assert!(rels_size > size);
    let items = vec![stream, rels, part("xl/workbook.xml")];
    let mut limits = Limits::default();
    // The stream at the limit is read; the package relationships past it
    // are not, so whether a main document or an unreferenced part is
    // there is not known.
    limits.xml.max_bytes = limit(size);
    let (_, seen) = read_with(items.clone(), &limits);
    let too_large = Diagnostic::XmlTooLarge {
        size: rels_size,
        limit: limit(size),
    };
    assert_eq!(seen, vec![at(too_large, "/_rels/.rels")]);
    limits.xml.max_bytes = limit(size - 1);
    let (_, seen) = read_with(items, &limits);
    let too_large = Diagnostic::XmlTooLarge {
        size,
        limit: limit(size - 1),
    };
    assert_eq!(seen[0], at(too_large, STREAM));
}

#[test]
fn a_failure_of_the_host_while_a_stream_inflates_is_an_error() {
    let bytes = super::builder::Zip::of(package(types_with(""), vec![])).build();
    let mut diagnostics = Diagnostics::new();
    let archive = ZipArchive::open(
        Reader::new(bytes.clone()),
        &ZipLimits::default(),
        &mut diagnostics,
    );
    let data = archive.unwrap().unwrap().entries()[0].data_range();
    let reader = Reader::poisoned(bytes, data);
    let error = Package::open(reader, &Limits::default(), &mut diagnostics).unwrap_err();
    assert!(matches!(error, PackageError::Io(_)));
}

#[test]
fn a_zip_that_cannot_be_read_is_no_package() {
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(
        Reader::new(b"nope".to_vec()),
        &Limits::default(),
        &mut diagnostics,
    );
    assert!(package.unwrap().is_none());
    assert_eq!(diagnostics.records()[0].diagnostic, Diagnostic::NotAZip {});
}

#[test]
fn a_default_extension_matches_once_decoded_as_a_part_name_is() {
    let stream = types_with(r#"<Default Extension="%C3%A9" ContentType="a/b"/>"#);
    let items = package(stream, vec![part("x.%c3%a9")]);
    let seen = found(items);
    assert_eq!(seen, vec![at(Diagnostic::PartUnreferenced {}, "/x.é")]);
}

#[test]
fn a_refused_mapping_still_covers_what_it_names() {
    let stream = types(&format!(
        r#"<Default Extension="rels" ContentType="{RELS_TYPE}"/>
           <Override PartName="xl/workbook.xml" ContentType="a/b"/>
           <Default Extension="bin"/>
           <Default Extension="b;c" ContentType="a/b"/>
           <Default Extension="dat" ContentType="a/b"/>"#
    ));
    let items = package(stream, vec![part("a.bin"), part("a.b;c"), part("a.png")]);
    let (package, seen) = super::opc::read(items);
    let missing = seen
        .iter()
        .filter(|(d, _)| *d == Diagnostic::PartContentTypeMissing {})
        .map(|(_, at)| at.as_str())
        .collect::<Vec<_>>();
    // Only the part no mapping names, refused or not, is reported.
    assert_eq!(missing, ["/a.png"]);
    let package = package.unwrap();
    let unknown = |name: &str| package.part(name).unwrap().content_type_unknown();
    assert!(unknown("/xl/workbook.xml") && unknown("/a.bin") && unknown("/a.b;c"));
    assert!(!unknown("/a.png") && !unknown("/_rels/.rels"));
}

#[test]
fn without_a_stream_no_part_type_is_known() {
    let (package, _) = super::opc::read(vec![package_rels(""), part("xl/workbook.xml")]);
    let package = package.unwrap();
    assert!(package.parts().iter().all(|p| p.content_type_unknown()));
}

#[test]
fn a_refused_mapping_makes_a_later_one_of_the_same_key_refused_not_a_duplicate() {
    let stream = types_with(concat!(
        r#"<Override PartName="xl/workbook.xml" ContentType="a/b"/>"#,
        r#"<Override PartName="/xl/workbook.xml" ContentType="a/b"/>"#,
        r#"<Default Extension="%C3%A9" ContentType="a/b"/>"#,
        r#"<Default Extension="é" ContentType="a/b"/>"#,
    ));
    let (package, seen) = super::opc::read(package(stream, vec![part("a.%C3%A9")]));
    let codes: Vec<&str> = seen.iter().map(|(d, _)| d.code().as_str()).collect();
    assert_eq!(
        codes,
        [
            "content_types_part_name_invalid",
            "xml_schema_invalid",
            "part_unreferenced"
        ]
    );
    let package = package.unwrap();
    let unknown = |name: &str| package.part(name).unwrap().content_type_unknown();
    assert!(unknown("/xl/workbook.xml") && unknown("/a.é"));
}

#[test]
fn an_entry_that_may_have_been_the_stream_is_not_reported_missing_again() {
    let mut items = package(types(DEFAULTS), vec![part("a.bin")]);
    items[0].local.name = b"/[Content_Types].xml".to_vec();
    items[0].central.name = b"/[Content_Types].xml".to_vec();
    let invalid = Diagnostic::EntryNameInvalid {
        reason: PartNameReason::EmptySegment,
    };
    assert_eq!(
        found(items),
        vec![
            at(invalid, "/[Content_Types].xml"),
            at(Diagnostic::PartUnreferenced {}, "/a.bin"),
        ]
    );
}

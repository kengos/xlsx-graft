//! What `PackageEdit` refuses and how `finish` ends when it cannot write:
//! calls that name a part wrongly, packages that cannot be edited, a zip
//! entry that cannot be read or is damaged, XML that is refused, and
//! failures of the host.

use super::{
    Refuse, Text, contents, edit, entries, finish, finish_with, found, media, open, open_loose,
    part, written_zip, zip,
};
use crate::tests::Sink;
use crate::{Link, LinkSource, PackageEdit, PartRewriter, PartWriter, WriteError, WriteLimits};
use std::io::{self, BufRead, Cursor, Write};
use xlsx_graft_diagnostics::{Diagnostics, Limit};
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_xml::{XmlLimits, XmlStreamError};

type Edit = PackageEdit<'static, Cursor<Vec<u8>>>;

fn add(edit: &mut Edit, name: &str, link: Option<Link>) -> Result<Option<String>, WriteError> {
    edit.add(
        part(name),
        media("application/xml"),
        Box::new(Text("<x/>")),
        link,
    )
}

fn from(name: &str) -> Option<Link> {
    Some(Link {
        source: LinkSource::Part(part(name)),
        rel_type: "urn:t".to_owned(),
    })
}

/// The edit of the tests' package with the entries `change` changes, which
/// the package is read from without checking its diagnostics.
fn changed(change: impl FnOnce(&mut Vec<(String, String)>)) -> Edit {
    let mut all = entries();
    change(&mut all);
    PackageEdit::new(open_loose(zip(&all)))
}

#[test]
fn a_relationships_part_is_reserved_to_the_edit() {
    let mut edit = edit();
    let rels = part("/xl/_rels/workbook.xml.rels");
    let error = edit.replace(&rels, Box::new(Text("<x/>"))).unwrap_err();
    assert!(
        matches!(&error, WriteError::PartReserved { name } if name == "/xl/_rels/workbook.xml.rels"),
        "{error:?}"
    );
    assert!(matches!(
        edit.remove(&rels),
        Err(WriteError::PartReserved { .. })
    ));
    assert!(matches!(
        add(&mut edit, "/new/_rels/a.xml.rels", None),
        Err(WriteError::PartReserved { .. })
    ));
    assert!(matches!(
        add(&mut edit, "/_rels/.rels", None),
        Err(WriteError::PartReserved { .. })
    ));
}

#[test]
fn a_part_the_package_does_not_have_is_missing() {
    let mut edit = edit();
    let error = edit
        .replace(&part("/xl/none.xml"), Box::new(Text("<x/>")))
        .unwrap_err();
    assert!(
        matches!(&error, WriteError::PartMissing { name } if name == "/xl/none.xml"),
        "{error:?}"
    );
    assert!(matches!(
        edit.remove(&part("/xl/none.xml")),
        Err(WriteError::PartMissing { .. })
    ));
    // A link whose source is not there, or was removed.
    let error = add(&mut edit, "/new.xml", from("/xl/none.xml")).unwrap_err();
    assert!(
        matches!(&error, WriteError::PartMissing { name } if name == "/xl/none.xml"),
        "{error:?}"
    );
    edit.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    assert!(matches!(
        add(&mut edit, "/new.xml", from("/xl/worksheets/sheet1.xml")),
        Err(WriteError::PartMissing { .. })
    ));
    assert!(matches!(
        edit.remove(&part("/xl/worksheets/sheet1.xml")),
        Err(WriteError::PartMissing { .. })
    ));
    // A part added is not a part of the package: it cannot be replaced or removed.
    add(&mut edit, "/added.xml", None).unwrap();
    assert!(matches!(
        edit.remove(&part("/added.xml")),
        Err(WriteError::PartMissing { .. })
    ));
}

#[test]
fn a_name_the_package_or_an_earlier_add_has_is_taken_ascii_letter_case_aside() {
    let mut edit = edit();
    let error = add(&mut edit, "/XL/Workbook.xml", None).unwrap_err();
    assert!(
        matches!(&error, WriteError::PartExists { name } if name == "/XL/Workbook.xml"),
        "{error:?}"
    );
    add(&mut edit, "/new.xml", None).unwrap();
    assert!(matches!(
        add(&mut edit, "/NEW.xml", None),
        Err(WriteError::PartExists { .. })
    ));
    // A part removed is still the package's: replace it instead.
    edit.remove(&part("/xl/workbook.xml")).unwrap();
    assert!(matches!(
        add(&mut edit, "/xl/workbook.xml", None),
        Err(WriteError::PartExists { .. })
    ));
    let mut edit = changed(|all| all.push(("xl/a.xml".to_owned(), "<a/>".to_owned())));
    assert!(matches!(
        add(&mut edit, "/xl/A.xml", None),
        Err(WriteError::PartExists { .. })
    ));
    // Two entries of the same name are no part, but may each have been one.
    let mut edit = changed(|all| {
        all.push(("xl/a.xml".to_owned(), "<a/>".to_owned()));
        all.push(("xl/A.xml".to_owned(), "<a/>".to_owned()));
    });
    assert!(matches!(
        add(&mut edit, "/xl/a.XML", None),
        Err(WriteError::PartExists { .. })
    ));
}

#[test]
fn a_source_whose_relationships_are_not_all_known_takes_no_link_and_a_part_added_with_one_is_not_added()
 {
    let broken = |all: &mut Vec<(String, String)>| {
        all[4].1 = "<Relationships".to_owned();
        all[1].1 = "<Relationships".to_owned();
    };
    let mut edit = changed(broken);
    let error = add(&mut edit, "/new.xml", from("/xl/workbook.xml")).unwrap_err();
    assert!(
        matches!(&error, WriteError::RelationshipsUnknown { name } if name == "/xl/workbook.xml"),
        "{error:?}"
    );
    let link = Some(Link {
        source: LinkSource::Package,
        rel_type: "urn:t".to_owned(),
    });
    let error = add(&mut edit, "/new.xml", link).unwrap_err();
    assert!(
        matches!(&error, WriteError::RelationshipsUnknown { name } if name == "/_rels/.rels"),
        "{error:?}"
    );
    // Nothing was added: the same name is free.
    add(&mut edit, "/new.xml", None).unwrap();
}

#[test]
fn a_part_cannot_be_removed_when_the_relationships_of_the_package_are_not_all_known() {
    let mut edit = changed(|all| all[4].1 = "<Relationships".to_owned());
    assert!(matches!(
        edit.remove(&part("/xl/worksheets/sheet1.xml")),
        Err(WriteError::GraphIncomplete)
    ));
    // A part that is missing is reported as that first.
    assert!(matches!(
        edit.remove(&part("/none.xml")),
        Err(WriteError::PartMissing { .. })
    ));
}

#[test]
fn a_call_that_fails_changes_nothing() {
    let mut refused = edit();
    let _ = add(&mut refused, "/xl/workbook.xml", from("/xl/workbook.xml"));
    let _ = add(&mut refused, "/new.xml", from("/xl/none.xml"));
    let _ = refused.remove(&part("/none.xml"));
    let _ = refused.replace(&part("/none.xml"), Box::new(Text("<x/>")));
    assert_eq!(contents(&written_zip(refused)), entries());
}

/// The bytes of the tests' package with the central header of the entry
/// at `index` given the compression method 99, which no reader supports.
fn with_unsupported_method(index: usize) -> Vec<u8> {
    let mut bytes = zip(&entries());
    let at = bytes
        .windows(4)
        .enumerate()
        .filter(|(_, w)| w == b"PK\x01\x02")
        .map(|(at, _)| at)
        .nth(index);
    bytes[at.unwrap() + 10] = 99;
    bytes
}

#[test]
fn an_entry_that_cannot_be_read_stops_the_write_with_a_diagnostic_at_the_entry() {
    let mut diagnostics = Diagnostics::new();
    let package = xlsx_graft_package::Package::open(
        Cursor::new(with_unsupported_method(2)),
        &xlsx_graft_package::Limits::default(),
        &mut diagnostics,
    );
    assert_eq!(
        found(&diagnostics),
        ["zip_unsupported_compression docProps/app.xml"]
    );
    let (bytes, diagnostics) = finish(PackageEdit::new(package.unwrap().unwrap()));
    assert!(bytes.is_none());
    assert_eq!(
        found(&diagnostics),
        ["write_entry_unreadable docProps/app.xml"]
    );
    assert_eq!(
        diagnostics.records()[0].diagnostic.code().to_string(),
        "write_entry_unreadable"
    );
}

#[test]
fn every_entry_that_cannot_be_read_is_named_but_one_that_is_left_out_is_not() {
    let mut bytes = with_unsupported_method(2);
    let at = bytes
        .windows(4)
        .enumerate()
        .filter(|(_, w)| w == b"PK\x01\x02")
        .map(|(at, _)| at)
        .nth(5);
    bytes[at.unwrap() + 10] = 99;
    let package = || open_loose(bytes.clone());
    let (written, diagnostics) = finish(PackageEdit::new(package()));
    assert!(written.is_none());
    assert_eq!(
        found(&diagnostics),
        [
            "write_entry_unreadable docProps/app.xml",
            "write_entry_unreadable xl/worksheets/sheet1.xml"
        ]
    );
    let mut edit = PackageEdit::new(package());
    edit.remove(&part("/docProps/app.xml")).unwrap();
    edit.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    let all = contents(&written_zip(edit));
    let names: Vec<&str> = all.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "[Content_Types].xml",
            "_rels/.rels",
            "xl/workbook.xml",
            "xl/_rels/workbook.xml.rels"
        ]
    );
}

#[test]
fn an_entry_found_damaged_while_it_is_copied_stops_the_write() {
    let mut bytes = zip(&entries());
    let at = bytes
        .windows(12)
        .position(|w| w == b"<worksheet/>")
        .unwrap();
    bytes[at + 1] ^= 0xff;
    let (written, diagnostics) = finish(PackageEdit::new(open(bytes)));
    assert!(written.is_none());
    assert_eq!(
        found(&diagnostics),
        ["zip_crc_mismatch xl/worksheets/sheet1.xml"]
    );
}

#[test]
fn a_content_types_stream_that_could_not_be_read_stops_an_edit_that_adds_without_a_new_diagnostic()
{
    let mut edit = changed(|all| all[0].1 = "<Types".to_owned());
    add(&mut edit, "/new.xml", None).unwrap();
    let (written, diagnostics) = finish(edit);
    assert!(written.is_none());
    // The package reported the stream already: nothing more is pushed.
    assert_eq!(found(&diagnostics), Vec::<String>::new());
}

#[test]
fn xml_past_a_limit_is_a_diagnostic_at_its_part_and_no_zip() {
    let limited = |xml: XmlLimits| WriteLimits {
        xml,
        ..WriteLimits::default()
    };
    let mut edit = edit();
    add(&mut edit, "/new.xml", None).unwrap();
    let depth = XmlLimits {
        max_depth: Limit::default_of(1),
        ..XmlLimits::default()
    };
    let (written, diagnostics) = finish_with(edit, &limited(depth));
    assert!(written.is_none());
    assert_eq!(found(&diagnostics), ["xml_too_deep [Content_Types].xml"]);
    // A Relationships part: its `Relationship` has three attributes, and the
    // content types stream's children two.
    let mut edit = self::edit();
    add(&mut edit, "/new.xml", from("/xl/workbook.xml")).unwrap();
    let attributes = XmlLimits {
        max_attributes: Limit::default_of(2),
        ..XmlLimits::default()
    };
    let (written, diagnostics) = finish_with(edit, &limited(attributes));
    assert!(written.is_none());
    assert_eq!(
        found(&diagnostics),
        ["xml_too_many_attributes /xl/_rels/workbook.xml.rels"]
    );
}

/// An edit that replaces, adds with a link to an existing and to a new
/// Relationships part, and removes.
fn busy() -> Edit {
    let mut edit = edit();
    edit.replace(&part("/xl/workbook.xml"), Box::new(Text("<new/>")))
        .unwrap();
    add(&mut edit, "/a.xml", from("/xl/workbook.xml")).unwrap();
    add(&mut edit, "/b.xml", from("/docProps/app.xml")).unwrap();
    edit.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    edit
}

#[test]
fn a_write_to_a_destination_that_fails_returns_the_error_wherever_it_fails() {
    let (whole, _) = finish(busy());
    let whole = whole.unwrap().len();
    for fail_after in (0..whole).step_by(31) {
        let mut diagnostics = Diagnostics::new();
        let result = busy().finish(
            Sink::failing_after(fail_after),
            &mut MemorySpools,
            &WriteLimits::default(),
            &mut diagnostics,
        );
        assert!(
            matches!(result, Err(WriteError::Io(_))),
            "{fail_after}: {result:?}"
        );
    }
    let mut diagnostics = Diagnostics::new();
    let result = busy().finish(
        Sink::failing_after(whole),
        &mut MemorySpools,
        &WriteLimits::default(),
        &mut diagnostics,
    );
    assert_eq!(result.unwrap().unwrap().bytes.len(), whole);
}

/// A rewriter and writer that fail as a host does.
struct Fails;

impl PartRewriter for Fails {
    fn rewrite(
        &mut self,
        _original: &mut dyn BufRead,
        _out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        Err(io::Error::other("the disk is full").into())
    }
}

impl PartWriter for Fails {
    fn write(&mut self, _out: &mut dyn Write) -> Result<(), WriteError> {
        Err(io::Error::other("the disk is full").into())
    }
}

#[test]
fn a_failure_of_a_rewriter_or_a_writer_that_is_not_a_problem_of_the_workbook_is_returned() {
    let result = |edit: Edit| {
        edit.finish(
            Sink::default(),
            &mut MemorySpools,
            &WriteLimits::default(),
            &mut Diagnostics::new(),
        )
    };
    let mut with_rewriter = edit();
    with_rewriter
        .replace(&part("/xl/workbook.xml"), Box::new(Fails))
        .unwrap();
    assert!(matches!(result(with_rewriter), Err(WriteError::Io(_))));
    let mut with_writer = edit();
    with_writer
        .add(
            part("/new.xml"),
            media("application/xml"),
            Box::new(Fails),
            None,
        )
        .unwrap();
    assert!(matches!(result(with_writer), Err(WriteError::Io(_))));
}

#[test]
fn the_errors_say_what_is_wrong() {
    let errors = [
        WriteError::PartMissing {
            name: "/a.xml".to_owned(),
        },
        WriteError::PartExists {
            name: "/a.xml".to_owned(),
        },
        WriteError::PartReserved {
            name: "/_rels/.rels".to_owned(),
        },
        WriteError::RelationshipsUnknown {
            name: "/a.xml".to_owned(),
        },
        WriteError::GraphIncomplete,
        WriteError::Invalid(Refuse::diagnostic()),
    ];
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    let refused = format!(
        "the content of a part is refused (xml_doctype): {}",
        Refuse::diagnostic().message()
    );
    assert_eq!(
        messages,
        [
            "the package has no part /a.xml",
            "the name /a.xml is the same part as a name of the package, or a folder of one, or in a folder of one",
            "/_rels/.rels is a Relationships part, which the edit writes itself",
            "the relationships of /a.xml are not all known, so one cannot be added to them",
            "the relationships of the package are not all known, so a part cannot be removed",
            refused.as_str(),
        ]
    );
}

#[test]
fn an_xml_stream_error_converts_to_the_write_error_it_stands_for() {
    let invalid = WriteError::from(XmlStreamError::Invalid(Refuse::diagnostic()));
    assert!(matches!(invalid, WriteError::Invalid(d) if d == Refuse::diagnostic()));
    let io = WriteError::from(XmlStreamError::Io(io::Error::other("gone")));
    assert!(matches!(&io, WriteError::Io(e) if e.to_string() == "gone"));
    // An error of the writer the content is deflated into keeps its variant.
    let carried = WriteError::from(XmlStreamError::Io(io::Error::other(WriteError::Unusable)));
    assert!(matches!(carried, WriteError::Unusable));
}

#[test]
fn the_default_limits_spill_at_8_mib_and_read_xml_under_the_default_xml_limits() {
    let limits = WriteLimits::default();
    assert_eq!(limits.part_spill_bytes, 8 * 1024 * 1024);
    assert_eq!(limits.xml, XmlLimits::default());
}

#[test]
fn a_package_with_no_content_types_stream_is_not_written_when_the_edit_adds_or_removes() {
    let without = |all: &mut Vec<(String, String)>| {
        all.remove(0);
    };
    // Nothing is written: `finish` returns before it makes the zip.
    let finished = |edit: Edit| {
        let (written, diagnostics) = finish(edit);
        (written.is_some(), found(&diagnostics))
    };
    let mut adding = changed(without);
    add(&mut adding, "/new.xml", None).unwrap();
    assert_eq!(finished(adding), (false, Vec::<String>::new()));
    let mut removing = changed(without);
    removing.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
    assert_eq!(finished(removing), (false, Vec::<String>::new()));
    // An edit that leaves the stream alone is written.
    let mut replacing = changed(without);
    replacing
        .replace(&part("/xl/workbook.xml"), Box::new(Text("<new/>")))
        .unwrap();
    assert!(finished(replacing).0);
}

#[test]
fn a_part_refused_is_reported_under_its_part_name_not_its_entry_name() {
    let encoded = |all: &mut Vec<(String, String)>| {
        all.push(("xl/%E3%82%B7.xml".to_owned(), "<a/>".to_owned()));
    };
    let mut edit = changed(encoded);
    edit.replace(&part("/xl/\u{30b7}.xml"), Box::new(Refuse))
        .unwrap();
    let (written, diagnostics) = finish(edit);
    assert!(written.is_none());
    assert_eq!(found(&diagnostics), ["xml_doctype /xl/\u{30b7}.xml"]);
}

#[test]
fn a_name_derived_from_or_a_folder_of_another_part_name_is_taken() {
    let mut edit = edit();
    for name in [
        "/xl/workbook.xml/extra.xml",
        "/XL/Workbook.xml/a/b.xml",
        "/xl",
        "/XL",
        "/docProps",
    ] {
        let error = add(&mut edit, name, None).unwrap_err();
        assert!(
            matches!(&error, WriteError::PartExists { name: n } if n == name),
            "{name}: {error:?}"
        );
    }
    // A name that only starts with the same characters is another part.
    add(&mut edit, "/xl/workbook.xmlx", None).unwrap();
    add(&mut edit, "/xl2", None).unwrap();
    // The same among the parts added.
    add(&mut edit, "/a/b.xml", None).unwrap();
    assert!(matches!(
        add(&mut edit, "/a/b.xml/c.xml", None),
        Err(WriteError::PartExists { .. })
    ));
    assert!(matches!(
        add(&mut edit, "/A", None),
        Err(WriteError::PartExists { .. })
    ));
    add(&mut edit, "/a/c.xml", None).unwrap();
}

#[test]
fn a_link_that_makes_a_relationships_part_and_a_part_added_cannot_overlap() {
    let names = |edit: Edit| -> Vec<String> {
        let all = contents(&written_zip(edit));
        all[7..].iter().map(|(name, _)| name.clone()).collect()
    };
    // A part at the folder the new Relationships part would be in, added first.
    let mut edit = edit();
    add(&mut edit, "/docProps/_rels", None).unwrap();
    let error = add(&mut edit, "/docProps/b.xml", from("/docProps/app.xml")).unwrap_err();
    assert!(
        matches!(&error, WriteError::PartExists { name } if name == "/docProps/_rels/app.xml.rels"),
        "{error:?}"
    );
    // The call that failed changed nothing: its part is not added, nor claimed.
    add(&mut edit, "/docProps/b.xml", None).unwrap();
    assert_eq!(names(edit), ["docProps/_rels", "docProps/b.xml"]);
    // The same two, the link first.
    let mut edit = self::edit();
    add(&mut edit, "/docProps/b.xml", from("/docProps/app.xml")).unwrap();
    let error = add(&mut edit, "/docProps/_rels", None).unwrap_err();
    assert!(matches!(&error, WriteError::PartExists { .. }), "{error:?}");
    assert_eq!(
        names(edit),
        ["docProps/b.xml", "docProps/_rels/app.xml.rels"]
    );
    // A Relationships part the package has is claimed already: a folder of it
    // is refused, and a link from its source makes no new one.
    let mut edit = self::edit();
    add(&mut edit, "/xl/_rels", None).unwrap_err();
    add(&mut edit, "/xl/c.xml", from("/xl/workbook.xml")).unwrap();
}

#[test]
fn a_relationship_type_with_a_character_xml_does_not_allow_is_refused_and_changes_nothing() {
    let link = |rel_type: &str| {
        Some(Link {
            source: LinkSource::Package,
            rel_type: rel_type.to_owned(),
        })
    };
    let mut edit = edit();
    for bad in [
        "urn:a\u{1}b",
        "\u{8}",
        "a\u{b}",
        "a\u{c}",
        "\u{1f}",
        "a\u{fffe}",
        "\u{ffff}b",
    ] {
        let result = edit.add(
            part("/new.xml"),
            media("application/xml"),
            Box::new(Text("<x/>")),
            link(bad),
        );
        assert!(
            matches!(&result, Err(WriteError::RelationshipTypeInvalid { rel_type }) if rel_type == bad),
            "{bad:?}: {result:?}"
        );
    }
    // Nothing was taken: the same name is free, and the package is as it was.
    assert_eq!(contents(&written_zip(edit))[..7], entries()[..]);
    // The characters XML 1.0 allows are written, and the package reads back.
    let mut edit = self::edit();
    let allowed = "urn:\t\n\r \u{d7ff}\u{e000}\u{fffd}\u{10000}\u{10ffff}&<>\"'";
    let id = edit.add(
        part("/new.xml"),
        media("application/xml"),
        Box::new(Text("<x/>")),
        link(allowed),
    );
    assert_eq!(id.unwrap().as_deref(), Some("rId3"));
    let bytes = written_zip(edit);
    let package = super::open_loose(bytes);
    assert_eq!(
        package.relationships()[2].rel_type(),
        xlsx_graft_package::collapse(allowed)
    );
}

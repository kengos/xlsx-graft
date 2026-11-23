//! The names an edit takes, checked from every side: the content types
//! stream, the zip entries and the Relationships parts of the package written
//! never hold a name twice or two names that overlap, whatever the package
//! read held. The first tests are single cases (a name, an `Override`, a link
//! from a Relationships part); the last takes every kind of input against
//! every kind of call.

use super::{
    DECLARATION, Entries, RELS_NS, Text, contents, edit, entries, finish, found, media, open_loose,
    part, reread, written_zip, zip,
};
use crate::{Link, LinkSource, PackageEdit, WriteError};
use std::collections::BTreeMap;
use std::io::Cursor;
use xlsx_graft_diagnostics::Diagnostics;

type Edit = PackageEdit<'static, Cursor<Vec<u8>>>;

fn changed(change: impl FnOnce(&mut Entries)) -> Edit {
    let mut all = entries();
    change(&mut all);
    PackageEdit::new(open_loose(zip(&all)))
}

fn push(all: &mut Entries, name: &str, content: &str) {
    all.push((name.to_owned(), content.to_owned()));
}

/// Adds `/xx` as `name` with `link` from the part `source`, if any.
fn add(edit: &mut Edit, name: &str, source: Option<&str>) -> Result<Option<String>, WriteError> {
    let link = source.map(|source| Link {
        source: LinkSource::Part(part(source)),
        rel_type: "urn:t".to_owned(),
    });
    edit.add(
        part(name),
        media("application/xml"),
        Box::new(Text("<x/>")),
        link,
    )
}

/// A stream's `Override` added before its end tag.
fn with_override(all: &mut Entries, attributes: &str) {
    let element = format!(r#"<Override {attributes} ContentType="application/xml"/>"#);
    all[0].1 = all[0].1.replace("</Types>", &format!("{element}</Types>"));
}

fn exists(result: Result<Option<String>, WriteError>, name: &str) {
    match result {
        Err(WriteError::PartExists { name: refused }) => assert_eq!(refused, name),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_override_that_names_no_part_but_the_name_added_is_replaced_not_repeated() {
    // A leftover from a part that was deleted, then a value that is no part
    // name and keys the same.
    for attributes in [r#"PartName="/xl/new.xml""#, r#"PartName="xl/new.xml""#] {
        let mut edit = changed(|all| with_override(all, attributes));
        add(&mut edit, "/xl/new.xml", None).unwrap();
        let bytes = written_zip_loose(edit);
        let stream = &contents(&bytes)[0].1;
        assert_eq!(stream.matches("new.xml").count(), 1, "{stream}");
        let codes = found(&reread(&bytes));
        assert_eq!(codes, ["part_unreferenced /xl/new.xml"], "{attributes}");
    }
}

/// The zip `edit` writes, which must be a zip, whatever the diagnostics of the
/// package it started from.
fn written_zip_loose(edit: Edit) -> Vec<u8> {
    let (bytes, diagnostics) = finish(edit);
    assert_eq!(found(&diagnostics), Vec::<String>::new());
    bytes.unwrap()
}

#[test]
fn an_override_for_the_relationships_part_made_is_replaced_not_repeated() {
    let mut edit = changed(|all| with_override(all, r#"PartName="/docProps/_rels/app.xml.rels""#));
    add(&mut edit, "/docProps/b.xml", Some("/docProps/app.xml")).unwrap();
    let bytes = written_zip_loose(edit);
    let stream = &contents(&bytes)[0].1;
    assert_eq!(stream.matches("_rels/app.xml.rels").count(), 1, "{stream}");
    assert_eq!(found(&reread(&bytes)), Vec::<String>::new());
}

#[test]
fn a_stream_that_cannot_say_what_every_mapping_names_stops_an_edit_that_adds_or_removes() {
    let wild = |all: &mut Entries| {
        all[0].1 = all[0]
            .1
            .replace("</Types>", r#"<Override ContentType="a"/></Types>"#)
    };
    let twice = |all: &mut Entries| push(all, "[content_types].xml", &all[0].1.clone());
    for make in [&wild as &dyn Fn(&mut Entries), &twice] {
        let mut adding = changed(|all| make(all));
        add(&mut adding, "/xl/new.xml", None).unwrap();
        let (bytes, diagnostics) = finish(adding);
        assert!(bytes.is_none());
        assert_eq!(found(&diagnostics), Vec::<String>::new());
        let mut removing = changed(|all| make(all));
        removing.remove(&part("/xl/worksheets/sheet1.xml")).unwrap();
        assert!(finish(removing).0.is_none());
        // An edit that leaves the stream alone is written.
        let mut replacing = changed(|all| make(all));
        replacing
            .replace(&part("/xl/workbook.xml"), Box::new(Text("<new/>")))
            .unwrap();
        assert!(finish(replacing).0.is_some());
    }
}

#[test]
fn a_part_is_not_given_a_relationships_part_that_the_package_holds_with_no_source_part() {
    let rels = format!(r#"{DECLARATION}<Relationships xmlns="{RELS_NS}"/>"#);
    let mut edit = changed(|all| push(all, "xl/_rels/ghost.xml.rels", &rels));
    exists(
        add(&mut edit, "/xl/ghost.xml", None),
        "/xl/_rels/ghost.xml.rels",
    );
    // Another name is free.
    add(&mut edit, "/xl/other.xml", None).unwrap();
    // The same name taken by two entries that are no part: the package reads
    // neither, and a part added would still be read with that name as its
    // relationships.
    let mut edit = changed(|all| {
        push(all, "xl/_rels/ghost.xml.rels", "<a/>");
        push(all, "XL/_RELS/GHOST.XML.RELS", "<a/>");
    });
    exists(
        add(&mut edit, "/xl/ghost.xml", None),
        "/xl/_rels/ghost.xml.rels",
    );
}

#[test]
fn a_link_from_a_relationships_part_is_refused() {
    let mut edit = edit();
    let source = "/xl/_rels/workbook.xml.rels";
    let error = add(&mut edit, "/xl/new.xml", Some(source)).unwrap_err();
    assert!(
        matches!(error, WriteError::PartReserved { .. }),
        "{error:?}"
    );
    assert!(add(&mut edit, "/xl/new.xml", None).is_ok());
}

#[test]
fn a_name_under_a_name_the_package_refused_for_its_entry_is_taken() {
    // Two entries of the same part name are no part, and each takes it.
    let mut edit = changed(|all| {
        push(all, "xl/dup.xml", "<a/>");
        push(all, "XL/DUP.XML", "<a/>");
    });
    exists(
        add(&mut edit, "/xl/dup.xml/c.xml", None),
        "/xl/dup.xml/c.xml",
    );
    exists(add(&mut edit, "/xl/dup.xml", None), "/xl/dup.xml");
    // The Relationships part a link makes is under such a name.
    let mut edit = changed(|all| {
        push(all, "docProps/_rels", "<a/>");
        push(all, "DOCPROPS/_RELS", "<a/>");
    });
    let result = add(&mut edit, "/docProps/b.xml", Some("/docProps/app.xml"));
    exists(result, "/docProps/_rels/app.xml.rels");
    // The call that failed took nothing.
    add(&mut edit, "/docProps/b.xml", None).unwrap();
}

#[test]
fn the_first_link_from_a_source_spelled_in_another_letter_case_makes_the_relationships_part_as_the_package_spells_it()
 {
    let mut edit = edit();
    add(&mut edit, "/docProps/a.xml", Some("/DOCPROPS/APP.XML")).unwrap();
    let bytes = written_zip(edit);
    let all = contents(&bytes);
    assert_eq!(all.last().unwrap().0, "docProps/_rels/app.xml.rels");
    let stream = &all[0].1;
    assert!(
        stream.contains(r#"PartName="/docProps/_rels/app.xml.rels""#),
        "{stream}"
    );
    assert!(!stream.contains("DOCPROPS"), "{stream}");
}

#[test]
fn a_part_named_like_a_rels_folder_overlaps_the_relationships_parts_under_it() {
    // `/xl/_rels` is no Relationships part, and a folder of the package's.
    let mut edit = edit();
    exists(add(&mut edit, "/xl/_rels", None), "/xl/_rels");
    // The part and the Relationships part its link makes overlap each other.
    exists(
        add(&mut edit, "/docProps/_rels", Some("/docProps/app.xml")),
        "/docProps/_rels/app.xml.rels",
    );
    add(&mut edit, "/docProps/_rels", None).unwrap();
}

/// How many times each diagnostic, as its code and the entry, part,
/// Relationships part and relationship it points at, is in `diagnostics`,
/// whatever its severity.
fn counted(diagnostics: &Diagnostics) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for line in found(diagnostics) {
        *counts.entry(line).or_insert(0) += 1;
    }
    counts
}

/// A kind of input: what it is, whether an edit that adds a part must stop
/// (`Ok(None)`, with no diagnostic of its own), whether a part can be removed
/// from it (`GraphIncomplete` when it holds a Relationships part the package
/// could not read whole) and how it changes the entries of the package.
struct Input {
    kind: &'static str,
    stops: bool,
    removable: bool,
    change: Box<dyn Fn(&mut Entries)>,
}

fn input(kind: &'static str, change: impl Fn(&mut Entries) + 'static) -> Input {
    let change = Box::new(change);
    Input {
        kind,
        stops: false,
        removable: true,
        change,
    }
}

impl Input {
    /// An edit of this kind of input must stop where it adds a part.
    fn stops(mut self) -> Self {
        self.stops = true;
        self
    }

    /// No part can be removed from this kind of input.
    fn fixed(mut self) -> Self {
        self.removable = false;
        self
    }
}

/// The kinds of input that hold a name no part has, or a stream that cannot
/// type a part added.
fn inputs() -> Vec<Input> {
    let rels = format!(r#"{DECLARATION}<Relationships xmlns="{RELS_NS}"/>"#);
    vec![
        input("clean", |_| {}),
        input("two entries of one part name", |all| {
            push(all, "xl/dup.xml", "<a/>");
            push(all, "XL/DUP.XML", "<a/>");
        }),
        input("an entry that is no part name", |all| {
            push(all, "xl/a%zz.xml", "<a/>");
        }),
        input("an override of a part that is not there", |all| {
            with_override(all, r#"PartName="/xl/new.xml""#);
        }),
        input("an override that is no part name", |all| {
            with_override(all, r#"PartName="xl/new.xml""#);
        }),
        input("a Relationships part with no source", move |all| {
            push(all, "xl/_rels/ghost.xml.rels", &rels);
        })
        .fixed(),
        input("a Relationships part name in two entries", |all| {
            push(all, "xl/_rels/ghost.xml.rels", "<a/>");
            push(all, "XL/_RELS/GHOST.XML.RELS", "<a/>");
        })
        .fixed(),
        input("a derived name", |all| {
            push(all, "xl/workbook.xml/derived.xml", "<a/>");
        }),
        input("a mapping that names nothing", |all| {
            all[0].1 = all[0]
                .1
                .replace("</Types>", r#"<Override ContentType="a"/></Types>"#);
        })
        .stops(),
        input("two content types streams", |all| {
            let stream = all[0].1.clone();
            push(all, "[content_types].xml", &stream);
        })
        .stops(),
    ]
}

/// A call: the name to add, the source of its link, the parts removed first.
type Call = (&'static str, Option<&'static str>, Vec<&'static str>);

fn calls() -> Vec<Call> {
    let sheet = "/xl/worksheets/sheet1.xml";
    vec![
        ("/xl/new.xml", None, vec![]),
        ("/xl/new.xml", Some("/docProps/app.xml"), vec![]),
        ("/xl/dup.xml", None, vec![]),
        ("/xl/dup.xml/c.xml", None, vec![]),
        ("/xl", None, vec![]),
        ("/xl/ghost.xml", None, vec![]),
        ("/docProps/_rels", None, vec![]),
        ("/docProps/_rels", Some("/DOCPROPS/APP.XML"), vec![]),
        ("/xl/workbook.xml/extra.xml", None, vec![]),
        ("/xl/a.xml", Some("/xl/workbook.xml"), vec![]),
        ("/xl/worksheets/sheet1.xml/x.xml", None, vec![sheet]),
        ("/xl/worksheets/_rels", None, vec![sheet]),
        ("/xl/worksheets", Some("/xl/workbook.xml"), vec![sheet]),
    ]
}

#[test]
fn an_edit_that_takes_a_name_is_refused_or_writes_no_more_of_any_diagnostic_than_the_input_had() {
    let (mut refused, mut stopped, mut written) = (0, 0, 0);
    for Input {
        kind,
        stops,
        removable,
        change,
    } in inputs()
    {
        let mut all = entries();
        change(&mut all);
        let before = counted(&reread(&zip(&all)));
        for (name, source, removed) in calls() {
            let mut edit = PackageEdit::new(open_loose(zip(&all)));
            let case =
                format!("{kind}: add {name} linked from {source:?} after removing {removed:?}");
            // Removing a part is refused only from an input with a
            // Relationships part the package could not read whole.
            let removal: Result<(), WriteError> =
                removed.iter().try_for_each(|n| edit.remove(&part(n)));
            if let Err(error) = removal {
                assert!(
                    !removable && matches!(error, WriteError::GraphIncomplete),
                    "{case}: {error:?}"
                );
                refused += 1;
                continue;
            }
            assert!(removable || removed.is_empty(), "{case}");
            // A name is refused for the names it takes, never for another reason.
            if let Err(error) = add(&mut edit, name, source) {
                assert!(
                    matches!(error, WriteError::PartExists { .. }),
                    "{case}: {error:?}"
                );
                refused += 1;
                continue;
            }
            let (bytes, diagnostics) = finish(edit);
            assert_eq!(found(&diagnostics), Vec::<String>::new(), "{case}");
            // A stream that cannot type a part stops every edit that adds one.
            assert_eq!(bytes.is_none(), stops, "{case}");
            let Some(bytes) = bytes else {
                stopped += 1;
                continue;
            };
            written += 1;
            let mut after = counted(&reread(&bytes));
            // A part added with no link is unreferenced: that notice, at the part
            // added, is the one diagnostic the input could not have had.
            if source.is_none() {
                after.remove(&format!("part_unreferenced {name}"));
            }
            for (line, count) in &after {
                let had = before.get(line).copied().unwrap_or(0);
                assert!(*count <= had, "{case}: {line} is {count} times, was {had}");
            }
        }
    }
    assert!(
        refused > 10 && stopped > 10 && written > 20,
        "{refused} {stopped} {written}"
    );
}

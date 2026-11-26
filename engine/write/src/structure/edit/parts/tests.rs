//! What `classify` makes of the walk of a part: a failure of the host is an
//! error, and the damage of the part (its XML, its zip entry) is a report;
//! and how many references `walk` keeps when it is given the number of
//! references the report can still hold.

use super::super::change::{Change, Deleted, SheetIds};
use super::classify;
use super::text::Context;
use super::walk::{Mode, Outcome, Rules, walk};
use crate::{WriteError, WriteLimits};
use std::collections::BTreeMap;
use std::io;
use std::sync::Arc;
use xlsx_graft_xml::{XmlRewriter, XmlStreamError};

#[test]
fn a_failure_of_the_host_is_an_error_and_damage_of_the_part_is_a_report() {
    let io = || XmlStreamError::Io(io::Error::other("the disk failed"));
    let outcome = classify(Err(io()), false, "/xl/a.xml", 5);
    assert!(matches!(outcome, Err(WriteError::Io(_))));
    let outcome = classify(Err(io()), true, "/xl/a.xml", 5).unwrap();
    assert_eq!((outcome.kept.len(), outcome.omitted), (1, 0));
    let outcome = classify(Ok(Outcome::default()), false, "/xl/a.xml", 5).unwrap();
    assert!(outcome.kept.is_empty());
    // When the report can hold no more references, the unreadable part is
    // counted and not held.
    let outcome = classify(Err(io()), true, "/xl/a.xml", 0).unwrap();
    assert_eq!((outcome.kept.len(), outcome.omitted), (0, 1));
}

#[test]
fn an_empty_sheet_name_is_not_looked_for_and_the_others_are_found_ignoring_letter_case() {
    let empty = Change::Delete(Deleted {
        name: Arc::from(""),
        sides: Vec::new(),
        prev: None,
        next: None,
    });
    let rename = Change::Rename {
        old: Arc::from("Alpha"),
        new: "B".to_owned(),
    };
    let context = Context::new(vec![empty, rename], SheetIds::default(), BTreeMap::new());
    assert_eq!(context.mentioned("anything"), None);
    assert_eq!(context.mentioned("1+ALPHA!x").map(|s| &**s), Some("Alpha"));
}

/// A context of the rename of `Alpha` to `Delta`.
fn renaming() -> Context {
    let mut edit = super::super::tests::open(&["Alpha", "Beta"], "");
    edit.rename_sheet("Alpha", "Delta").unwrap();
    let changes = std::mem::take(&mut edit.state.changes);
    let ids = std::mem::take(&mut edit.state.sheet_ids);
    Context::new(changes, ids, BTreeMap::new())
}

/// What `walk` gives for `xml` when the report can still hold `remaining`
/// references, and what it writes.
fn walked(xml: &str, remaining: usize) -> (Outcome, String) {
    let rules = Rules {
        mode: Mode::Formulas {
            rewrite: true,
            pivot: false,
        },
        tabs: None,
    };
    let mut out = Vec::new();
    let mut rw = XmlRewriter::new(xml.as_bytes(), &mut out, &WriteLimits::default().xml);
    let outcome = walk(&mut rw, "/xl/a.xml", &renaming(), rules, remaining).unwrap();
    drop(rw);
    (outcome, String::from_utf8(out).unwrap())
}

#[test]
fn a_walk_keeps_what_the_report_can_still_hold_counts_the_rest_and_still_rewrites() {
    let cells = r#"<c r="A1"><f>Alpha!"x</f></c>"#.repeat(3);
    let xml = format!(r#"<worksheet>{cells}<c r="B1"><f>Alpha!A1</f></c></worksheet>"#);
    let expected = xml.replace("<f>Alpha!A1", "<f>Delta!A1");
    for (remaining, kept, omitted) in [(0, 0, 3), (2, 2, 1), (3, 3, 0), (4, 3, 0)] {
        let (outcome, written) = walked(&xml, remaining);
        assert_eq!(
            (outcome.kept.len(), outcome.omitted),
            (kept, omitted),
            "{remaining}"
        );
        assert!(outcome.changed, "{remaining}");
        assert_eq!(written, expected, "{remaining}");
    }
}

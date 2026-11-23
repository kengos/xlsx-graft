//! Properties of the names `xlsx_graft_write::structure` accepts: the
//! verifier reports no problem with a sheet name or a defined name the
//! rules accept, whichever scope the defined name has.
//!
//! The test is here and not in `xlsx-graft-write` because the writer is
//! published before the verifier (`scripts/release/crates.txt`), so it
//! cannot depend on it.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

use proptest::prelude::*;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_verify::{NameEntry, SheetEntry, Structure};
use xlsx_graft_write::structure::{check_defined_name, check_sheet_name};

/// The characters a sheet name made to be accepted holds: letters and digits,
/// a space, ASCII symbols (none is forbidden in a sheet name), and characters
/// past ASCII (letters, a full-width digit, an emoji of two UTF-16 units). No
/// `y`, so no name is `History`.
const SHEET_CHARS: &[char] = &[
    'a', 'B', 'c', 'Z', '0', '7', ' ', '.', '_', '-', '(', ')', '#', '!', '"', '$', '%', '&', '+',
    ',', ';', '<', '=', '>', '@', '^', '{', '|', '}', '~', '`', 'é', '売', '１', '😀',
];

/// A sheet name the rules accept: 2 to 7 characters (at most 14 UTF-16
/// units), a `'` only between two others.
fn accepted_sheet_name() -> impl Strategy<Value = String> {
    let character = || proptest::sample::select(SHEET_CHARS);
    let middle = prop_oneof![
        4 => character(),
        1 => Just('\''),
    ];
    (
        character(),
        proptest::collection::vec(middle, 0..6),
        character(),
    )
        .prop_map(|(first, middle, last)| {
            let mut name = String::from(first);
            name.extend(middle);
            name.push(last);
            name
        })
}

/// The characters a defined name made to be accepted holds after its first.
const NAME_CHARS: &[char] = &['a', 'B', '_', '\\', '.', '0', '7', 'é', '売', '１'];

/// A defined name the rules accept: it starts with `_` or a letter past ASCII
/// (never `\`, which alone or with one character is refused), so it is
/// never of the form of a cell reference, `TRUE` or an `_xlnm.` name.
fn accepted_defined_name() -> impl Strategy<Value = String> {
    (
        proptest::sample::select(&['_', 'é', '売'][..]),
        proptest::collection::vec(proptest::sample::select(NAME_CHARS), 0..12),
    )
        .prop_map(|(first, rest)| std::iter::once(first).chain(rest).collect())
}

/// The characters names are made of: the ones each rule is about, and a few
/// letters, digits and characters past ASCII.
const ALPHABET: &[char] = &[
    'a', 'B', 'R', 'C', 'T', 'x', '1', '9', '_', '.', '?', '\\', '\'', '[', ']', ':', '*', '/',
    ' ', '-', '\t', '\u{0}', '\u{fffe}', '\u{3000}', '１', 'é', '売', '😀',
];

/// Any name, forbidden characters among them, so that most are refused: the
/// property is that none the rules accept raises a diagnostic.
///
/// Of the strategies in this file, `accepted_sheet_name` and
/// `accepted_defined_name` copy those of `xlsx-graft-write`'s tests, because
/// the crates cannot share test code; this and `ALPHABET` are this file's own.
fn any_name() -> impl Strategy<Value = String> {
    const WORDS: &[&str] = &[
        "History",
        "TRUE",
        "_xlnm.",
        "A1",
        "R1C1",
        "_xlnm.Print_Area",
    ];
    let pieces = prop_oneof![
        6 => proptest::sample::select(ALPHABET).prop_map(String::from),
        1 => proptest::sample::select(WORDS).prop_map(String::from),
    ];
    proptest::collection::vec(pieces, 0..40).prop_map(|pieces| pieces.concat())
}

/// Whether the verifier reports nothing for `structure`.
fn clean(structure: &Structure) -> bool {
    let mut diagnostics = Diagnostics::new();
    structure.check(&mut diagnostics);
    diagnostics.records().is_empty()
}

/// A workbook of the sheets named `sheets` and the defined names `names`.
fn with_names(sheets: Vec<String>, names: Vec<NameEntry>) -> Structure {
    Structure {
        workbook_part: "/xl/workbook.xml".to_owned(),
        sheets: sheets
            .into_iter()
            .zip(1..)
            .map(|(name, sheet_id)| SheetEntry { name, sheet_id })
            .collect(),
        defined_names: names,
        ..Structure::default()
    }
}

/// A defined name `name` scoped to `scope`, holding `1`.
fn entry(name: &str, scope: Option<u32>) -> NameEntry {
    NameEntry {
        name: name.to_owned(),
        local_sheet_id: scope,
        formula: "1".to_owned(),
    }
}

proptest! {
    #[test]
    fn a_sheet_name_made_to_be_accepted_is_and_raises_no_diagnostic(name in accepted_sheet_name()) {
        prop_assert_eq!(check_sheet_name(&name, []), Ok(()), "{:?}", name);
        prop_assert!(clean(&with_names(vec![name.clone()], vec![])), "{name:?}");
    }

    #[test]
    fn a_defined_name_made_to_be_accepted_is_in_both_scopes_and_raises_no_diagnostic(
        name in accepted_defined_name(),
    ) {
        for scope in [None, Some(0)] {
            prop_assert_eq!(check_defined_name(&name, scope.is_some(), []), Ok(()), "{:?}", name);
            let structure = with_names(vec!["S".to_owned()], vec![entry(&name, scope)]);
            prop_assert!(clean(&structure), "{name:?} {scope:?}");
        }
    }

    #[test]
    fn a_sheet_name_accepted_raises_no_diagnostic(name in any_name()) {
        if check_sheet_name(&name, []).is_ok() {
            prop_assert!(clean(&with_names(vec![name.clone()], vec![])), "{name:?}");
        }
    }

    #[test]
    fn a_defined_name_accepted_raises_no_diagnostic(name in any_name()) {
        for scope in [None, Some(0)] {
            if check_defined_name(&name, scope.is_some(), []).is_ok() {
                let structure = with_names(vec!["S".to_owned()], vec![entry(&name, scope)]);
                prop_assert!(clean(&structure), "{name:?} {scope:?}");
            }
        }
    }
}

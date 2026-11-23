//! Properties of the names the rules accept: the prefix written for sheet
//! names is read back by the formula reader as the sheets it names,
//! spanning the prefix and its `!`; and the reader sees a defined name as
//! one name. That the verifier reports no problem with one is tested in
//! `engine/fixtures-gen/tests/names`.

use crate::structure::{check_defined_name, check_sheet_name, sheet_prefix};
use proptest::prelude::*;
use xlsx_graft_formula::analyze;

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
fn sheet_name() -> impl Strategy<Value = String> {
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
fn defined_name() -> impl Strategy<Value = String> {
    (
        proptest::sample::select(&['_', 'é', '売'][..]),
        proptest::collection::vec(proptest::sample::select(NAME_CHARS), 0..12),
    )
        .prop_map(|(first, rest)| std::iter::once(first).chain(rest).collect())
}

/// Checks that `name` is accepted in both scopes and that the formula reader
/// reads it as one name that spans the whole text and as nothing else.
fn assert_read_as_one_name(name: &str) -> Result<(), TestCaseError> {
    for scoped in [false, true] {
        prop_assert_eq!(check_defined_name(name, scoped, []), Ok(()), "{:?}", name);
    }
    let read = analyze(name);
    let facts = &read.facts;
    prop_assert_eq!(facts.names.len(), 1, "{:?}", name);
    prop_assert_eq!(read.slice(facts.names[0].span), name);
    prop_assert!(facts.calls.is_empty(), "{:?}", name);
    prop_assert!(facts.params.is_empty(), "{:?}", name);
    prop_assert!(facts.references.is_empty(), "{:?}", name);
    prop_assert!(facts.malformed.is_empty(), "{:?}", name);
    Ok(())
}

#[test]
fn fixed_defined_names_are_read_as_one_name() {
    for name in [
        "売上・前年",
        "a.b",
        "\\ab",
        "_\\",
        "R1C1x",
        "_xlfnx",
        "xlfn.a",
        "_xlnm",
    ] {
        assert_read_as_one_name(name).unwrap();
    }
}

proptest! {
    #[test]
    fn a_defined_name_accepted_is_read_as_one_name(name in defined_name()) {
        assert_read_as_one_name(&name)?;
    }

    #[test]
    fn a_prefix_written_is_read_back(
        first in sheet_name(),
        last in proptest::option::of(sheet_name()),
    ) {
        prop_assert_eq!(check_sheet_name(&first, []), Ok(()));
        if let Some(last) = &last {
            prop_assert_eq!(check_sheet_name(last, []), Ok(()));
        }
        let prefix = sheet_prefix(&first, last.as_deref());
        let read = analyze(&format!("{prefix}!A1"));
        let seen = &read.facts.sheet_prefixes;
        prop_assert_eq!(seen.len(), 1, "{:?}", prefix);
        prop_assert_eq!(&seen[0].first, &first);
        prop_assert_eq!(&seen[0].last, &last);
        prop_assert_eq!(&seen[0].book, &None);
        prop_assert_eq!(read.slice(seen[0].span), format!("{prefix}!"));
    }
}

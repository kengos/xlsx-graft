//! `check_sheet_name`: each rule at its boundary, and the order they are
//! checked in.

use crate::structure::{SheetNameFault, StructureError, check_sheet_name};

/// The error for `name` breaking the rule `reason`.
fn invalid(name: &str, reason: SheetNameFault) -> Result<(), StructureError> {
    Err(StructureError::SheetNameInvalid {
        name: name.to_owned(),
        reason,
    })
}

#[test]
fn a_name_is_accepted_up_to_31_utf16_units_and_the_characters_a_formula_quotes() {
    let units_31 = "a".repeat(31);
    let pairs_15 = "😀".repeat(15);
    for name in [
        "Sheet1",
        "売上 (前年)",
        "it's",
        "a'b",
        "A1",
        "R1C1",
        "Historyx",
        units_31.as_str(),
        // 15 surrogate pairs are 30 units, and one more letter 31.
        &format!("{pairs_15}a"),
    ] {
        assert_eq!(check_sheet_name(name, []), Ok(()), "{name:?}");
    }
}

#[test]
fn each_rule_a_name_breaks_is_named() {
    let units_32 = "a".repeat(32);
    let pairs_16 = "😀".repeat(16);
    let cases = [
        ("", SheetNameFault::Empty),
        (units_32.as_str(), SheetNameFault::TooLong),
        (pairs_16.as_str(), SheetNameFault::TooLong),
        ("a[b", SheetNameFault::Character),
        ("a]b", SheetNameFault::Character),
        ("a:b", SheetNameFault::Character),
        ("a*b", SheetNameFault::Character),
        ("a?b", SheetNameFault::Character),
        ("a/b", SheetNameFault::Character),
        ("a\\b", SheetNameFault::Character),
        ("'a", SheetNameFault::Apostrophe),
        ("a'", SheetNameFault::Apostrophe),
        ("a\tb", SheetNameFault::Control),
        ("a\nb", SheetNameFault::Control),
        ("a\rb", SheetNameFault::Control),
        ("a\u{0}b", SheetNameFault::Control),
        ("a\u{1f}b", SheetNameFault::Control),
        ("a\u{fffe}b", SheetNameFault::Control),
        ("a\u{ffff}b", SheetNameFault::Control),
        ("History", SheetNameFault::Reserved),
        ("HISTORY", SheetNameFault::Reserved),
        ("history", SheetNameFault::Reserved),
    ];
    for (name, reason) in cases {
        assert_eq!(
            check_sheet_name(name, []),
            invalid(name, reason),
            "{name:?}"
        );
    }
}

#[test]
fn the_first_rule_in_order_is_the_one_named() {
    let long = format!("{}/", "a".repeat(31));
    assert_eq!(
        check_sheet_name(&long, ["x"]),
        invalid(&long, SheetNameFault::TooLong)
    );
    assert_eq!(
        check_sheet_name("'\u{0}", ["x"]),
        invalid("'\u{0}", SheetNameFault::Apostrophe)
    );
    // No name is both `History` and holds a control character, so
    // `Control` before `Reserved` is not observable; each comes before `Taken`.
    assert_eq!(
        check_sheet_name("a\u{0}", ["a\u{0}"]),
        invalid("a\u{0}", SheetNameFault::Control)
    );
    assert_eq!(
        check_sheet_name("'[", ["x"]),
        invalid("'[", SheetNameFault::Character)
    );
    assert_eq!(
        check_sheet_name("\u{0}[", ["x"]),
        invalid("\u{0}[", SheetNameFault::Character)
    );
    assert_eq!(
        check_sheet_name("History", ["history"]),
        invalid("History", SheetNameFault::Reserved)
    );
}

#[test]
fn a_name_another_sheet_has_is_refused_letter_case_aside() {
    let taken = |name: &str, other: &str| {
        Err(StructureError::SheetNameTaken {
            name: name.to_owned(),
            other: other.to_owned(),
        })
    };
    assert_eq!(
        check_sheet_name("Data", ["Other", "data", "DATA"]),
        taken("Data", "data")
    );
    assert_eq!(check_sheet_name("Éa", ["éA"]), taken("Éa", "éA"));
    assert_eq!(check_sheet_name("Data", ["Data2", "Dat"]), Ok(()));
}

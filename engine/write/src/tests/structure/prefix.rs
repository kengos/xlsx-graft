//! `needs_quotes` and `sheet_prefix`: which names are written in quotes,
//! and the prefix of one sheet or of a 3-D reference.

use crate::structure::{needs_quotes, sheet_prefix};

#[test]
fn a_name_is_quoted_unless_a_formula_reads_it_as_a_name_alone() {
    for plain in [
        "Sheet1", "Data", "_a", "\\a", "a.b", "a_1", "ABCD", "ABCD1", "TRUEx", "A1B", "Sheet.2",
        "_xlfnx",
    ] {
        assert!(!needs_quotes(plain), "{plain}");
    }
    for quoted in [
        "",
        "a b",
        "1a",
        "A",
        "AB",
        "ABC",
        "A1",
        "abc12",
        "XFD1048576",
        "Rx",
        "Cx",
        "RR",
        "R",
        "C",
        "RC",
        "R1C1",
        "r2c",
        "R1",
        "C12",
        "TRUE",
        "false",
        "_xlfn.X",
        "_XLPM.a",
        ".a",
        "a-b",
        "it's",
        "売上",
        "a'",
        "(a)",
    ] {
        assert!(needs_quotes(quoted), "{quoted}");
    }
}

#[test]
fn a_prefix_is_quoted_whole_and_its_quotes_doubled() {
    let cases = [
        ("Sheet1", None, "Sheet1"),
        ("a b", None, "'a b'"),
        ("it's", None, "'it''s'"),
        ("Sheet1", Some("Sheet3"), "Sheet1:Sheet3"),
        // Cells in form: quoted.
        ("S1", Some("S3"), "'S1:S3'"),
        ("S 1", Some("Sheet3"), "'S 1:Sheet3'"),
        ("Sheet1", Some("S 3"), "'Sheet1:S 3'"),
        ("a'b", Some("c'd"), "'a''b:c''d'"),
        ("A1", None, "'A1'"),
        ("A1", Some("B2"), "'A1:B2'"),
    ];
    for (first, last, prefix) in cases {
        assert_eq!(sheet_prefix(first, last), prefix, "{first} {last:?}");
    }
}

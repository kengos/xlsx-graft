//! `include/xlsx_graft.h` defines every code the library uses with the
//! value the library gives it: the status codes, and the kinds of values,
//! sheets and sheet states.
//!
//! The header is hand-written, so it can drift from the Rust constants. A
//! binding branches on these numbers, and nothing else pins them. (Which
//! functions the header declares is checked against the built library by
//! `make engine:capi-abi`, not here: a Rust test links the rlib, which stays
//! green when an export loses its `no_mangle`.)

use std::path::PathBuf;
use xlsx_graft_capi::*;

#[test]
fn every_code_in_the_header_has_the_value_the_library_uses() {
    let statuses = [
        ("XLSX_GRAFT_OK", XLSX_GRAFT_OK),
        ("XLSX_GRAFT_ERR_NULL_ARG", XLSX_GRAFT_ERR_NULL_ARG),
        ("XLSX_GRAFT_ERR_PANIC", XLSX_GRAFT_ERR_PANIC),
        ("XLSX_GRAFT_ERR_IO", XLSX_GRAFT_ERR_IO),
        ("XLSX_GRAFT_ERR_INVALID_ARG", XLSX_GRAFT_ERR_INVALID_ARG),
        (
            "XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE",
            XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE,
        ),
        (
            "XLSX_GRAFT_ERR_SHEET_NOT_FOUND",
            XLSX_GRAFT_ERR_SHEET_NOT_FOUND,
        ),
        (
            "XLSX_GRAFT_ERR_SHEET_AMBIGUOUS",
            XLSX_GRAFT_ERR_SHEET_AMBIGUOUS,
        ),
        (
            "XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET",
            XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET,
        ),
        (
            "XLSX_GRAFT_ERR_SHEET_UNREADABLE",
            XLSX_GRAFT_ERR_SHEET_UNREADABLE,
        ),
    ];
    let kinds = [
        ("XLSX_GRAFT_VALUE_EMPTY", XLSX_GRAFT_VALUE_EMPTY),
        ("XLSX_GRAFT_VALUE_STRING", XLSX_GRAFT_VALUE_STRING),
        ("XLSX_GRAFT_VALUE_NUMBER", XLSX_GRAFT_VALUE_NUMBER),
        ("XLSX_GRAFT_VALUE_BOOLEAN", XLSX_GRAFT_VALUE_BOOLEAN),
        ("XLSX_GRAFT_VALUE_DATE", XLSX_GRAFT_VALUE_DATE),
        ("XLSX_GRAFT_VALUE_TIME", XLSX_GRAFT_VALUE_TIME),
        ("XLSX_GRAFT_VALUE_DATETIME", XLSX_GRAFT_VALUE_DATETIME),
        ("XLSX_GRAFT_VALUE_ELAPSED", XLSX_GRAFT_VALUE_ELAPSED),
        ("XLSX_GRAFT_VALUE_ERROR", XLSX_GRAFT_VALUE_ERROR),
        ("XLSX_GRAFT_SHEET_WORKSHEET", XLSX_GRAFT_SHEET_WORKSHEET),
        ("XLSX_GRAFT_SHEET_CHARTSHEET", XLSX_GRAFT_SHEET_CHARTSHEET),
        ("XLSX_GRAFT_SHEET_DIALOGSHEET", XLSX_GRAFT_SHEET_DIALOGSHEET),
        ("XLSX_GRAFT_SHEET_MACROSHEET", XLSX_GRAFT_SHEET_MACROSHEET),
        ("XLSX_GRAFT_SHEET_UNKNOWN", XLSX_GRAFT_SHEET_UNKNOWN),
        ("XLSX_GRAFT_STATE_VISIBLE", XLSX_GRAFT_STATE_VISIBLE),
        ("XLSX_GRAFT_STATE_HIDDEN", XLSX_GRAFT_STATE_HIDDEN),
        ("XLSX_GRAFT_STATE_VERY_HIDDEN", XLSX_GRAFT_STATE_VERY_HIDDEN),
    ];
    let expected: Vec<(String, i64)> = (statuses
        .iter()
        .map(|&(name, value)| (name, i64::from(value))))
    .chain(kinds.iter().map(|&(name, value)| (name, i64::from(value))))
    .map(|(name, value)| (name.to_owned(), value))
    .collect();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("include/xlsx_graft.h");
    let header = std::fs::read_to_string(path).expect("the header");
    // Every `#define XLSX_GRAFT_<NAME> <value>`; the include guard has no
    // value and is left out.
    let defines: Vec<(String, i64)> = header
        .lines()
        .filter_map(|line| line.strip_prefix("#define XLSX_GRAFT_"))
        .filter_map(|rest| {
            let mut parts = rest.split_whitespace();
            let name = format!("XLSX_GRAFT_{}", parts.next()?);
            Some((name, parts.next()?.parse().ok()?))
        })
        .collect();

    // Print the count before asserting on it: a parser that matched nothing
    // would otherwise satisfy every `contains` check below.
    println!("header codes found: {}", defines.len());
    assert_eq!(
        defines.len(),
        expected.len(),
        "the header defines {defines:?}, the library has {expected:?}"
    );
    for entry in &expected {
        assert!(
            defines.contains(entry),
            "the header must define {} as {}, got {defines:?}",
            entry.0,
            entry.1
        );
    }
}

//! `include/xlsx_graft.h` defines the status codes the library returns.
//!
//! The header is hand-written, so it can drift from the Rust constants. A
//! binding branches on these numbers, and nothing else pins them. (Which
//! functions the header declares is checked against the built library by
//! `make engine:capi-abi`, not here: a Rust test links the rlib, which stays
//! green when an export loses its `no_mangle`.)

use std::path::PathBuf;
use xlsx_graft_capi::{XLSX_GRAFT_ERR_NULL_ARG, XLSX_GRAFT_ERR_PANIC, XLSX_GRAFT_OK};

#[test]
fn every_status_code_in_the_header_has_the_value_the_library_returns() {
    let expected = [
        ("XLSX_GRAFT_OK", XLSX_GRAFT_OK),
        ("XLSX_GRAFT_ERR_NULL_ARG", XLSX_GRAFT_ERR_NULL_ARG),
        ("XLSX_GRAFT_ERR_PANIC", XLSX_GRAFT_ERR_PANIC),
    ];
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("include/xlsx_graft.h");
    let header = std::fs::read_to_string(path).expect("the header");
    let defines: Vec<(String, i32)> = header
        .lines()
        .filter_map(|line| line.strip_prefix("#define "))
        .filter(|rest| rest.starts_with("XLSX_GRAFT_OK") || rest.starts_with("XLSX_GRAFT_ERR_"))
        .filter_map(|rest| {
            let mut parts = rest.split_whitespace();
            let name = parts.next()?.to_string();
            Some((name, parts.next()?.parse().ok()?))
        })
        .collect();

    // Print the count before asserting on it: a parser that matched nothing
    // would otherwise satisfy every `contains` check below.
    println!("header status codes found: {}", defines.len());
    assert_eq!(
        defines.len(),
        expected.len(),
        "the header defines {defines:?}, the library has {expected:?}"
    );
    for (name, value) in expected {
        assert!(
            defines.contains(&(name.to_string(), value)),
            "the header must define {name} as {value}, got {defines:?}"
        );
    }
}

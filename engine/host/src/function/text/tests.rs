//! Tests for `FunctionText`: the text of a versioned function, a baseline
//! function and an unknown name, and the line of a cell no row has yet.

use super::{FunctionText, entry_line};
use crate::function::describe_function;
use crate::function::json::EntryJson;
use xlsx_graft_compat::{Entry, Excel, Fallback};

fn text(name: &str) -> String {
    FunctionText(&describe_function(name)).to_string()
}

#[test]
fn a_versioned_function_lists_its_support_its_requirements_and_its_sources() {
    let text = text("sequence");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "SEQUENCE — versioned");
    assert_eq!(lines[1], "  file name: _xlfn.SEQUENCE");
    assert_eq!(lines[2], "  safety: reviewed_safe");
    assert_eq!(lines[3], "  excel          2021    fallback name_error");
    assert_eq!(lines[6], "  google-sheets  current");
    assert_eq!(lines[7], "  numbers        not known");
    assert_eq!(lines[8], "  onlyoffice     8.0");
    assert_eq!(lines[9], "  requires dynamic_array");
    assert_eq!(
        lines[10],
        "    excel          2021    fallback legacy_array"
    );
    assert_eq!(lines[16], "  sources");
    assert!(lines[17].starts_with("    https://"));
    assert!(lines.iter().all(|line| line == &line.trim_end()));
}

#[test]
fn a_cell_says_no_a_bound_a_range_and_its_note() {
    let filterxml = text("filterxml");
    assert!(filterxml.contains("  safety: unreviewed\n"));
    assert!(filterxml.contains(
        "  excel-mac      no      note: relies on Windows; Excel for Mac does not return its result\n"
    ));
    assert!(text("XLOOKUP").contains("  numbers        <=14.4  note: Numbers for iCloud"));
    assert!(text("AGGREGATE").contains("  excel          >2007,<=2016 fallback name_error\n"));
}

#[test]
fn a_function_whose_prefix_is_not_known_says_so_and_a_dangerous_one_names_its_danger() {
    assert!(text("GROUPBY").contains("  file name: not known\n"));
    assert!(text("webservice").contains("  safety: dangerous (network)\n"));
}

#[test]
fn a_baseline_function_lists_its_file_name_safety_and_source() {
    assert_eq!(
        text("sum"),
        "SUM — baseline\n  file name: SUM\n  safety: reviewed_safe\n  sources\n    ECMA-376-1:2016 §18.17.7\n"
    );
    assert_eq!(
        text("ANCHORARRAY"),
        "ANCHORARRAY — baseline\n  file name: not known\n  safety: reviewed_safe\n  sources: none\n"
    );
}

#[test]
fn an_unknown_name_is_one_line() {
    assert_eq!(
        text("nope"),
        "NOPE — unknown: the table does not list it; verify denies it\n"
    );
}

#[test]
fn a_cell_that_was_removed_or_differs_says_so() {
    let entry = Entry::added(Excel::V2010)
        .removed(Excel::V365)
        .fallback(Fallback::NameError)
        .partial("differs");
    assert_eq!(
        entry_line("excel", &EntryJson::of(&entry)),
        "excel          2010    removed 365 partial fallback name_error note: differs"
    );
}

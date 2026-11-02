//! Unit tests for what every command prints the same way in text.

use super::{Problem, escape, problems};
use crate::workbook::Refused;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Limit, Location};
use xlsx_graft_read::Refusal;

#[test]
fn a_string_keeps_its_line_and_column() {
    let mut line = String::new();
    escape(&mut line, "a\\b\tc\nd\re");
    assert_eq!(line, r"a\\b\tc\nd\re");
}

#[test]
fn a_diagnostic_in_text_says_its_severity_code_place_and_message() {
    let mut diagnostics = Diagnostics::with_limit(Limit::default_of(1));
    let missing = Diagnostic::SharedStringMissing { index: 7 };
    let sheet = Location::default().with_part("/xl/worksheets/sheet1.xml");
    diagnostics.push(missing.clone(), sheet);
    diagnostics.push(missing.clone(), Location::default().with_entry("a.xml"));
    let records = diagnostics.finish();
    let lines: Vec<String> = (records.iter())
        .map(|record| Problem(record).to_string())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        format!(
            "error shared_string_missing in /xl/worksheets/sheet1.xml: {}",
            missing.message()
        )
    );
    // The truncation notice points nowhere in the package.
    assert!(lines[1].starts_with("notice diagnostics_truncated: "));
    let rels = Location::default().with_rels_part("/_rels/.rels");
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(missing, rels.with_relationship_id("rId1"));
    let records = diagnostics.finish();
    assert!(
        Problem(&records[0])
            .to_string()
            .contains(" in /_rels/.rels: ")
    );
}

#[test]
fn the_problems_are_a_line_each_and_the_error_last() {
    let mut diagnostics = Diagnostics::new();
    let missing = Diagnostic::SharedStringMissing { index: 7 };
    diagnostics.push(missing.clone(), Location::default());
    let records = diagnostics.finish();
    let mut err = Vec::new();
    let refused = Refused(Refusal::WorkbookUnreadable);
    problems(&mut err, &records, Some(&refused));
    let expected = format!(
        "xlsx-graft: error shared_string_missing: {}\nxlsx-graft: {}\n",
        missing.message(),
        Refusal::WorkbookUnreadable,
    );
    assert_eq!(String::from_utf8(err).unwrap(), expected);
}

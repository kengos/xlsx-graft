//! Unit tests for the JSON of a refusal; opening a workbook is
//! `tests/bin`'s, which gives each command a file.

use super::Refused;
use xlsx_graft_read::Refusal;

#[test]
fn a_refusal_prints_as_its_code_and_message() {
    let refused = Refused(Refusal::NoSuchName("X".to_owned()));
    let json = serde_json::to_value(&refused).unwrap();
    let expected =
        serde_json::json!({ "code": "sheet_not_found", "message": "no sheet is named \"X\"" });
    assert_eq!(json, expected);
}

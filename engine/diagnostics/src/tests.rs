//! Unit tests for the crate root: the severity's wire strings.

use super::Severity;

#[test]
fn severity_serializes_as_its_snake_case_name() {
    let wire = serde_json::to_string(&[Severity::Error, Severity::Warn, Severity::Notice]).unwrap();
    assert_eq!(wire, r#"["error","warn","notice"]"#);
}

#[test]
fn severity_as_str_is_its_wire_string() {
    for severity in [Severity::Error, Severity::Warn, Severity::Notice] {
        let wire = serde_json::to_string(&severity).unwrap();
        assert_eq!(wire, format!("\"{}\"", severity.as_str()));
    }
}

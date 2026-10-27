//! Contract tests for the code table: the golden JSON of every code, and the
//! wire strings it may and may not use.

use crate::{Code, Diagnostic, Diagnostics, Limit, Location, Severity};

/// Every wire string a code once had and no longer has. A removed code's
/// string is never reused, so a row may not bring one of these back.
const REMOVED: &[&str] = &[];

/// One diagnostic of `code`, with arguments and a location that show its
/// shape. The match has no wildcard, so a new row does not compile until it
/// has a sample here and an entry in `golden.json`.
fn sample(code: Code) -> (Diagnostic, Location) {
    match code {
        Code::DiagnosticsTruncated => (
            Diagnostic::DiagnosticsTruncated {
                dropped: 3,
                dropped_errors: 0,
                limit: Limit::default_of(1_000),
            },
            Location::default(),
        ),
    }
}

#[test]
fn every_code_serializes_as_its_golden_json() {
    let mut diagnostics = Diagnostics::with_limit(Limit::default_of(Code::ALL.len() as u64));
    for &code in Code::ALL {
        let (diagnostic, location) = sample(code);
        assert_eq!(diagnostic.code(), code);
        diagnostics.push(diagnostic, location);
    }
    let records = diagnostics.finish();
    let origin = records[0].origin();
    let json = serde_json::to_string_pretty(&records)
        .unwrap()
        .replace(&origin, "<origin>");
    assert_eq!(json, include_str!("golden.json").trim_end());
}

#[test]
fn wire_strings_are_unique_snake_case_and_never_removed_ones() {
    let wires: Vec<&str> = Code::ALL.iter().map(|code| code.as_str()).collect();
    let unique: std::collections::BTreeSet<&str> = wires.iter().copied().collect();
    assert_eq!(unique.len(), wires.len());
    let snake_case = |wire: &&str| {
        wire.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    };
    assert!(wires.iter().all(snake_case));
    assert!(!wires.iter().any(|wire| REMOVED.contains(wire)));
}

#[test]
fn a_code_displays_and_serializes_as_its_wire_string() {
    let code = Code::DiagnosticsTruncated;
    assert_eq!(code.to_string(), "diagnostics_truncated");
    assert_eq!(
        serde_json::to_string(&code).unwrap(),
        r#""diagnostics_truncated""#
    );
}

#[test]
fn a_diagnostic_carries_its_code_severity() {
    let (diagnostic, _) = sample(Code::DiagnosticsTruncated);
    assert_eq!(diagnostic.severity(), Severity::Notice);
    assert_eq!(Code::DiagnosticsTruncated.severity(), Severity::Notice);
}

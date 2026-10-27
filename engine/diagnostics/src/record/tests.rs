//! Unit tests for the JSON object a `Record` serializes as: its keys and
//! their order.

use crate::{Diagnostic, Diagnostics, Limit, Location};

#[test]
fn a_record_serializes_every_key_in_order() {
    let mut diagnostics = Diagnostics::new();
    let diagnostic = Diagnostic::DiagnosticsTruncated {
        dropped: 3,
        dropped_errors: 0,
        limit: Limit::default_of(1_000),
    };
    diagnostics.push(
        diagnostic,
        Location::default().with_part("/xl/workbook.xml"),
    );
    let record = &diagnostics.records()[0];
    let expected = format!(
        concat!(
            r#"{{"code":"diagnostics_truncated","severity":"notice","#,
            r#""message":"3 more diagnostic(s), 0 of them error(s), were found after the first 1000 and are not listed","#,
            r#""args":{{"dropped":3,"droppedErrors":0,"limit":1000}},"location":{{"part":"/xl/workbook.xml"}},"#,
            r#""decidedBy":{{"kind":"limit","name":"maxDiagnostics","value":1000,"source":"default"}},"#,
            r#""fix":null,"origin":"{}"}}"#,
        ),
        record.origin()
    );
    assert_eq!(serde_json::to_string(record).unwrap(), expected);
}

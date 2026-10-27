//! Unit tests for the reason enums: each prints as it serializes.

use super::EncodingReason;

#[test]
fn every_encoding_reason_prints_as_its_wire_string() {
    let reasons = [
        EncodingReason::OtherEncoding,
        EncodingReason::DeclarationMismatch,
        EncodingReason::InvalidUtf8,
        EncodingReason::InvalidUtf16,
        EncodingReason::Utf16WithoutBom,
    ];
    let printed: Vec<String> = reasons.iter().map(|r| format!("\"{r}\"")).collect();
    let wire = serde_json::to_string(&reasons).unwrap();
    assert_eq!(format!("[{}]", printed.join(",")), wire);
    assert_eq!(
        wire,
        r#"["other_encoding","declaration_mismatch","invalid_utf8","invalid_utf16","utf16_without_bom"]"#
    );
}

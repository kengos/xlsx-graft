//! Unit tests for the reason enums: each prints as it serializes.

use super::{
    EncodingReason, FormulaMalformedReason, FunctionDenial, InflateReason, PartKind,
    PartNameReason, SchemaReason, SheetNameReason, TargetList, TargetReason, ZipDirectoryReason,
    ZipHeaderField,
};
use serde::Serialize;
use std::fmt::Display;

/// Asserts that `reasons` serialize as `wire` and print as the same strings.
fn assert_wire<T: Serialize + Display>(reasons: &[T], wire: &str) {
    let printed: Vec<String> = reasons.iter().map(|r| format!("\"{r}\"")).collect();
    assert_eq!(format!("[{}]", printed.join(",")), wire);
    assert_eq!(serde_json::to_string(reasons).unwrap(), wire);
}

#[test]
fn every_encoding_reason_prints_as_its_wire_string() {
    assert_wire(
        &[
            EncodingReason::OtherEncoding,
            EncodingReason::DeclarationMismatch,
            EncodingReason::InvalidUtf8,
            EncodingReason::InvalidUtf16,
            EncodingReason::Utf16WithoutBom,
        ],
        r#"["other_encoding","declaration_mismatch","invalid_utf8","invalid_utf16","utf16_without_bom"]"#,
    );
}

#[test]
fn every_zip_directory_reason_prints_as_its_wire_string() {
    assert_wire(
        &[
            ZipDirectoryReason::MissingHeader,
            ZipDirectoryReason::ExtraBytes,
            ZipDirectoryReason::Zip64RecordMissing,
            ZipDirectoryReason::TrailingBytes,
            ZipDirectoryReason::EndRecordsDisagree,
            ZipDirectoryReason::Zip64FieldMissing,
        ],
        r#"["missing_header","extra_bytes","zip64_record_missing","trailing_bytes","end_records_disagree","zip64_field_missing"]"#,
    );
}

#[test]
fn every_zip_header_field_prints_as_its_wire_string() {
    assert_wire(
        &[
            ZipHeaderField::Signature,
            ZipHeaderField::Name,
            ZipHeaderField::Flags,
            ZipHeaderField::Method,
            ZipHeaderField::Crc32,
            ZipHeaderField::CompressedSize,
            ZipHeaderField::UncompressedSize,
            ZipHeaderField::Descriptor,
        ],
        r#"["signature","name","flags","method","crc32","compressed_size","uncompressed_size","descriptor"]"#,
    );
}

#[test]
fn every_inflate_reason_prints_as_its_wire_string() {
    assert_wire(
        &[
            InflateReason::CorruptData,
            InflateReason::EndedEarly,
            InflateReason::LongerThanDeclared,
        ],
        r#"["corrupt_data","ended_early","longer_than_declared"]"#,
    );
}

#[test]
fn every_part_name_reason_prints_as_its_wire_string() {
    assert_wire(
        &[
            PartNameReason::NotUtf8,
            PartNameReason::MissingLeadingSlash,
            PartNameReason::EmptySegment,
            PartNameReason::InvalidCharacter,
            PartNameReason::EncodedSlash,
            PartNameReason::EncodedUnreserved,
            PartNameReason::EndsWithDot,
            PartNameReason::HasScheme,
            PartNameReason::HasAuthority,
        ],
        r#"["not_utf8","missing_leading_slash","empty_segment","invalid_character","encoded_slash","encoded_unreserved","ends_with_dot","has_scheme","has_authority"]"#,
    );
}

#[test]
fn every_schema_reason_prints_as_its_wire_string() {
    assert_wire(
        &[
            SchemaReason::UnexpectedElement,
            SchemaReason::UnexpectedAttribute,
            SchemaReason::MissingAttribute,
            SchemaReason::MissingElement,
            SchemaReason::UnexpectedText,
            SchemaReason::InvalidValue,
        ],
        r#"["unexpected_element","unexpected_attribute","missing_attribute","missing_element","unexpected_text","invalid_value"]"#,
    );
}

#[test]
fn every_sheet_name_reason_prints_as_its_wire_string() {
    assert_wire(
        &[
            SheetNameReason::Empty,
            SheetNameReason::TooLong,
            SheetNameReason::Character,
            SheetNameReason::Apostrophe,
        ],
        r#"["empty","too_long","character","apostrophe"]"#,
    );
}

#[test]
fn every_part_kind_prints_as_its_wire_string() {
    assert_wire(
        &[
            PartKind::Macro,
            PartKind::ExternalLink,
            PartKind::Dde,
            PartKind::Ole,
            PartKind::Activex,
            PartKind::DataConnection,
            PartKind::WebPublish,
        ],
        r#"["macro","external_link","dde","ole","activex","data_connection","web_publish"]"#,
    );
}

#[test]
fn every_formula_malformed_reason_prints_as_its_wire_string() {
    assert_wire(
        &[
            FormulaMalformedReason::UndecodableEscape,
            FormulaMalformedReason::UpperCaseEscape,
            FormulaMalformedReason::UnclosedString,
            FormulaMalformedReason::UnclosedSheetName,
            FormulaMalformedReason::UnclosedBracket,
            FormulaMalformedReason::BracketsTooDeep,
            FormulaMalformedReason::UnknownCharacter,
            FormulaMalformedReason::StraySheetName,
            FormulaMalformedReason::Markup,
        ],
        r#"["undecodable_escape","upper_case_escape","unclosed_string","unclosed_sheet_name","unclosed_bracket","brackets_too_deep","unknown_character","stray_sheet_name","markup"]"#,
    );
}

#[test]
fn every_function_denial_prints_as_its_wire_string() {
    assert_wire(
        &[FunctionDenial::Unreviewed, FunctionDenial::Unknown],
        r#"["unreviewed","unknown"]"#,
    );
}

#[test]
fn every_target_list_and_reason_prints_as_its_wire_string() {
    assert_wire(
        &[TargetList::Targets, TargetList::BestEffort],
        r#"["targets","best_effort"]"#,
    );
    assert_wire(
        &[
            TargetReason::Malformed,
            TargetReason::UnknownApp,
            TargetReason::UnknownVersion,
            TargetReason::VersionRequired,
            TargetReason::VersionNotAllowed,
            TargetReason::Empty,
            TargetReason::Overlap,
        ],
        r#"["malformed","unknown_app","unknown_version","version_required","version_not_allowed","empty","overlap"]"#,
    );
}

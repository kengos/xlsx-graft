//! Contract tests for the code table: the golden JSON of every code, and the
//! wire strings it may and may not use.

use crate::{
    Code, DecidedBy, Diagnostic, Diagnostics, Echo, EncodingReason, InflateReason, Limit, Location,
    Severity, ZipDirectoryReason, ZipHeaderField,
};

/// Every wire string a code once had and no longer has. A removed code's
/// string is never reused, so a row may not bring one of these back.
const REMOVED: &[&str] = &[];

/// One diagnostic of `code`, with arguments and a location that show its
/// shape. The match has no wildcard, so a new row does not compile until it
/// has a sample here and an entry in `golden.json`.
#[allow(
    clippy::too_many_lines,
    reason = "a table of one arm per code, growing with the code table; a wildcard to split it would let a new code compile without a sample"
)]
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
        Code::XmlMalformed => (
            Diagnostic::XmlMalformed {
                offset: 42,
                detail: Echo::new("start tag not closed: `</a>` not found before end of input"),
            },
            Location::default().with_part("/_rels/.rels"),
        ),
        Code::XmlDoctype => (
            Diagnostic::XmlDoctype { offset: 39 },
            Location::default().with_part("/[Content_Types].xml"),
        ),
        Code::XmlEncodingUnsupported => (
            Diagnostic::XmlEncodingUnsupported {
                encoding: Echo::new("UTF-7"),
                reason: EncodingReason::OtherEncoding,
            },
            Location::default().with_part("/_rels/.rels"),
        ),
        Code::XmlTooDeep => (
            Diagnostic::XmlTooDeep {
                offset: 1_200,
                depth: 300,
                limit: Limit::default_of(256),
            },
            Location::default().with_part("/_rels/.rels"),
        ),
        Code::XmlTooManyAttributes => (
            Diagnostic::XmlTooManyAttributes {
                offset: 54,
                count: 1_025,
                limit: Limit::default_of(1_024),
            },
            Location::default().with_part("/_rels/.rels"),
        ),
        Code::XmlTooManyNamespaces => (
            Diagnostic::XmlTooManyNamespaces {
                offset: 0,
                count: 257,
                limit: Limit::default_of(256),
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::XmlTooLarge => (
            Diagnostic::XmlTooLarge {
                size: 16_777_217,
                limit: Limit::default_of(16_777_216),
            },
            Location::default().with_part("/_rels/.rels"),
        ),
        Code::NotAZip => (Diagnostic::NotAZip {}, Location::default()),
        Code::OleCompoundFile => (Diagnostic::OleCompoundFile {}, Location::default()),
        Code::ZipTruncated => (Diagnostic::ZipTruncated {}, Location::default()),
        Code::ZipDirectoryInvalid => (
            Diagnostic::ZipDirectoryInvalid {
                reason: ZipDirectoryReason::MissingHeader,
            },
            Location::default(),
        ),
        Code::ZipUnlistedData => (
            Diagnostic::ZipUnlistedData { size: 12 },
            Location::default().with_entry("xl/workbook.xml"),
        ),
        Code::ZipMultiDisk => (Diagnostic::ZipMultiDisk {}, Location::default()),
        Code::ZipPrependedData => (
            Diagnostic::ZipPrependedData { size: 4_096 },
            Location::default(),
        ),
        Code::ZipEntryCountExceeded => (
            Diagnostic::ZipEntryCountExceeded {
                count: 12_000,
                limit: Limit::default_of(10_000),
            },
            Location::default(),
        ),
        Code::ZipDirectoryTooLarge => (
            Diagnostic::ZipDirectoryTooLarge {
                size: 16_777_217,
                limit: Limit::default_of(16_777_216),
            },
            Location::default(),
        ),
        Code::EntryNameTooLong => (
            Diagnostic::EntryNameTooLong {
                length: 1_025,
                limit: Limit::default_of(1_024),
            },
            Location::default().with_entry("xl/worksheets/sheet1.xml"),
        ),
        Code::ZipEncryptedEntry => (
            Diagnostic::ZipEncryptedEntry {},
            Location::default().with_entry("xl/workbook.xml"),
        ),
        Code::ZipUnsupportedCompression => (
            Diagnostic::ZipUnsupportedCompression { method: 12 },
            Location::default().with_entry("xl/workbook.xml"),
        ),
        Code::ZipHeaderMismatch => (
            Diagnostic::ZipHeaderMismatch {
                field: ZipHeaderField::Name,
            },
            Location::default().with_entry("xl/workbook.xml"),
        ),
        Code::ZipEntriesOverlap => (
            Diagnostic::ZipEntriesOverlap {
                other: Echo::new("xl/styles.xml"),
            },
            Location::default().with_entry("xl/workbook.xml"),
        ),
        Code::ZipEntryOverlapsDirectory => (
            Diagnostic::ZipEntryOverlapsDirectory {},
            Location::default().with_entry("xl/workbook.xml"),
        ),
        Code::ZipEntryTooLarge => (
            Diagnostic::ZipEntryTooLarge {
                size: 2_147_483_649,
                limit: Limit::default_of(2_147_483_648),
            },
            Location::default().with_entry("xl/worksheets/sheet1.xml"),
        ),
        Code::ZipTotalTooLarge => (
            Diagnostic::ZipTotalTooLarge {
                size: 8_589_934_593,
                limit: Limit::default_of(8_589_934_592),
            },
            Location::default().with_entry("xl/worksheets/sheet9.xml"),
        ),
        Code::ZipRatioExceeded => (
            Diagnostic::ZipRatioExceeded {
                size: 1_073_741_824,
                compressed_size: 1_000_000,
                limit: Limit::default_of(1_000),
            },
            Location::default().with_entry("xl/worksheets/sheet1.xml"),
        ),
        Code::ZipCrcMismatch => (
            Diagnostic::ZipCrcMismatch {
                expected: 0x1234_abcd,
                actual: 0x0000_00ff,
            },
            Location::default().with_entry("xl/workbook.xml"),
        ),
        Code::ZipInflateFailed => (
            Diagnostic::ZipInflateFailed {
                reason: InflateReason::CorruptData,
            },
            Location::default().with_entry("xl/workbook.xml"),
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

#[test]
fn an_inflate_failure_cites_the_rule_its_reason_breaks() {
    let cited = |reason| Diagnostic::ZipInflateFailed { reason }.decided_by();
    assert_eq!(
        cited(InflateReason::CorruptData),
        Some(DecidedBy::standard("RFC 1951 §3.2"))
    );
    for reason in [InflateReason::EndedEarly, InflateReason::LongerThanDeclared] {
        assert_eq!(
            cited(reason),
            Some(DecidedBy::standard("PKWARE APPNOTE 6.3.10 §4.4.9"))
        );
    }
}

#[test]
fn a_ratio_fix_rounds_up_and_sends_an_empty_compressed_size_to_the_workbook() {
    let fix = |compressed_size| {
        Diagnostic::ZipRatioExceeded {
            size: 1_001,
            compressed_size,
            limit: Limit::default_of(1_000),
        }
        .fix()
    };
    assert_eq!(
        fix(1),
        Some(crate::Fix::policy("maxCompressionRatio: 1001"))
    );
    assert_eq!(fix(2), Some(crate::Fix::policy("maxCompressionRatio: 501")));
    assert_eq!(
        fix(0),
        Some(crate::Fix::workbook("save the workbook again from Excel"))
    );
}

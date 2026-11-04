//! Contract tests for the code table: the golden JSON of every code, and the
//! wire strings it may and may not use.

use crate::{
    Code, DecidedBy, Diagnostic, Diagnostics, Echo, EncodingReason, FormulaMalformedReason,
    FunctionDenial, InflateReason, Limit, Location, PartKind, PartNameReason, SchemaReason,
    Severity, SheetNameReason, TargetApp, TargetFallback, TargetList, TargetReason,
    ZipDirectoryReason, ZipHeaderField,
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
        Code::XmlNamespaceTooLong => (
            Diagnostic::XmlNamespaceTooLong {
                offset: 0,
                length: 257,
                limit: Limit::default_of(256),
            },
            Location::default().with_part("/xl/styles.xml"),
        ),
        Code::XmlEventTooLarge => (
            Diagnostic::XmlEventTooLarge {
                offset: 40,
                size: 16_777_217,
                limit: Limit::default_of(16_777_216),
            },
            Location::default().with_part("/xl/sharedStrings.xml"),
        ),
        Code::XmlTooLarge => (
            Diagnostic::XmlTooLarge {
                size: 16_777_217,
                limit: Limit::default_of(16_777_216),
            },
            Location::default().with_part("/_rels/.rels"),
        ),
        Code::XmlSchemaInvalid => (
            Diagnostic::XmlSchemaInvalid {
                offset: 120,
                reason: SchemaReason::UnexpectedAttribute,
                name: Echo::new("xml:base"),
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
        Code::EntryNameInvalid => (
            Diagnostic::EntryNameInvalid {
                reason: PartNameReason::InvalidCharacter,
            },
            Location::default().with_entry("xl\\workbook.xml"),
        ),
        Code::PartNameDuplicate => (
            Diagnostic::PartNameDuplicate {
                other: Echo::new("xl/workbook.xml"),
            },
            Location::default().with_entry("xl/Workbook.xml"),
        ),
        Code::PartNameDerived => (
            Diagnostic::PartNameDerived {
                other: Echo::new("/xl/workbook.xml"),
            },
            Location::default().with_part("/xl/workbook.xml/sheet1.xml"),
        ),
        Code::ContentTypesMissing => (Diagnostic::ContentTypesMissing {}, Location::default()),
        Code::ContentTypesDuplicateDefault => (
            Diagnostic::ContentTypesDuplicateDefault {
                extension: Echo::new("XML"),
            },
            Location::default().with_entry("[Content_Types].xml"),
        ),
        Code::ContentTypesDuplicateOverride => (
            Diagnostic::ContentTypesDuplicateOverride {
                part_name: Echo::new("/xl/workbook.xml"),
            },
            Location::default().with_entry("[Content_Types].xml"),
        ),
        Code::ContentTypesPartNameInvalid => (
            Diagnostic::ContentTypesPartNameInvalid {
                part_name: Echo::new("xl/workbook.xml"),
                reason: PartNameReason::InvalidCharacter,
            },
            Location::default().with_entry("[Content_Types].xml"),
        ),
        Code::ContentTypeInvalid => (
            Diagnostic::ContentTypeInvalid {
                content_type: Echo::new("application/xml; charset = utf-8"),
            },
            Location::default().with_entry("[Content_Types].xml"),
        ),
        Code::PartContentTypeMissing => (
            Diagnostic::PartContentTypeMissing {},
            Location::default().with_part("/xl/media/image1.emf"),
        ),
        Code::PartContentTypeMismatch => (
            Diagnostic::PartContentTypeMismatch {
                content_type: Echo::new("application/xml"),
            },
            Location::default().with_part("/xl/_rels/workbook.xml.rels"),
        ),
        Code::RelationshipsSourceMissing => (
            Diagnostic::RelationshipsSourceMissing {},
            Location::default().with_part("/xl/worksheets/_rels/sheet9.xml.rels"),
        ),
        Code::RelationshipsTooMany => (
            Diagnostic::RelationshipsTooMany {
                count: 100_001,
                limit: Limit::default_of(100_000),
            },
            Location::default().with_part("/xl/worksheets/_rels/sheet1.xml.rels"),
        ),
        Code::RelationshipsTotalTooMany => (
            Diagnostic::RelationshipsTotalTooMany {
                count: 1_000_100,
                limit: Limit::default_of(1_000_000),
            },
            Location::default().with_part("/xl/worksheets/_rels/sheet11.xml.rels"),
        ),
        Code::RelationshipIdDuplicate => (
            Diagnostic::RelationshipIdDuplicate {},
            Location::default()
                .with_rels_part("/xl/_rels/workbook.xml.rels")
                .with_relationship_id("rId1"),
        ),
        Code::RelationshipTargetInvalid => (
            Diagnostic::RelationshipTargetInvalid {
                target: Echo::new("http://example.com/a.xml"),
                reason: PartNameReason::HasScheme,
            },
            Location::default()
                .with_rels_part("/xl/_rels/workbook.xml.rels")
                .with_relationship_id("rId2"),
        ),
        Code::RelationshipTargetAboveRoot => (
            Diagnostic::RelationshipTargetAboveRoot {
                target: Echo::new("../../../xl/styles.xml"),
            },
            Location::default()
                .with_rels_part("/xl/_rels/workbook.xml.rels")
                .with_relationship_id("rId3"),
        ),
        Code::RelationshipTargetMissing => (
            Diagnostic::RelationshipTargetMissing {
                part_name: Echo::new("/xl/worksheets/sheet2.xml"),
            },
            Location::default()
                .with_rels_part("/xl/_rels/workbook.xml.rels")
                .with_relationship_id("rId4"),
        ),
        Code::RelationshipRelsPart => (
            Diagnostic::RelationshipRelsPart {},
            Location::default()
                .with_rels_part("/_rels/.rels")
                .with_relationship_id("rId5"),
        ),
        Code::MainDocumentMissing => (Diagnostic::MainDocumentMissing {}, Location::default()),
        Code::PartUnreferenced => (
            Diagnostic::PartUnreferenced {},
            Location::default().with_part("/xl/printerSettings/printerSettings1.bin"),
        ),
        Code::RelationshipTypeDuplicate => (
            Diagnostic::RelationshipTypeDuplicate {
                rel_type: Echo::new(
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings",
                ),
            },
            Location::default()
                .with_rels_part("/xl/_rels/workbook.xml.rels")
                .with_relationship_id("rId7"),
        ),
        Code::SharedStringsTooMany => (
            Diagnostic::SharedStringsTooMany {
                count: 16_777_217,
                limit: Limit::default_of(16_777_216),
            },
            Location::default().with_part("/xl/sharedStrings.xml"),
        ),
        Code::SharedStringsTooLarge => (
            Diagnostic::SharedStringsTooLarge {
                size: 4_294_967_300,
                limit: Limit::default_of(4_294_967_295),
            },
            Location::default().with_part("/xl/sharedStrings.xml"),
        ),
        Code::SharedStringTooLong => (
            Diagnostic::SharedStringTooLong {
                index: 3,
                length: 32_768,
            },
            Location::default().with_part("/xl/sharedStrings.xml"),
        ),
        Code::CellFormatsTooMany => (
            Diagnostic::CellFormatsTooMany {
                count: 65_537,
                limit: Limit::default_of(65_536),
            },
            Location::default().with_part("/xl/styles.xml"),
        ),
        Code::NumberFormatsTooMany => (
            Diagnostic::NumberFormatsTooMany {
                count: 65_537,
                limit: Limit::default_of(65_536),
            },
            Location::default().with_part("/xl/styles.xml"),
        ),
        Code::SheetNameDuplicate => (
            Diagnostic::SheetNameDuplicate {
                name: Echo::new("DATA"),
                other: Echo::new("Data"),
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::SheetPartDuplicate => (
            Diagnostic::SheetPartDuplicate {
                name: Echo::new("Sheet2"),
                other: Echo::new("Sheet1"),
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::SheetRelationshipMissing => (
            Diagnostic::SheetRelationshipMissing {
                relationship_id: Echo::new("rId9"),
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::SheetRelationshipTypeInvalid => (
            Diagnostic::SheetRelationshipTypeInvalid {
                rel_type: Echo::new(
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles",
                ),
            },
            Location::default()
                .with_rels_part("/xl/_rels/workbook.xml.rels")
                .with_relationship_id("rId3"),
        ),
        Code::SheetNameInvalid => (
            Diagnostic::SheetNameInvalid {
                name: Echo::new("Q1/Q2"),
                reason: SheetNameReason::Character,
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::SheetIdDuplicate => (
            Diagnostic::SheetIdDuplicate {
                name: Echo::new("Summary"),
                other: Echo::new("Data"),
                sheet_id: 5,
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::DefinedNameInvalid => (
            Diagnostic::DefinedNameInvalid {
                name: Echo::new("Bad Name"),
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::DefinedNameSheetIndexInvalid => (
            Diagnostic::DefinedNameSheetIndexInvalid {
                name: Echo::new("Total"),
                local_sheet_id: 99,
                sheets: 8,
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::DefinedNameDuplicate => (
            Diagnostic::DefinedNameDuplicate {
                name: Echo::new("BASE_NAME"),
                other: Echo::new("Base_Name"),
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::DefinedNameSheetMissing => (
            Diagnostic::DefinedNameSheetMissing {
                name: Echo::new("Base_Name"),
                sheet: Echo::new("NoSuch"),
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::TableRangeOverlap => (
            Diagnostic::TableRangeOverlap {
                name: Echo::new("T_Extra"),
                other: Echo::new("T_Data"),
            },
            Location::default().with_part("/xl/tables/table2.xml"),
        ),
        Code::TableNameDuplicate => (
            Diagnostic::TableNameDuplicate {
                name: Echo::new("Base_Name"),
                other: Echo::new("Base_Name"),
                defined_name: true,
            },
            Location::default().with_part("/xl/tables/table2.xml"),
        ),
        Code::TableIdDuplicate => (
            Diagnostic::TableIdDuplicate {
                name: Echo::new("T_Extra"),
                other: Echo::new("T_Data"),
                id: 1,
            },
            Location::default().with_part("/xl/tables/table2.xml"),
        ),
        Code::SharedStringMissing => (
            Diagnostic::SharedStringMissing { index: 12 },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::RowTooLarge => (
            Diagnostic::RowTooLarge {
                row: 7,
                size: 67_108_865,
                limit: Limit::default_of(67_108_864),
            },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::CellReferenceInvalid => (
            Diagnostic::CellReferenceInvalid {
                reference: Echo::new("XFE7"),
            },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::CellOrderInvalid => (
            Diagnostic::CellOrderInvalid {
                reference: Echo::new("B3"),
                previous: Echo::new("C3"),
            },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::CellStyleMissing => (
            Diagnostic::CellStyleMissing {
                reference: Echo::new("B2"),
                index: 99,
                count: 7,
            },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::ColumnStyleMissing => (
            Diagnostic::ColumnStyleMissing {
                columns: Echo::new("C:E"),
                index: 99,
                count: 7,
            },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::RowStyleMissing => (
            Diagnostic::RowStyleMissing {
                row: Echo::new("4"),
                index: 99,
                count: 7,
            },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::MergedCellsOverlap => (
            Diagnostic::MergedCellsOverlap {
                reference: Echo::new("H2:I3"),
                other: Echo::new("G1:H2"),
            },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::MergedCellsTooMany => (
            Diagnostic::MergedCellsTooMany {
                count: 300_000,
                limit: Limit::default_of(262_144),
            },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::InlineStringTooLong => (
            Diagnostic::InlineStringTooLong {
                reference: Echo::new("A1"),
                length: 32_768,
            },
            Location::default().with_part("/xl/worksheets/sheet1.xml"),
        ),
        Code::PartDenied => (
            Diagnostic::PartDenied {
                kind: PartKind::Macro,
            },
            Location::default().with_part("/xl/vbaProject.bin"),
        ),
        Code::PartForbidden => (
            Diagnostic::PartForbidden {
                kind: PartKind::ExternalLink,
                reason: Echo::new("no workbook may reach outside the host"),
            },
            Location::default()
                .with_rels_part("/xl/worksheets/_rels/sheet1.xml.rels")
                .with_relationship_id("rId1"),
        ),
        Code::FormulaMalformed => (
            Diagnostic::FormulaMalformed {
                reason: FormulaMalformedReason::UnclosedString,
                count: 1,
            },
            Location::default()
                .with_part("/xl/worksheets/sheet1.xml")
                .with_place("A1"),
        ),
        Code::FormulaTooLong => (
            Diagnostic::FormulaTooLong {
                size: 70_000,
                limit: Limit::default_of(65_536),
                count: 2,
            },
            Location::default()
                .with_part("/xl/worksheets/sheet1.xml")
                .with_place("C3"),
        ),
        Code::DefinedNamesTooMany => (
            Diagnostic::DefinedNamesTooMany {
                count: 70_000,
                limit: Limit::default_of(65_536),
            },
            Location::default().with_part("/xl/workbook.xml"),
        ),
        Code::FormulaDenied => (
            Diagnostic::FormulaDenied {
                kind: PartKind::Dde,
                count: 2,
            },
            Location::default()
                .with_part("/xl/worksheets/sheet1.xml")
                .with_place("A1"),
        ),
        Code::FormulaForbidden => (
            Diagnostic::FormulaForbidden {
                kind: PartKind::ExternalLink,
                reason: Echo::new("no workbook may reach outside the host"),
                count: 1,
            },
            Location::default()
                .with_part("/xl/workbook.xml")
                .with_place("definedName Rates"),
        ),
        Code::FunctionDenied => (
            Diagnostic::FunctionDenied {
                name: Echo::new("STOCKHISTORY"),
                denial: FunctionDenial::Unknown,
                count: 3,
            },
            Location::default()
                .with_part("/xl/worksheets/sheet1.xml")
                .with_place("B2"),
        ),
        Code::FunctionDangerous => (
            Diagnostic::FunctionDangerous {
                name: Echo::new("WEBSERVICE"),
                reason: Echo::new("reaches the network"),
                count: 1,
            },
            Location::default()
                .with_part("/xl/worksheets/sheet1.xml")
                .with_place("B2")
                .with_defined_name("Fetch"),
        ),
        Code::FunctionForbidden => (
            Diagnostic::FunctionForbidden {
                name: Echo::new("WEBSERVICE"),
                reason: Echo::new("no workbook may reach the network"),
                count: 1,
            },
            Location::default()
                .with_part("/xl/worksheets/sheet1.xml")
                .with_place("C3"),
        ),
        Code::TargetInvalid => (
            Diagnostic::TargetInvalid {
                list: TargetList::Targets,
                index: 1,
                entry: Echo::new("excel >= 2020"),
                reason: TargetReason::UnknownVersion,
                names: Echo::new("2007, 2010, 2013, 2016, 2019, 2021, 2024, 365"),
            },
            Location::default(),
        ),
        Code::TargetFunctionUnsupported => (
            Diagnostic::TargetFunctionUnsupported {
                name: Echo::new("XLOOKUP"),
                app: TargetApp::Excel,
                unsupported: Echo::new("2016, 2019"),
                unverified: Echo::new(""),
                requires: Echo::new(""),
                fallback: TargetFallback::NameError,
                count: 3,
            },
            Location::default()
                .with_part("/xl/worksheets/sheet1.xml")
                .with_place("B2"),
        ),
        Code::BestEffortFunctionUnsupported => (
            Diagnostic::BestEffortFunctionUnsupported {
                name: Echo::new("FILTER"),
                app: TargetApp::LibreOffice,
                unsupported: Echo::new("24.2"),
                unverified: Echo::new("24.8, 25.2"),
                requires: Echo::new("dynamic_array"),
                fallback: TargetFallback::Unknown,
                count: 1,
            },
            Location::default()
                .with_part("/xl/workbook.xml")
                .with_place("definedName Pick"),
        ),
        Code::TargetFeatureUnsupported => (
            Diagnostic::TargetFeatureUnsupported {
                name: Echo::new("threaded_comment"),
                app: TargetApp::Excel,
                unsupported: Echo::new("2019"),
                unverified: Echo::new("2024"),
                requires: Echo::new(""),
                fallback: TargetFallback::AsNote,
                count: 2,
            },
            Location::default().with_part("/xl/threadedComments/threadedComment1.xml"),
        ),
        Code::BestEffortFeatureUnsupported => (
            Diagnostic::BestEffortFeatureUnsupported {
                name: Echo::new("dynamic_array"),
                app: TargetApp::LibreOffice,
                unsupported: Echo::new(""),
                unverified: Echo::new("24.8, 25.2"),
                requires: Echo::new(""),
                fallback: TargetFallback::Unknown,
                count: 1,
            },
            Location::default().with_part("/xl/metadata.xml"),
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

#[test]
fn a_shared_strings_size_fix_raises_the_limit_until_32_bits_hold_no_more() {
    let fix = |size, limit| {
        Diagnostic::SharedStringsTooLarge {
            size,
            limit: Limit::default_of(limit),
        }
        .fix()
    };
    assert_eq!(
        fix(1_025, 1_024),
        Some(crate::Fix::policy("maxSharedStringsBytes: 1025"))
    );
    assert_eq!(
        fix(4_294_967_295, 4_294_967_294),
        Some(crate::Fix::policy("maxSharedStringsBytes: 4294967295"))
    );
    for (size, limit) in [
        (4_294_967_296, 4_294_967_295),
        (4_294_967_300, 4_294_967_290),
    ] {
        assert_eq!(
            fix(size, limit),
            Some(crate::Fix::workbook(
                "split the workbook: no limit admits more than 4 GiB of shared strings"
            ))
        );
    }
}

#[test]
fn a_function_denial_says_why_in_its_message() {
    let message = |denial| {
        Diagnostic::FunctionDenied {
            name: Echo::new("F"),
            denial,
            count: 1,
        }
        .message()
    };
    assert!(message(FunctionDenial::Unreviewed).contains("has not reviewed yet"));
    assert!(message(FunctionDenial::Unknown).contains("does not know"));
}

#[test]
fn a_malformed_formula_cites_what_decides_its_reason() {
    let malformed = |reason| Diagnostic::FormulaMalformed { reason, count: 1 };
    let standard = |reference| Some(DecidedBy::standard(reference));
    let again = Some(crate::Fix::workbook("save the workbook again from Excel"));
    let cases = [
        (
            FormulaMalformedReason::UndecodableEscape,
            standard("ECMA-376-1:2016 §22.9.2.19"),
            &again,
        ),
        (
            FormulaMalformedReason::UpperCaseEscape,
            Some(DecidedBy::Default {}),
            &again,
        ),
        (
            FormulaMalformedReason::UnclosedString,
            standard("ECMA-376-1:2016 §18.17.2"),
            &again,
        ),
        (
            FormulaMalformedReason::UnclosedSheetName,
            standard("ECMA-376-1:2016 §18.17.2"),
            &again,
        ),
        (
            FormulaMalformedReason::UnclosedBracket,
            standard("ECMA-376-1:2016 §18.17.2"),
            &again,
        ),
        (
            FormulaMalformedReason::UnknownCharacter,
            standard("ECMA-376-1:2016 §18.17.2"),
            &again,
        ),
        (
            FormulaMalformedReason::StraySheetName,
            standard("ECMA-376-1:2016 §18.17.2"),
            &again,
        ),
        (
            FormulaMalformedReason::Markup,
            Some(DecidedBy::Default {}),
            &again,
        ),
    ];
    for (reason, decided_by, fix) in cases {
        assert_eq!(malformed(reason).decided_by(), decided_by, "{reason}");
        assert_eq!(&malformed(reason).fix(), fix, "{reason}");
    }
    let deep = malformed(FormulaMalformedReason::BracketsTooDeep);
    assert_eq!(deep.decided_by(), Some(DecidedBy::Default {}));
    assert_eq!(
        deep.fix(),
        Some(crate::Fix::workbook(
            "nest the formula's brackets less deeply"
        ))
    );
}

#[test]
fn a_sheet_name_message_says_what_is_wrong_with_it() {
    let message = |name: &str, reason| {
        Diagnostic::SheetNameInvalid {
            name: Echo::new(name),
            reason,
        }
        .message()
    };
    assert_eq!(
        message("", SheetNameReason::Empty),
        "a sheet's name is empty"
    );
    assert_eq!(
        message("abc", SheetNameReason::TooLong),
        "the sheet name abc is longer than 31 characters, counted in UTF-16 code units"
    );
    assert_eq!(
        message("'abc", SheetNameReason::Apostrophe),
        "the sheet name 'abc starts or ends with '"
    );
}

#[test]
fn a_table_name_message_says_whether_the_other_is_a_table_or_a_defined_name() {
    let message = |defined_name| {
        Diagnostic::TableNameDuplicate {
            name: Echo::new("t_data"),
            other: Echo::new("T_Data"),
            defined_name,
        }
        .message()
    };
    assert_eq!(
        message(false),
        "the table t_data has the same name as the table T_Data, letter case aside"
    );
    assert_eq!(
        message(true),
        "the table t_data has the same name as the defined name T_Data, letter case aside"
    );
}

#[test]
fn a_target_message_names_the_list_and_says_what_is_wrong_with_the_entry() {
    let message = |list, reason, names: &str| {
        Diagnostic::TargetInvalid {
            list,
            index: 0,
            entry: Echo::new("x"),
            reason,
            names: Echo::new(names),
        }
        .message()
    };
    let targets = |reason, names| message(TargetList::Targets, reason, names);
    assert_eq!(
        targets(TargetReason::Malformed, ""),
        "targets[0] `x` is not written as `name`, `name version` or `name OP version`, OP one of >=, >, <=, <"
    );
    assert_eq!(
        targets(TargetReason::UnknownApp, "excel, numbers"),
        "targets[0] `x` names an application the engine does not know; it knows excel, numbers"
    );
    assert_eq!(
        targets(TargetReason::VersionRequired, "2016, 365"),
        "targets[0] `x` names no version, which the application needs; the versions the engine knows are 2016, 365"
    );
    assert_eq!(
        targets(TargetReason::VersionNotAllowed, "google-sheets"),
        "targets[0] `x` gives a version to an application that has none; write it as google-sheets"
    );
    assert_eq!(
        targets(TargetReason::Empty, "2016, 365"),
        "targets[0] `x` holds none of the versions the engine knows, which are 2016, 365"
    );
    assert_eq!(
        message(TargetList::BestEffort, TargetReason::Overlap, "2016, 365"),
        "bestEffort[0] `x` holds 2016, 365, which targets holds as well; a version is either required or best effort"
    );
    let unknown = Diagnostic::TargetInvalid {
        list: TargetList::BestEffort,
        index: 2,
        entry: Echo::new("excel 2020"),
        reason: TargetReason::UnknownVersion,
        names: Echo::new("2016, 365"),
    };
    assert_eq!(unknown.decided_by(), Some(DecidedBy::Template {}));
    assert_eq!(unknown.fix(), None);
}

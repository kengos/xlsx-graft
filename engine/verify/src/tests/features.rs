//! The features of the compatibility table `verify` finds as it reads the
//! parts, by media type, relationship type and element, and how each is
//! checked against the targets.

use super::{Book, Reader, Table, WORKSHEET, verify};
use crate::{Limits, Rules, Targets};
use std::io::Cursor;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics};

/// The SpreadsheetML namespace.
const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
/// Microsoft's relationship types of the parts that hold a feature, up to
/// the year.
const MS: &str = "http://schemas.microsoft.com/office/";
/// The metadata part, which holds the dynamic array properties.
const METADATA: &str = "/xl/metadata.xml";
/// A metadata part whose `XLDAPR` future metadata says `fDynamic`.
const DYNAMIC: &str = r#"<metadata xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:xda="http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray"><futureMetadata name="XLDAPR" count="1"><bk><extLst><ext uri="{bdbb8cdc-fa1e-496e-a857-3c3f30c029c3}"><xda:dynamicArrayProperties fDynamic="1" fCollapsed="0"/></ext></extLst></bk></futureMetadata></metadata>"#;

/// What verifying `book` against `targets` finds, one line each: the part,
/// and what.
fn found(book: Book, targets: &Targets) -> Vec<String> {
    let reader = Reader {
        inner: Cursor::new(book.zip()),
        poisoned: (0, 0),
    };
    let rules = Rules::default().allow_function("FILTER");
    let mut diagnostics = Diagnostics::new();
    verify(
        reader,
        &Limits::default(),
        &rules,
        &Table,
        targets,
        &mut diagnostics,
    )
    .unwrap();
    (diagnostics.finish().into_iter())
        .map(|record| {
            let part = (record.location.part).map_or_else(String::new, |p| p.as_str().to_owned());
            format!("{part}: {}", brief(&record.diagnostic))
        })
        .collect()
}

/// What `diagnostic` says, briefly.
fn brief(diagnostic: &Diagnostic) -> String {
    let (list, name, app, unsupported, unverified, count) = match diagnostic {
        Diagnostic::TargetFeatureUnsupported {
            name,
            app,
            unsupported,
            unverified,
            count,
            ..
        } => ("targets", name, app, unsupported, unverified, count),
        Diagnostic::BestEffortFeatureUnsupported {
            name,
            app,
            unsupported,
            unverified,
            count,
            ..
        } => ("bestEffort", name, app, unsupported, unverified, count),
        Diagnostic::TargetFunctionUnsupported {
            name,
            app,
            unsupported,
            unverified,
            count,
            ..
        } => ("targets fn", name, app, unsupported, unverified, count),
        other => return format!("{other:?}"),
    };
    format!("{list} {name} {app} [{unsupported}] [{unverified}] x{count}")
}

/// The targets of the two lists.
fn targets(targets: &[&str], best_effort: &[&str]) -> Targets {
    let mut diagnostics = Diagnostics::new();
    Targets::parse(targets, best_effort, &mut diagnostics).unwrap()
}

/// A workbook of one sheet holding sparklines, and a part of each kind
/// that holds a feature, some named by their media type, some by the
/// relationship that targets them and some by an element.
fn every_feature() -> Book {
    let sheet = "/xl/worksheets/sheet1.xml";
    let worksheet = format!(r#"<worksheet xmlns="{MAIN}"><sheetData/>{SPARKLINES}</worksheet>"#);
    let x15 = "http://schemas.microsoft.com/office/spreadsheetml/2010/11/main";
    let x14 = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
    let table_slicer = format!(
        r#"<slicerCacheDefinition xmlns="{x14}" name="S" sourceName="C"><extLst><ext uri="{{2F2917AC-EB37-4324-AD4E-5DD8C200BD13}}"><x15:tableSlicerCache xmlns:x15="{x15}" tableId="1" column="1"/></ext></extLst></slicerCacheDefinition>"#
    );
    let rich = "http://schemas.microsoft.com/office/spreadsheetml/2017/richdata";
    let structures = format!(
        r#"<rvStructures xmlns="{rich}" count="1"><s t="_localImage"><k n="_rvRel:LocalImageIdentifier" t="i"/></s></rvStructures>"#
    );
    Book::new()
        .part(sheet, WORKSHEET, &worksheet)
        .rel("/xl/workbook.xml", "worksheet", sheet)
        .part(METADATA, "application/xml", DYNAMIC)
        .rel("/xl/workbook.xml", "sheetMetadata", METADATA)
        .part(
            "/xl/threadedComments/threadedComment1.xml",
            "application/vnd.ms-excel.threadedcomments+xml",
            "<ThreadedComments/>",
        )
        .rel(sheet, OTHER, "/xl/threadedComments/threadedComment1.xml")
        .part(
            "/xl/threadedComments/threadedComment2.xml",
            "application/xml",
            "<x/>",
        )
        .rel(
            sheet,
            &format!("{MS}2017/10/relationships/threadedComment"),
            "/xl/threadedComments/threadedComment2.xml",
        )
        .part(
            "/xl/slicerCaches/slicerCache1.xml",
            "application/vnd.ms-excel.slicerCache+xml",
            "<x/>",
        )
        .rel(
            "/xl/workbook.xml",
            OTHER,
            "/xl/slicerCaches/slicerCache1.xml",
        )
        .part(
            "/xl/slicerCaches/slicerCache2.xml",
            "application/xml",
            &table_slicer,
        )
        .rel(
            "/xl/workbook.xml",
            &format!("{MS}2007/relationships/slicerCache"),
            "/xl/slicerCaches/slicerCache2.xml",
        )
        .part(
            "/xl/timelineCaches/timelineCache1.xml",
            "application/xml",
            "<x/>",
        )
        .rel(
            "/xl/workbook.xml",
            &format!("{MS}2010/relationships/TimelineCache"),
            "/xl/timelineCaches/timelineCache1.xml",
        )
        .part(
            "/xl/model/item.data",
            "application/vnd.openxmlformats-officedocument.model+data",
            "\u{1}not XML",
        )
        .rel("/xl/workbook.xml", OTHER, "/xl/model/item.data")
        .part(
            "/xl/richData/rdrichvaluestructure.xml",
            "application/xml",
            &structures,
        )
        .rel(
            METADATA,
            &format!("{MS}2017/06/relationships/rdRichValueStructure"),
            "/xl/richData/rdrichvaluestructure.xml",
        )
}

/// A relationship type that names no feature.
const OTHER: &str = "urn:test:other";

/// A sheet's `extLst` that holds a sparkline group of one sparkline.
const SPARKLINES: &str = r#"<extLst><ext uri="{05C60535-1F16-4fd2-B633-F4F36F0B64E0}"><x14:sparklineGroups xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"><x14:sparklineGroup><x14:sparklines><x14:sparkline/></x14:sparklines></x14:sparklineGroup></x14:sparklineGroups></ext></extLst>"#;

#[test]
fn every_feature_is_found_and_checked_against_both_lists() {
    let every = |targets: &Targets| found(every_feature(), targets);
    assert_eq!(
        every(&targets(&["excel 2007"], &["libreoffice 24.2"])),
        [
            "/xl/worksheets/sheet1.xml: targets sparkline excel [2007] [] x1",
            "/xl/metadata.xml: targets dynamic_array excel [2007] [] x1",
            "/xl/metadata.xml: bestEffort dynamic_array libreoffice [] [24.2] x1",
            "/xl/threadedComments/threadedComment1.xml: targets threaded_comment excel [2007] [] x2",
            "/xl/threadedComments/threadedComment1.xml: bestEffort threaded_comment libreoffice [] [24.2] x2",
            "/xl/slicerCaches/slicerCache1.xml: targets slicer excel [] [2007] x1",
            "/xl/slicerCaches/slicerCache1.xml: bestEffort slicer libreoffice [] [24.2] x1",
            "/xl/slicerCaches/slicerCache2.xml: targets table_slicer excel [] [2007] x1",
            "/xl/slicerCaches/slicerCache2.xml: bestEffort table_slicer libreoffice [] [24.2] x1",
            "/xl/timelineCaches/timelineCache1.xml: targets timeline excel [] [2007] x1",
            "/xl/timelineCaches/timelineCache1.xml: bestEffort timeline libreoffice [] [24.2] x1",
            "/xl/model/item.data: targets data_model excel [] [2007] x1",
            "/xl/model/item.data: bestEffort data_model libreoffice [] [24.2] x1",
            "/xl/richData/rdrichvaluestructure.xml: targets cell_image excel [] [2007] x1",
            "/xl/richData/rdrichvaluestructure.xml: bestEffort cell_image libreoffice [] [24.2] x1",
        ]
    );
    assert_eq!(
        every(&Targets::default()),
        [
            "/xl/worksheets/sheet1.xml: targets sparkline excel_mac [] [2024, 365] x1",
            "/xl/threadedComments/threadedComment1.xml: targets threaded_comment excel [] [2024] x2",
            "/xl/threadedComments/threadedComment1.xml: targets threaded_comment excel_mac [] [2024] x2",
            "/xl/model/item.data: targets data_model excel_mac [2024, 365] [] x1",
            "/xl/richData/rdrichvaluestructure.xml: targets cell_image excel [] [2024] x1",
            "/xl/richData/rdrichvaluestructure.xml: targets cell_image excel_mac [] [2024] x1",
        ]
    );
    assert_eq!(every(&targets(&["excel 365"], &[])), Vec::<String>::new());
}

/// A cell whose `FILTER` spills, as Excel writes one.
const SPILLED: &str =
    r#"<c r="A1" cm="1"><f t="array" ref="A1:A3">_xlfn._xlws.FILTER(B1:B3,B1:B3&gt;1)</f></c>"#;

#[test]
fn a_function_is_not_blamed_for_a_feature_the_workbook_was_found_to_use() {
    let lo = targets(&["libreoffice 24.8"], &[]);
    let sheet = || {
        let xml =
            format!(r#"<worksheet xmlns="{MAIN}"><sheetData>{SPILLED}</sheetData></worksheet>"#);
        Book::new()
            .part("/xl/worksheets/sheet1.xml", WORKSHEET, &xml)
            .rel("/xl/workbook.xml", "worksheet", "/xl/worksheets/sheet1.xml")
    };
    let with_metadata = || {
        (sheet().part(METADATA, "application/xml", DYNAMIC)).rel(
            "/xl/workbook.xml",
            "sheetMetadata",
            METADATA,
        )
    };
    assert_eq!(
        found(sheet(), &lo),
        ["/xl/worksheets/sheet1.xml: targets fn FILTER libreoffice [] [24.8] x1"]
    );
    assert_eq!(
        found(with_metadata(), &lo),
        ["/xl/metadata.xml: targets dynamic_array libreoffice [] [24.8] x1"]
    );
    assert_eq!(
        found(with_metadata(), &targets(&["libreoffice 24.2"], &[])),
        [
            "/xl/worksheets/sheet1.xml: targets fn FILTER libreoffice [24.2] [] x1",
            "/xl/metadata.xml: targets dynamic_array libreoffice [] [24.2] x1",
        ]
    );
}

#[test]
fn a_part_no_relationship_targets_holds_no_feature() {
    let book = Book::new()
        .part(
            "/xl/threadedComments/threadedComment1.xml",
            "application/vnd.ms-excel.threadedcomments+xml",
            "<ThreadedComments/>",
        )
        .part(METADATA, "application/xml", DYNAMIC);
    assert_eq!(
        found(book, &targets(&["excel 2007"], &[])),
        [
            "/xl/threadedComments/threadedComment1.xml: PartUnreferenced",
            "/xl/metadata.xml: PartUnreferenced",
        ]
    );
}

#[test]
fn an_element_in_a_part_of_another_role_counts_for_nothing() {
    let custom = format!(
        r#"<root xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main">{SPARKLINES}</root>"#
    );
    let book = (Book::new().part("/customXml/item1.xml", "application/xml", &custom))
        .rel("/xl/workbook.xml", "customXml", "/customXml/item1.xml")
        .part("/xl/m.xml", "application/xml", DYNAMIC)
        .rel("/xl/workbook.xml", OTHER, "/xl/m.xml");
    assert_eq!(
        found(book, &targets(&["excel 2007"], &[])),
        Vec::<String>::new()
    );
}

#[test]
fn a_part_whose_relationships_may_be_lost_is_read_by_its_media_type() {
    let book = Book::new()
        .part(
            "/xl/threadedComments/threadedComment1.xml",
            "application/vnd.ms-excel.threadedcomments+xml",
            "<ThreadedComments/>",
        )
        .part(
            "/xl/worksheets/_rels/sheet1.xml.rels",
            "application/vnd.openxmlformats-package.relationships+xml",
            "<Relationships",
        );
    assert_eq!(
        found(book, &targets(&["excel 2007"], &[])),
        [
            "/xl/worksheets/_rels/sheet1.xml.rels: RelationshipsSourceMissing",
            "/xl/threadedComments/threadedComment1.xml: targets threaded_comment excel [2007] [] x1",
        ]
    );
}

#[test]
fn a_part_only_a_part_nothing_reaches_targets_holds_no_feature() {
    let comments = "/xl/threadedComments/threadedComment1.xml";
    let book = Book::new()
        .part("/xl/worksheets/sheet9.xml", WORKSHEET, "<worksheet/>")
        .part(
            comments,
            "application/vnd.ms-excel.threadedcomments+xml",
            "<x/>",
        )
        .rel(
            "/xl/worksheets/sheet9.xml",
            &format!("{MS}2017/10/relationships/threadedComment"),
            comments,
        );
    assert_eq!(
        found(book, &targets(&["excel 2007"], &[])),
        ["/xl/worksheets/sheet9.xml: PartUnreferenced"]
    );
}

#[test]
fn with_relationships_lost_a_relationship_from_any_part_names_a_role() {
    let comments = "/xl/threadedComments/threadedComment1.xml";
    let book = Book::new()
        .part("/xl/worksheets/sheet9.xml", WORKSHEET, "<worksheet/>")
        .part(comments, "application/xml", "<x/>")
        .rel(
            "/xl/worksheets/sheet9.xml",
            &format!("{MS}2017/10/relationships/threadedComment"),
            comments,
        )
        .part(
            "/xl/_rels/lost.xml.rels",
            "application/vnd.openxmlformats-package.relationships+xml",
            "<Relationships",
        );
    assert_eq!(
        found(book, &targets(&["excel 2007"], &[])),
        [
            "/xl/_rels/lost.xml.rels: RelationshipsSourceMissing",
            "/xl/threadedComments/threadedComment1.xml: targets threaded_comment excel [2007] [] x1",
        ]
    );
}

#[test]
fn parts_that_target_each_other_are_followed_once() {
    let (sheet, comments) = ("/xl/worksheets/sheet1.xml", "/xl/threadedComments/c.xml");
    let book = Book::new()
        .part(sheet, WORKSHEET, "<worksheet/>")
        .rel("/xl/workbook.xml", "worksheet", sheet)
        .part(comments, "application/xml", "<x/>")
        .rel(
            sheet,
            &format!("{MS}2017/10/relationships/threadedComment"),
            comments,
        )
        .rel(comments, OTHER, sheet)
        .rel(comments, OTHER, "/xl/workbook.xml");
    assert_eq!(
        found(book, &targets(&["excel 2007"], &[])),
        ["/xl/threadedComments/c.xml: targets threaded_comment excel [2007] [] x1"]
    );
}

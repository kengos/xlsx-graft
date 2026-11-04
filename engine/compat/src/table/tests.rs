//! Unit tests for the table's data: the checks every row must pass, and
//! the helpers its rows are written with.

use super::sources::MS_XLFN;
use super::{FUNCTIONS, Feature, YES, between, le, no, on, s, unk, xl};
use crate::app::{Excel, ExcelMac, GoogleSheets, LibreOffice, Numbers, OnlyOffice, Version};
use crate::entry::{Added, Entry, Fallback, Status};
use crate::row::Row;
use std::collections::HashSet;

/// Every row: the functions', then the features'.
fn rows() -> Vec<&'static Row> {
    FUNCTIONS
        .iter()
        .chain(Feature::ALL.iter().map(|feature| feature.row()))
        .collect()
}

#[test]
fn every_function_is_named_in_upper_case_without_a_prefix() {
    for row in FUNCTIONS {
        let name = row.name;
        assert!(
            name.starts_with(|c: char| c.is_ascii_uppercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '.'),
            "{name}"
        );
        assert!(!name.contains("_XL"), "{name}");
    }
}

#[test]
fn no_name_has_two_rows() {
    let mut seen = HashSet::new();
    for row in rows() {
        assert!(
            seen.insert(row.name.to_ascii_uppercase()),
            "{} has two rows",
            row.name
        );
    }
    assert_eq!(seen.len(), FUNCTIONS.len() + Feature::ALL.len());
}

#[test]
fn every_row_cites_its_sources_once_each() {
    for row in rows() {
        assert!(!row.sources.is_empty(), "{} cites no source", row.name);
        let unique: HashSet<_> = row.sources.iter().collect();
        assert_eq!(
            unique.len(),
            row.sources.len(),
            "{} cites a source twice",
            row.name
        );
        for source in row.sources {
            assert!(source.starts_with("https://"), "{}: {source}", row.name);
        }
    }
}

#[test]
fn every_feature_is_its_own_row() {
    for &feature in Feature::ALL {
        assert_eq!(feature.row().name, feature.as_str());
        assert!(
            feature.row().requires.is_empty(),
            "{} requires a feature",
            feature.as_str()
        );
    }
    let names: Vec<_> = Feature::ALL
        .iter()
        .map(|feature| feature.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "dynamic_array",
            "threaded_comment",
            "slicer",
            "table_slicer",
            "timeline",
            "data_model",
            "sparkline",
            "cell_image"
        ]
    );
}

#[test]
fn a_function_requires_each_feature_once() {
    for row in FUNCTIONS {
        let unique: HashSet<_> = row.requires.iter().collect();
        assert_eq!(unique.len(), row.requires.len(), "{}", row.name);
    }
}

/// Each cell of a row, as when it was added, when it was removed and what
/// an older version shows, with each version written as its place in its
/// application's list.
fn cells(row: &Row) -> Vec<(Added<usize>, Option<usize>, Option<Fallback>)> {
    fn cell<V: Version>(entry: Entry<V>) -> (Added<usize>, Option<usize>, Option<Fallback>) {
        let index = |version: V| V::ALL.iter().position(|&v| v == version).unwrap();
        let added = match entry.added {
            Added::In(v) => Added::In(index(v)),
            Added::AtMost(v) => Added::AtMost(index(v)),
            Added::Between(missing, present) => Added::Between(index(missing), index(present)),
            Added::Never => Added::Never,
            Added::Unknown => Added::Unknown,
        };
        (added, entry.removed.map(index), entry.fallback)
    }
    let support = row.support;
    vec![
        cell(support.excel),
        cell(support.excel_mac),
        cell(support.libreoffice),
        cell(support.google_sheets),
        cell(support.numbers),
        cell(support.onlyoffice),
    ]
}

#[test]
fn every_cell_is_in_order() {
    for row in rows() {
        for (added, removed, _) in cells(row) {
            if let Added::Between(missing, present) = added {
                assert!(missing < present, "{}", row.name);
            }
            let first = match added {
                Added::In(v) | Added::AtMost(v) | Added::Between(_, v) => Some(v),
                Added::Never | Added::Unknown => None,
            };
            if let Some(removed) = removed {
                assert!(first.is_some_and(|first| first < removed), "{}", row.name);
            }
        }
    }
}

#[test]
fn a_name_error_fallback_cites_the_page_that_says_so() {
    for row in rows() {
        if cells(row)
            .iter()
            .any(|&(_, _, fallback)| fallback == Some(Fallback::NameError))
        {
            assert!(row.sources.contains(&MS_XLFN), "{}", row.name);
        }
    }
}

#[test]
fn every_function_row_knows_excel_has_it_today() {
    // A function row exists because Excel added it after 2007: Excel 365
    // has it, and Excel 2007 does not or no source says.
    for row in FUNCTIONS {
        let excel = row.support.excel;
        assert_eq!(excel.at(Excel::V365), Status::Supported, "{}", row.name);
        assert_ne!(excel.at(Excel::V2007), Status::Supported, "{}", row.name);
        assert_eq!(excel.fallback, Some(Fallback::NameError), "{}", row.name);
    }
}

#[test]
fn the_table_counts_its_functions_by_the_excel_that_added_them() {
    let count = |version: Excel| {
        FUNCTIONS
            .iter()
            .filter(|row| row.support.excel.added == Added::In(version))
            .count()
    };
    assert_eq!(
        [
            Excel::V2010,
            Excel::V2013,
            Excel::V2016,
            Excel::V2019,
            Excel::V2021,
            Excel::V2024,
            Excel::V365
        ]
        .map(count),
        [56, 50, 5, 6, 11, 23, 12]
    );
    let undated = |present: Excel| {
        let added = Added::Between(Excel::V2007, present);
        FUNCTIONS
            .iter()
            .filter(|row| row.support.excel.added == added)
            .count()
    };
    assert_eq!([Excel::V2016, Excel::V365].map(undated), [1, 4]);
    assert_eq!(FUNCTIONS.len(), 168);
}

#[test]
fn the_helpers_build_the_cells_they_name() {
    // The rows call these in constant context; call them here at run time.
    assert_eq!(on(Numbers::V14_4), Entry::added(Numbers::V14_4));
    assert_eq!(
        xl(Excel::V2021),
        Entry::added(Excel::V2021).fallback(Fallback::NameError)
    );
    assert_eq!(le(Excel::V2016), Entry::at_most(Excel::V2016));
    assert_eq!(
        between(Excel::V2007, Excel::V2016),
        Entry::between(Excel::V2007, Excel::V2016)
    );
    assert_eq!(no::<LibreOffice>(), Entry::never());
    assert_eq!(unk::<OnlyOffice>(), Entry::unknown());
    assert_eq!(YES, Entry::added(GoogleSheets::Current));
    let support = s(
        xl(Excel::V2010),
        xl(ExcelMac::V2016),
        unk(),
        unk(),
        unk(),
        unk(),
    )
    .with_excel_mac(no())
    .with_libreoffice(on(LibreOffice::V24_8))
    .with_google_sheets(YES)
    .with_numbers(on(Numbers::V14_4))
    .with_onlyoffice(on(OnlyOffice::V9_3));
    assert_eq!(support.excel, xl(Excel::V2010));
    assert_eq!(support.excel_mac, Entry::never());
    assert_eq!(support.libreoffice, Entry::added(LibreOffice::V24_8));
    assert_eq!(support.google_sheets, YES);
    assert_eq!(support.numbers, Entry::added(Numbers::V14_4));
    assert_eq!(support.onlyoffice, Entry::added(OnlyOffice::V9_3));
}

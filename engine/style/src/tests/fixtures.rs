//! `StylesEdit` over the styles part of every workbook in the repository's
//! `fixtures/` folder (the ones Excel wrote that have one): written back with
//! nothing added, it is the same bytes; with styles added, `xlsx-graft-read`
//! reads the cell formats it expects at the indexes given; and the same
//! styles asked of the part that was written, or again, are at the same
//! indexes and add nothing.
//!
//! `fixtures/` is outside the crate: the test reads the workbooks from disk
//! when it runs.

use super::read_back::kinds;
use crate::{
    Alignment, Border, BorderSide, BorderStyle, Color, DATE, DATE_TIME, ELAPSED, Fill, Font,
    HorizontalAlignment, PatternType, Protection, Style, StyleLimits, StylesEdit, TIME,
};
use std::fs::File;
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_read::FormatKind;
use xlsx_graft_xml::XmlLimits;

/// The repository's `fixtures/` folder.
fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// The name and the styles part of each workbook in `fixtures/` that has one.
fn styles_parts() -> Vec<(String, Vec<u8>)> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(folder())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("xlsx" | "xlsm")
            )
        })
        .collect();
    paths.sort();
    let mut parts = Vec::new();
    for path in paths {
        let mut diagnostics = Diagnostics::new();
        let file = File::open(&path).unwrap();
        let mut package = Package::open(file, &Limits::default(), &mut diagnostics)
            .unwrap()
            .unwrap();
        let at = (package.parts().iter())
            .position(|part| part.name().as_str().ends_with("xl/styles.xml"));
        if let Some(at) = at {
            let limit = XmlLimits::default().max_bytes;
            let bytes = package
                .read_xml_part(at, limit, &mut diagnostics)
                .unwrap()
                .unwrap();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            parts.push((name, bytes));
        }
    }
    parts
}

/// Each style to add, and the kind of format a cell with it reads as, where
/// `None` is the kind the base cell format has.
fn styles() -> Vec<(Style, Option<FormatKind>)> {
    let number = |code: &str, kind| (Style::number_format(code), Some(kind));
    let font = Font {
        bold: Some(true),
        italic: Some(true),
        name: Some("Arial".into()),
        size: Some(14.0),
        color: Some(Color::Rgb([0xFF, 0xC0, 0, 0])),
        ..Font::default()
    };
    let side = BorderSide {
        style: BorderStyle::Thin,
        color: None,
    };
    vec![
        number(DATE, FormatKind::Date),
        number(TIME, FormatKind::Time),
        number(DATE_TIME, FormatKind::DateTime),
        number(ELAPSED, FormatKind::Duration),
        number("0.00", FormatKind::Number),
        number("@", FormatKind::Text),
        (
            Style {
                font: Some(font),
                ..Style::default()
            },
            None,
        ),
        (
            Style {
                fill: Some(Fill::Pattern {
                    pattern: PatternType::Solid,
                    foreground: Some(Color::Theme {
                        index: 4,
                        tint: Some(0.8),
                    }),
                    background: None,
                }),
                ..Style::default()
            },
            None,
        ),
        (
            Style {
                border: Some(Border {
                    bottom: Some(side),
                    top: Some(side),
                    ..Border::default()
                }),
                ..Style::default()
            },
            None,
        ),
        (
            Style {
                alignment: Some(Alignment {
                    horizontal: Some(HorizontalAlignment::Center),
                    wrap_text: Some(true),
                    ..Alignment::default()
                }),
                protection: Some(Protection {
                    locked: Some(false),
                    hidden: None,
                }),
                ..Style::default()
            },
            None,
        ),
    ]
}

/// The part `edit` writes from `original`.
fn rewritten(edit: &StylesEdit, original: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    edit.rewrite(&mut &*original, &mut out).unwrap();
    out
}

/// The cell format of each of `styles` laid over the cell format 0.
fn add_all(edit: &mut StylesEdit, styles: &[(Style, Option<FormatKind>)]) -> Vec<u32> {
    let add = |(style, _): &(Style, Option<FormatKind>)| edit.cell_format(0, style).unwrap();
    styles.iter().map(add).collect()
}

#[test]
fn the_fixtures_with_a_styles_part_are_the_eleven_known() {
    let names: Vec<String> = styles_parts().into_iter().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        [
            "chart.xlsx",
            "date1904.xlsx",
            "dynamic-array.xlsx",
            "external-link.xlsx",
            "external-source.xlsx",
            "formulas.xlsx",
            "macro.xlsm",
            "merged.xlsx",
            "pivot.xlsx",
            "python-in-excel.xlsm",
            "table.xlsx",
        ]
    );
}

#[test]
fn a_part_nothing_was_added_to_is_written_back_byte_for_byte() {
    for (name, original) in styles_parts() {
        let edit = StylesEdit::read(&original, &StyleLimits::default()).unwrap();
        assert_eq!(rewritten(&edit, &original), original, "{name}");
    }
}

#[test]
fn the_cell_formats_added_are_read_back_as_the_kinds_asked_for() {
    for (name, original) in styles_parts() {
        let text = String::from_utf8(original.clone()).unwrap();
        let before = kinds(&text);
        let mut edit = StylesEdit::read(&original, &StyleLimits::default()).unwrap();
        let styles = styles();
        let indexes = add_all(&mut edit, &styles);
        let out = String::from_utf8(rewritten(&edit, &original)).unwrap();
        let after = kinds(&out);
        let top = indexes.iter().max().unwrap();
        let total = before.len().max(usize::try_from(*top).unwrap() + 1);
        assert_eq!(after.len(), total, "{name}");
        assert!(total > before.len(), "{name}");
        assert_eq!(after[..before.len()], before[..], "{name}");
        for ((_, kind), index) in styles.iter().zip(&indexes) {
            let expected = kind.unwrap_or(before[0]);
            assert_eq!(
                after[usize::try_from(*index).unwrap()],
                expected,
                "{name} {index}"
            );
        }
    }
}

#[test]
fn the_same_style_asked_again_is_at_the_same_index() {
    for (name, original) in styles_parts() {
        let mut edit = StylesEdit::read(&original, &StyleLimits::default()).unwrap();
        let styles = styles();
        let first = add_all(&mut edit, &styles);
        let out = rewritten(&edit, &original);
        assert_eq!(add_all(&mut edit, &styles), first, "{name}");
        assert_eq!(rewritten(&edit, &original), out, "{name}");
    }
}

#[test]
fn the_part_that_was_written_is_read_again_to_the_same_indexes_and_adds_nothing() {
    for (name, original) in styles_parts() {
        let mut edit = StylesEdit::read(&original, &StyleLimits::default()).unwrap();
        let styles = styles();
        let first = add_all(&mut edit, &styles);
        let out = rewritten(&edit, &original);
        let mut again = StylesEdit::read(&out, &StyleLimits::default()).unwrap();
        assert_eq!(add_all(&mut again, &styles), first, "{name}");
        assert_eq!(rewritten(&again, &out), out, "{name}");
    }
}

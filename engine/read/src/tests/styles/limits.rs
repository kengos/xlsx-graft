//! The limits on the styles, each at its value and one past it.

use super::super::at;
use super::{PART, cells, numbers, read_with, styled, styled_with};
use crate::{ReadLimits, Styles};
use xlsx_graft_diagnostics::{Diagnostic, Limit};
use xlsx_graft_xml::XmlLimits;

#[test]
fn the_defaults_are_the_documented_values() {
    let limits = ReadLimits::default();
    assert_eq!(limits.max_cell_formats, Limit::default_of(65_536));
    assert_eq!(limits.max_number_formats, Limit::default_of(65_536));
}

#[test]
fn as_many_cell_formats_as_the_limit_read_and_one_more_is_refused() {
    let limits = ReadLimits {
        max_cell_formats: Limit::default_of(3),
        ..ReadLimits::default()
    };
    let (styles, seen) = read_with(styled(&cells(&[0, 1, 2])), &limits);
    assert_eq!(seen, vec![]);
    assert!(matches!(styles, Styles::Formats(ref kinds) if kinds.len() == 3));

    let (styles, seen) = read_with(styled(&cells(&[0, 1, 2, 3, 4])), &limits);
    assert_eq!(styles, Styles::Unknown);
    let limit = limits.max_cell_formats;
    let diagnostic = Diagnostic::CellFormatsTooMany { count: 4, limit };
    assert_eq!(seen, vec![at(diagnostic, PART)]);
}

#[test]
fn as_many_number_formats_as_the_limit_read_and_one_more_is_refused() {
    let limits = ReadLimits {
        max_number_formats: Limit::default_of(2),
        ..ReadLimits::default()
    };
    let two = numbers(&[(164, "0"), (165, "0")]);
    let (styles, seen) = read_with(styled(&two), &limits);
    assert_eq!(seen, vec![]);
    assert_eq!(styles, Styles::Formats(vec![]));

    // A repeated id counts again.
    let three = numbers(&[(164, "0"), (165, "0"), (164, "0"), (166, "0")]);
    let (styles, seen) = read_with(styled(&three), &limits);
    assert_eq!(styles, Styles::Unknown);
    let limit = limits.max_number_formats;
    let diagnostic = Diagnostic::NumberFormatsTooMany { count: 3, limit };
    assert_eq!(seen, vec![at(diagnostic, PART)]);
}

#[test]
fn a_part_as_large_as_the_xml_limit_reads_and_one_byte_more_is_refused() {
    let xml = format!(r#"<styleSheet xmlns="{}"/>"#, super::super::NS);
    let size = xml.len() as u64;
    let limits = |bytes| ReadLimits {
        xml: XmlLimits {
            max_bytes: Limit::default_of(bytes),
            ..XmlLimits::default()
        },
        ..ReadLimits::default()
    };
    let (styles, seen) = read_with(styled_with(super::part(&xml)), &limits(size));
    assert_eq!(seen, vec![]);
    assert_eq!(styles, Styles::Formats(vec![]));

    let limits = limits(size - 1);
    let (styles, seen) = read_with(styled_with(super::part(&xml)), &limits);
    assert_eq!(styles, Styles::Unknown);
    let limit = limits.xml.max_bytes;
    let diagnostic = Diagnostic::XmlTooLarge { size, limit };
    assert_eq!(seen, vec![at(diagnostic, PART)]);
}

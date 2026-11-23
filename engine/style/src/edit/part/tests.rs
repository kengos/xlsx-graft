//! `write_error`: each failure of [`StylesEdit::rewrite`](crate::StylesEdit)
//! and each error `cell_format` returns, as the error of a rewriter.

use super::write_error;
use crate::edit::{StyleError, StyleKind};
use crate::{StyleLimits, StylesEdit};
use std::io;
use xlsx_graft_diagnostics::Diagnostic;
use xlsx_graft_write::WriteError;

/// The diagnostic that reading a part that is not XML gives.
fn diagnostic() -> Diagnostic {
    match StylesEdit::read(b"<styleSheet", &StyleLimits::default()) {
        Err(StyleError::Invalid(diagnostic)) => diagnostic,
        other => panic!("{other:?}"),
    }
}

#[test]
fn style_error_invalid_becomes_write_error_invalid_with_the_same_diagnostic() {
    let expected = diagnostic();
    match write_error(StyleError::Invalid(diagnostic())) {
        WriteError::Invalid(found) => assert_eq!(found, expected),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_io_error_is_an_io_error_of_the_same_kind() {
    let error = io::Error::new(io::ErrorKind::BrokenPipe, "closed");
    match write_error(StyleError::Io(error)) {
        WriteError::Io(error) => assert_eq!(error.kind(), io::ErrorKind::BrokenPipe),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_io_error_that_wraps_a_write_error_becomes_that_write_error() {
    let error = io::Error::other(WriteError::Unusable);
    assert!(matches!(
        write_error(StyleError::Io(error)),
        WriteError::Unusable
    ));
}

#[test]
fn the_errors_cell_format_returns_are_io_errors_that_say_what_they_were() {
    let errors = [
        StyleError::TooMany {
            kind: StyleKind::Font,
            limit: 512,
        },
        StyleError::InvalidValue { field: "sz" },
        StyleError::NoCellFormat { base: 7 },
    ];
    for error in errors {
        let said = error.to_string();
        match write_error(error) {
            WriteError::Io(error) => {
                assert_eq!(error.kind(), io::ErrorKind::Other);
                assert_eq!(error.to_string(), said);
            }
            other => panic!("{other:?}"),
        }
    }
}

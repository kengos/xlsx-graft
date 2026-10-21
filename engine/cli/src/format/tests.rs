//! Unit tests for `emit`: each format's bytes, and the two ways it fails.

use super::{Format, Report, emit};
use crate::{CliError, VersionReport};
use serde::{Serialize, Serializer};
use std::io::{self, Write};

fn emit_to_string<T: Report>(format: Format, value: &T) -> String {
    let mut out = Vec::new();
    emit(format, value, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

/// An output whose JSON form fails to serialize.
struct Unserializable;

impl Serialize for Unserializable {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("refused"))
    }
}

impl Report for Unserializable {
    fn write_text(&self, out: &mut dyn Write) -> io::Result<()> {
        writeln!(out, "unserializable")
    }
}

#[test]
fn text_is_the_reports_own_text_form() {
    let report = VersionReport { version: "1.2.3" };
    assert_eq!(emit_to_string(Format::Text, &report), "xlsx-graft 1.2.3\n");
}

#[test]
fn json_is_the_serialized_report_on_one_line() {
    let report = VersionReport { version: "1.2.3" };
    assert_eq!(
        emit_to_string(Format::Json, &report),
        "{\"version\":\"1.2.3\"}\n"
    );
}

#[test]
fn a_refused_write_is_a_write_error_in_either_format() {
    let report = VersionReport::current();
    for format in [Format::Text, Format::Json] {
        // A zero-length buffer takes no bytes, like a full disk.
        let mut full: &mut [u8] = &mut [];
        let err = emit(format, &report, &mut full).unwrap_err();
        assert!(matches!(err, CliError::Write(e) if e.kind() == io::ErrorKind::WriteZero));
    }
}

#[test]
fn a_value_that_will_not_serialize_is_a_serialize_error_and_writes_nothing() {
    let mut out = Vec::new();
    let err = emit(Format::Json, &Unserializable, &mut out).unwrap_err();
    assert!(matches!(err, CliError::Serialize(_)));
    assert!(out.is_empty());
    // Its text form still prints: only JSON goes through serde.
    assert_eq!(
        emit_to_string(Format::Text, &Unserializable),
        "unserializable\n"
    );
}

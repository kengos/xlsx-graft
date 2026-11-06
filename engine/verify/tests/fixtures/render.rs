//! The results of reading and verifying a workbook, as the lines
//! `expected.rs` lists.

use std::io::Cursor;
use xlsx_graft_diagnostics::{Diagnostics, Record};
use xlsx_graft_package::Limits as PackageLimits;
use xlsx_graft_read::{Book, ReadLimits, Row, Value};
use xlsx_graft_spool::TempFiles;
use xlsx_graft_verify::{DefaultFunctions, Limits, Rules, Targets, verify as run_verify};

/// A diagnostic as one line: its code, then each field of its location
/// it has (`part=`, `rel=`, `place=`, `name=`), in that order.
fn line(record: &Record) -> String {
    let location = &record.location;
    let mut out = String::from(record.diagnostic.code().as_str());
    let fields = [
        ("part", &location.part),
        ("rel", &location.relationship_id),
        ("place", &location.place),
        ("name", &location.defined_name),
    ];
    for (key, value) in fields {
        if let Some(value) = value {
            out += &format!(" {key}={}", value.as_str());
        }
    }
    out
}

/// What `verify` pushes for the workbook in `bytes`, with the engine's
/// default limits, rules, function table and targets, a line each in the
/// order pushed.
pub(crate) fn verify(bytes: &[u8]) -> Vec<String> {
    let mut diagnostics = Diagnostics::new();
    run_verify(
        Cursor::new(bytes),
        &Limits::default(),
        &Rules::default(),
        &DefaultFunctions,
        &Targets::default(),
        &mut diagnostics,
    )
    .unwrap();
    diagnostics.records().iter().map(line).collect()
}

/// A row as one line: the sheet's name and the row's number
/// (`Sheet1!2:`), then each cell's value as [`cell`] writes it, separated
/// by ` | `.
fn row_line(sheet: &str, row: &Row) -> String {
    let cells: Vec<String> = row.values().map(|value| cell(&value)).collect();
    format!("{sheet}!{}: {}", row.number(), cells.join(" | "))
}

/// A cell's value as [`row_line`] writes it: nothing for an empty cell, a
/// string in quotes, a number as Rust prints an `f64`, a boolean as
/// `true` or `false`, a date, a time or a date and time in ISO 8601
/// (`2024-01-15`), and an error value or elapsed time after its kind
/// (`error #REF!`).
fn cell(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::String(text) => format!("{text:?}"),
        Value::Number(number) => format!("{number}"),
        Value::Bool(flag) => format!("{flag}"),
        Value::Date(date) => format!("{date}"),
        Value::Time(time) => format!("{time}"),
        Value::DateTime(date_time) => format!("{date_time}"),
        Value::Elapsed(elapsed) => format!("elapsed {elapsed:?}"),
        Value::Error(code) => format!("error {code}"),
    }
}

/// What reading the workbook in `bytes` gives, with the default limits:
/// its date system, then each sheet's name and kind followed by the rows
/// of a worksheet, then the diagnostics opening and reading pushed.
pub(crate) fn read(bytes: &[u8]) -> Vec<String> {
    let mut diagnostics = Diagnostics::new();
    let book = Book::open(
        Cursor::new(bytes),
        &PackageLimits::default(),
        &ReadLimits::default(),
        &mut TempFiles::new(),
        &mut diagnostics,
    )
    .unwrap();
    let mut lines = Vec::new();
    if let Some(book) = book {
        lines.push(format!("date system {:?}", book.date_system()));
        for (index, sheet) in book.sheets().iter().enumerate() {
            let kind = sheet.part().map(|part| format!("{:?}", part.kind()));
            lines.push(format!(
                "sheet {:?} {}",
                sheet.name(),
                kind.unwrap_or_default()
            ));
            let Some(mut rows) = book.rows(index) else {
                continue;
            };
            let mut row = Row::new();
            while rows.next_row(&mut row, &mut diagnostics).unwrap() {
                lines.push(row_line(sheet.name(), &row));
            }
        }
    }
    lines.extend(
        diagnostics
            .records()
            .iter()
            .map(|r| format!("diagnostic {}", line(r))),
    );
    lines
}

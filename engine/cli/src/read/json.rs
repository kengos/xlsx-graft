//! `read --format json`: JSON Lines, a row a line and the end last, in the
//! shape `help.rs` gives as the contract.
//!
//! A number is serde_json's: the shortest text that reads back as the same
//! `f64`, with a fraction or an exponent always, so a reader that tells
//! integers from floats (PHP's `json_decode`) reads a float.

use super::{End, Sink};
use crate::CliError;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::io::Write;
use xlsx_graft_read::{Row, Value};

/// Prints each line to `out`, through a buffer of its own it reuses.
pub(super) struct Json<'a> {
    out: &'a mut dyn Write,
    line: Vec<u8>,
}

impl<'a> Json<'a> {
    pub(super) fn new(out: &'a mut dyn Write) -> Self {
        Self {
            out,
            line: Vec::new(),
        }
    }

    /// Prints `value` as one line.
    fn line<T: Serialize>(&mut self, value: &T) -> Result<(), CliError> {
        self.line.clear();
        serde_json::to_writer(&mut self.line, value)?;
        self.line.push(b'\n');
        self.out.write_all(&self.line).map_err(CliError::Write)
    }
}

impl Sink for Json<'_> {
    fn row(&mut self, row: &Row) -> Result<(), CliError> {
        self.line(&RowLine(row))
    }

    fn end(&mut self, end: &End) -> Result<(), CliError> {
        self.line(end)
    }
}

/// `{"row": <number>, "cells": [<cell>, ...]}`.
struct RowLine<'r>(&'r Row);

impl Serialize for RowLine<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("row", &self.0.number())?;
        map.serialize_entry("cells", &Cells(self.0))?;
        map.end()
    }
}

/// A row's values, A first.
struct Cells<'r>(&'r Row);

impl Serialize for Cells<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.values().map(Cell))
    }
}

/// `{"type": <type>, "value": <value>}`.
pub(super) struct Cell<'v>(pub(super) Value<'v>);

impl Serialize for Cell<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        match self.0 {
            Value::Empty => typed(&mut map, "empty", &())?,
            Value::String(text) => typed(&mut map, "string", text)?,
            Value::Number(number) => typed(&mut map, "number", &number)?,
            Value::Bool(bool) => typed(&mut map, "boolean", &bool)?,
            Value::Date(date) => typed(&mut map, "date", &format_args!("{date}"))?,
            Value::Time(time) => typed(&mut map, "time", &format_args!("{time}"))?,
            Value::DateTime(at) => typed(&mut map, "datetime", &format_args!("{at}"))?,
            Value::Elapsed(elapsed) => typed(&mut map, "elapsed", &elapsed.milliseconds())?,
            Value::Error(text) => typed(&mut map, "error", text)?,
        }
        map.end()
    }
}

/// Writes `type` and `value` to `map`.
fn typed<M: SerializeMap, V: Serialize + ?Sized>(
    map: &mut M,
    kind: &str,
    value: &V,
) -> Result<(), M::Error> {
    map.serialize_entry("type", kind)?;
    map.serialize_entry("value", value)
}

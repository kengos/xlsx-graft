//! [`write_cell`]: a cell's value written as its `<c>` element; the `<t>`
//! text and the `<v>` number it holds are written by the modules of
//! `encode/`.
//!
//! The `<c>` element is ECMA-376-1:2016 §18.3.1.4. A value that is refused
//! is refused before anything is written, so `out` gets no byte of it.

mod number;
mod shared;
mod text;

pub use shared::SharedStringsWriter;

use crate::{CellError, CellRef, CellValue, DateSystem};
use number::{integer, number};
use std::borrow::Cow;
use std::io::Write;
use text::{truncate, write_t};

/// Whether the `<t>` written holds the whole of the string given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "a cut text is written, but not all of it"]
pub enum Written {
    /// The string was written whole.
    Whole,
    /// The string was longer than 32,767 UTF-16 code units, the most Excel
    /// for Mac keeps in a string, and was cut at a character boundary.
    Truncated,
}

/// What goes between the start tag of a `<c>` and its end.
enum Body<'a> {
    /// Nothing: the element is empty.
    Nothing,
    /// A `<v>` holding the text.
    Value(Cow<'a, str>),
    /// An `<is>` holding the text as a `<t>`.
    Inline(&'a str),
}

/// Writes `value` as the cell `at`, with the cell format `style` (the
/// index into `cellXfs`; none and 0, the default, write no `s`).
///
/// The `t` attribute is written only for text (`inlineStr`), a boolean
/// (`b`), an error value (`e`) and a shared string (`s`): a number and a
/// date or time have none, and `t="d"` and `t="str"` are never written. A
/// string that could be read as a formula, a number or a date (`=A1`,
/// `00123`, `2024-01-01`) is also written as an inline string. `system` is the date
/// system the workbook counts a date, time or elapsed time in.
///
/// A string longer than 32,767 UTF-16 code units is cut, which the returned
/// [`Written::Truncated`] says.
///
/// # Errors
///
/// [`CellError::NotFinite`] and [`CellError::InexactInteger`] for a number
/// or an integer a cell cannot hold, and [`CellError::DateOutOfRange`] for a
/// date, date-time or elapsed time `system` has no serial date-time for; each
/// is returned before anything is written to `out`. [`CellError::Io`] when
/// writing to `out` fails.
pub fn write_cell(
    out: &mut impl Write,
    at: CellRef,
    style: Option<u32>,
    value: &CellValue<'_>,
    system: DateSystem,
) -> Result<Written, CellError> {
    let (t, body, written) = lexical(value, system)?;
    write!(out, r#"<c r="{at}""#)?;
    if let Some(style) = style.filter(|&style| style != 0) {
        write!(out, r#" s="{style}""#)?;
    }
    if let Some(t) = t {
        write!(out, r#" t="{t}""#)?;
    }
    match body {
        Body::Nothing => out.write_all(b"/>")?,
        Body::Value(v) => write!(out, "><v>{v}</v></c>")?,
        Body::Inline(text) => {
            out.write_all(b"><is>")?;
            write_t(out, text)?;
            out.write_all(b"</is></c>")?;
        }
    }
    Ok(written)
}

/// The `t` of a cell, what goes in the element, and whether the string was
/// cut.
type Lexical<'a> = (Option<&'static str>, Body<'a>, Written);

/// The `t` of `value`, what goes in the element, and whether the string was
/// cut.
fn lexical<'a>(value: &CellValue<'a>, system: DateSystem) -> Result<Lexical<'a>, CellError> {
    Ok(match *value {
        CellValue::Empty => (None, Body::Nothing, Written::Whole),
        CellValue::String(text) => {
            let (text, written) = truncate(text);
            (Some("inlineStr"), Body::Inline(text), written)
        }
        CellValue::Number(x) => numeric(number(x)?),
        CellValue::Integer(n) => numeric(integer(n)?),
        CellValue::Bool(b) => (
            Some("b"),
            borrowed(if b { "1" } else { "0" }),
            Written::Whole,
        ),
        CellValue::Error(e) => (Some("e"), borrowed(e.text()), Written::Whole),
        CellValue::Date(d) => numeric(number(d.serial(system).ok_or(CellError::DateOutOfRange)?)?),
        CellValue::Time(t) => numeric(number(t.serial())?),
        CellValue::DateTime(d) => {
            numeric(number(d.serial(system).ok_or(CellError::DateOutOfRange)?)?)
        }
        CellValue::Elapsed(e) => {
            numeric(number(e.serial(system).ok_or(CellError::DateOutOfRange)?)?)
        }
        CellValue::SharedString(i) => (
            Some("s"),
            Body::Value(Cow::Owned(i.to_string())),
            Written::Whole,
        ),
    })
}

/// A number's `<v>`, with no `t`.
fn numeric<'a>(text: String) -> Lexical<'a> {
    (None, Body::Value(Cow::Owned(text)), Written::Whole)
}

/// A `<v>` of a fixed text.
fn borrowed(text: &'static str) -> Body<'static> {
    Body::Value(Cow::Borrowed(text))
}

#[cfg(test)]
mod tests;

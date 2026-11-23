//! [`SharedStringsWriter`]: the shared strings part written one string at a
//! time.

use super::Written;
use super::text::{truncate, write_t};
use crate::CellError;
use std::io::Write;

/// The main namespace of SpreadsheetML.
const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// Writes the shared strings part (`sst`, ECMA-376-1:2016 §18.4.9), one
/// `<si>` for each string pushed, in the order pushed, with no
/// `count` or `uniqueCount`.
///
/// A string is numbered from 0 as it is pushed, and the number is what a
/// [`CellValue::SharedString`](crate::CellValue::SharedString) holds. A
/// string pushed twice has two numbers: pushing does not look for
/// duplicates, so a caller that wants one `<si>` for equal strings keeps
/// the number `push` returned for each string and uses it again from the
/// second time. From the string numbered `u64::MAX` on, every string is given
/// `u64::MAX`, so different strings share a number.
///
/// # Examples
///
/// ```
/// use xlsx_graft_cell::{SharedStringsWriter, Written};
///
/// let mut strings = SharedStringsWriter::new(Vec::new()).unwrap();
/// assert_eq!(strings.push("a").unwrap(), (0, Written::Whole));
/// assert_eq!(strings.push("b").unwrap(), (1, Written::Whole));
/// let part = String::from_utf8(strings.finish().unwrap()).unwrap();
/// assert!(part.ends_with("<si><t>a</t></si><si><t>b</t></si></sst>"));
/// ```
#[derive(Debug)]
pub struct SharedStringsWriter<W: Write> {
    /// Where the part is written.
    out: W,
    /// The number the next string gets.
    next: u64,
}

impl<W: Write> SharedStringsWriter<W> {
    /// Starts the part in `out`: the XML declaration and the `sst` start
    /// tag.
    ///
    /// # Errors
    ///
    /// [`CellError::Io`] when writing to `out` fails.
    pub fn new(mut out: W) -> Result<Self, CellError> {
        writeln!(
            out,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
        )?;
        write!(out, r#"<sst xmlns="{NS}">"#)?;
        Ok(Self { out, next: 0 })
    }

    /// Adds `text` as the next string, cut to 32,767 UTF-16 code units
    /// ([`Written::Truncated`] when it was), and returns its number.
    ///
    /// # Errors
    ///
    /// [`CellError::Io`] when writing to `out` fails.
    pub fn push(&mut self, text: &str) -> Result<(u64, Written), CellError> {
        let (text, written) = truncate(text);
        self.out.write_all(b"<si>")?;
        write_t(&mut self.out, text)?;
        self.out.write_all(b"</si>")?;
        let number = self.next;
        self.next = number.saturating_add(1);
        Ok((number, written))
    }

    /// Ends the part and returns the writer.
    ///
    /// # Errors
    ///
    /// [`CellError::Io`] when writing to `out` fails.
    pub fn finish(mut self) -> Result<W, CellError> {
        self.out.write_all(b"</sst>")?;
        Ok(self.out)
    }
}

#[cfg(test)]
mod tests;

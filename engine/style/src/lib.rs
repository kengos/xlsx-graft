//! xlsx-graft style: the writing side of a workbook's styles. This crate
//! holds the values a caller states to format a cell ([`Style`] and the
//! types in it) and [`StylesEdit`], which reads an existing styles part
//! (`xl/styles.xml`, ECMA-376-1:2016 §18.8), lays a [`Style`] over a cell
//! format it has, adds the cell format, font, fill, border and number format
//! that result unless the part has them already, and writes the part again.
//! Below them, a private canonical form holds one item of the part (a
//! `font`, `fill`, `border` or `xf`), so that two items with the same effect
//! are found equal.
//!
//! # Adding a cell format
//!
//! [`StylesEdit::read`] reads the part, [`StylesEdit::cell_format`] gives
//! the index a cell's `s` has (the same index for the same style, so the
//! part does not grow with every call), and [`StylesEdit::rewrite`] writes
//! the part with what was added at the end of its lists. What the part
//! already has is not moved or removed, and a part that nothing was added to
//! is written back byte for byte.
//!
//! ```
//! use xlsx_graft_style::{Font, Style, StyleLimits, StylesEdit};
//!
//! let original: &[u8] = br#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border/></borders><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellXfs></styleSheet>"#;
//! let mut edit = StylesEdit::read(original, &StyleLimits::default())?;
//!
//! let bold = Style {
//!     font: Some(Font {
//!         bold: Some(true),
//!         ..Default::default()
//!     }),
//!     ..Default::default()
//! };
//! // Laid over the cell format 0, the one a cell without an `s` has.
//! let index = edit.cell_format(0, &bold)?;
//! assert_eq!(index, 1);
//! assert_eq!(edit.cell_format(0, &bold)?, index);
//!
//! let mut out = Vec::new();
//! let mut original = original;
//! edit.rewrite(&mut original, &mut out)?;
//! let out = String::from_utf8(out)?;
//! assert!(out.contains(r#"<fonts count="2">"#));
//! assert!(out.contains(r#"<cellXfs count="2">"#));
//! assert!(out.contains(r#"<xf numFmtId="0" fontId="1" fillId="0" borderId="0" applyFont="1"/>"#));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # What a [`Style`] promises
//!
//! A [`Style`] is laid over a base cell format, the one a cell has already,
//! and states only what differs from it. In every `Option` field, `None`
//! keeps the base cell format's value and `Some` overrides it.
//! `Some(Underline::None)`, `Some(VerticalAlign::Baseline)`, `Some(false)` of
//! a font's bold, italic or strike, and
//! `Some(BorderSide { style: BorderStyle::None, .. })` remove that setting
//! from the base. `Some(false)` of an attribute the schema gives no default
//! (`locked`, `hidden`, `wrapText`, `shrinkToFit`, `diagonalUp`,
//! `diagonalDown`) writes it as `0`, such as `locked="0"`: the cell is not
//! locked.
//!
//! A [`Style`] is checked when it is laid over a cell format, not when it is
//! built: [`StylesEdit::cell_format`] refuses a text rotation of 181 to 254,
//! a tint outside -1 to 1, a font size that is not a number from 1 to 409, an
//! empty name or number format code, and one with a character XML cannot
//! hold ([`StyleError::InvalidValue`]), since the XML writer would write a
//! styles part Excel may refuse.
//!
//! # Number formats
//!
//! [`DATE`], [`TIME`], [`DATE_TIME`] and [`ELAPSED`] are format codes that
//! `xlsx-graft-read` reads back as a date, a time, a date and time and an
//! elapsed time. [`Style::number_format`] states one.
//!
//! ```
//! use xlsx_graft_style::{Style, DATE};
//!
//! let style = Style::number_format(DATE);
//! assert_eq!(style.number_format.as_deref(), Some("yyyy-mm-dd"));
//! ```
//!
//! # Building a style
//!
//! The structs ([`Style`], [`Font`], [`Border`], [`BorderSide`],
//! [`Alignment`], [`Protection`]) are built with a struct literal and
//! `..Default::default()`, so a field added later does not break the code.
//! The enumerations and [`Fill`] and [`Color`] are `#[non_exhaustive]`: a
//! `match` on them needs a wildcard arm.
//!
//! ```
//! use xlsx_graft_style::{
//!     Alignment, Border, BorderSide, BorderStyle, Color, Fill, Font, HorizontalAlignment,
//!     PatternType, Protection, Style, Underline,
//! };
//!
//! let style = Style {
//!     font: Some(Font {
//!         bold: Some(true),
//!         underline: Some(Underline::Single),
//!         color: Some(Color::Rgb([0xFF, 0xC0, 0x00, 0x00])),
//!         ..Default::default()
//!     }),
//!     fill: Some(Fill::Pattern {
//!         pattern: PatternType::Solid,
//!         foreground: Some(Color::Theme { index: 4, tint: Some(0.8) }),
//!         background: None,
//!     }),
//!     border: Some(Border {
//!         bottom: Some(BorderSide {
//!             style: BorderStyle::Thin,
//!             color: None,
//!         }),
//!         ..Default::default()
//!     }),
//!     alignment: Some(Alignment {
//!         horizontal: Some(HorizontalAlignment::Center),
//!         ..Default::default()
//!     }),
//!     protection: Some(Protection {
//!         locked: Some(false),
//!         ..Default::default()
//!     }),
//!     ..Default::default()
//! };
//! assert_eq!(style.number_format, None);
//! assert_eq!(style.font.and_then(|font| font.bold), Some(true));
//! ```

mod number_format;
mod value;

mod canonical;
mod edit;

pub use edit::{StyleError, StyleKind, StyleLimits, StylesEdit};
pub use number_format::{DATE, DATE_TIME, ELAPSED, TIME};
pub use value::{
    Alignment, Border, BorderSide, BorderStyle, Color, Fill, Font, HorizontalAlignment,
    PatternType, Protection, Style, Underline, VerticalAlign, VerticalAlignment,
};

#[cfg(test)]
mod tests;

//! xlsx-graft style: the writing side of a workbook's styles. This crate
//! holds the values a caller states to format a cell ([`Style`] and the
//! types in it) and, below them, how the styles part (`xl/styles.xml`,
//! ECMA-376-1:2016 §18.8) holds one of its items (a `font`, `fill`,
//! `border` or `xf`) in a canonical form, overlays a [`Style`] on it and
//! writes it as XML. The canonical form, the overlay and the writer are
//! private to the crate for now: the styles editor that uses them is in the
//! next pull request.
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
//! A [`Style`] is not checked here. The XML writer assumes valid values:
//! a text rotation of 181 to 254, a tint outside -1 to 1, a font size that is
//! not a finite positive number or a name with a character XML cannot hold
//! make it write a styles part Excel may refuse. The next pull request's
//! cell format function checks them before it uses the writer.
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

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "StylesEdit uses it in the next pull request")
)]
mod canonical;

pub use number_format::{DATE, DATE_TIME, ELAPSED, TIME};
pub use value::{
    Alignment, Border, BorderSide, BorderStyle, Color, Fill, Font, HorizontalAlignment,
    PatternType, Protection, Style, Underline, VerticalAlign, VerticalAlignment,
};

#[cfg(test)]
mod tests;

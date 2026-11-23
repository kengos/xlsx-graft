//! The cell formats `styles` adds: [`entries`], one [`Entry`] for each row
//! of column A, in order, with the value the row's cell holds.

use xlsx_graft_cell::CellValue;
use xlsx_graft_style::{
    Alignment, Border, BorderSide, BorderStyle, Color, DATE, DATE_TIME, ELAPSED, Fill, Font,
    HorizontalAlignment, PatternType, Protection, Style, TIME, Underline, VerticalAlign,
    VerticalAlignment,
};

/// The value of a row whose style is a number format: 2024-01-01 18:00 as a
/// serial date-time, which each date, time and elapsed time format shows.
const SAMPLE_NUMBER: f64 = 45_292.75;

/// A style to add, and the value of the cell it is given to.
pub(super) struct Entry {
    /// The style, laid over the cell format 0.
    pub(super) style: Style,
    /// The value of the cell: [`SAMPLE_NUMBER`] when the style is a number
    /// format, otherwise a string that names the style.
    pub(super) value: CellValue<'static>,
}

/// A row of `style` whose cell holds the string `name`.
fn named(name: &'static str, style: Style) -> Entry {
    Entry {
        style,
        value: CellValue::String(name),
    }
}

/// A row of the number format `code`.
fn number(code: &str) -> Entry {
    Entry {
        style: Style::number_format(code),
        value: CellValue::Number(SAMPLE_NUMBER),
    }
}

/// A style of a font alone.
fn font(font: Font) -> Style {
    Style {
        font: Some(font),
        ..Style::default()
    }
}

/// A border side of `style` in the ARGB colour `color`, or in the
/// application's colour when it is `None`.
fn side(style: BorderStyle, color: Option<[u8; 4]>) -> Option<BorderSide> {
    Some(BorderSide {
        style,
        color: color.map(Color::Rgb),
    })
}

/// A style of a border alone.
fn border(border: Border) -> Style {
    Style {
        border: Some(border),
        ..Style::default()
    }
}

/// A style of an alignment alone.
fn alignment(alignment: Alignment) -> Style {
    Style {
        alignment: Some(alignment),
        ..Style::default()
    }
}

/// A style of a pattern fill alone.
fn fill(pattern: PatternType, color: [u8; 4]) -> Style {
    Style {
        fill: Some(Fill::Pattern {
            pattern,
            foreground: Some(Color::Rgb(color)),
            background: None,
        }),
        ..Style::default()
    }
}

/// The 18 rows, for A1 on: the fonts, the fills, the borders, the number
/// formats, the alignments and the protection that `StylesEdit` writes.
pub(super) fn entries() -> Vec<Entry> {
    const RED: [u8; 4] = [0xFF, 0xC0, 0x00, 0x00];
    const BLUE: [u8; 4] = [0xFF, 0x00, 0x70, 0xC0];
    let thin = side(BorderStyle::Thin, None);
    vec![
        named(
            "Bold",
            font(Font {
                bold: Some(true),
                ..Font::default()
            }),
        ),
        named(
            "Italic underline red",
            font(Font {
                italic: Some(true),
                underline: Some(Underline::Single),
                color: Some(Color::Rgb(RED)),
                ..Font::default()
            }),
        ),
        named(
            "Font name and size",
            font(Font {
                name: Some("Arial".to_owned()),
                size: Some(14.0),
                ..Font::default()
            }),
        ),
        named(
            "Strike superscript",
            font(Font {
                strike: Some(true),
                vertical_align: Some(VerticalAlign::Superscript),
                ..Font::default()
            }),
        ),
        named(
            "Solid yellow fill",
            fill(PatternType::Solid, [0xFF, 0xFF, 0xFF, 0x00]),
        ),
        named("Gray125 pattern", fill(PatternType::Gray125, BLUE)),
        named(
            "Thin border",
            border(Border {
                left: thin,
                right: thin,
                top: thin,
                bottom: thin,
                ..Border::default()
            }),
        ),
        named(
            "Thick bottom double top",
            border(Border {
                bottom: side(BorderStyle::Thick, Some(RED)),
                top: side(BorderStyle::Double, Some(BLUE)),
                ..Border::default()
            }),
        ),
        named(
            "Diagonal",
            border(Border {
                diagonal: thin,
                diagonal_up: Some(true),
                diagonal_down: Some(true),
                ..Border::default()
            }),
        ),
        number(DATE),
        number(TIME),
        number(DATE_TIME),
        number(ELAPSED),
        number("#,##0.00"),
        number("0.0%"),
        named(
            "Centre wrap",
            alignment(Alignment {
                horizontal: Some(HorizontalAlignment::Center),
                wrap_text: Some(true),
                ..Alignment::default()
            }),
        ),
        named(
            "Rotation top indent",
            alignment(Alignment {
                vertical: Some(VerticalAlignment::Top),
                indent: Some(2),
                text_rotation: Some(45),
                ..Alignment::default()
            }),
        ),
        named(
            "Protection",
            Style {
                protection: Some(Protection {
                    locked: Some(false),
                    hidden: Some(true),
                }),
                ..Style::default()
            },
        ),
    ]
}

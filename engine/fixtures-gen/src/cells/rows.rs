//! The rows of the worksheet `cells` writes: a description in column A and
//! a value in column B, as [`Row`]s of owned [`Sample`]s, so that the tests
//! can read what was written back against the same samples.

mod clock;

use xlsx_graft_cell::{
    CellValue, Date, DateSystem, DateTime, Elapsed, ErrorValue, Time, ValueError,
};

/// The strings of the shared strings part, in the order they are pushed:
/// the string numbered `i` is what the row `Shared(i)` reads back as.
pub(super) const SHARED: [&str; 5] = [
    "shared plain",
    "shared _x0041_",
    "shared \u{1}",
    " shared preserve ",
    "shared a\rb",
];

/// A value written in column B, and the cell format it is written with.
pub(super) enum Sample {
    /// A string, written inline.
    Text(String),
    /// A number.
    Number(f64),
    /// An integer.
    Integer(i64),
    /// A boolean.
    Bool(bool),
    /// An error value.
    Error(ErrorValue),
    /// A date, under the cell format `yyyy-mm-dd`.
    Date(Date),
    /// A time of day, under the cell format `hh:mm:ss.000`.
    Time(Time),
    /// A date and a time of day, under the cell format
    /// `yyyy-mm-dd hh:mm:ss.000`.
    DateTime(DateTime),
    /// Elapsed time, under the cell format `[h]:mm:ss.000`.
    Elapsed(Elapsed),
    /// The string at this index of [`SHARED`].
    Shared(u64),
    /// A cell with no value and no cell format.
    Empty,
}

impl Sample {
    /// The value `write_cell` writes.
    pub(super) fn value(&self) -> CellValue<'_> {
        match self {
            Self::Text(text) => CellValue::String(text),
            Self::Number(number) => CellValue::Number(*number),
            Self::Integer(integer) => CellValue::Integer(*integer),
            Self::Bool(flag) => CellValue::Bool(*flag),
            Self::Error(error) => CellValue::Error(*error),
            Self::Date(date) => CellValue::Date(*date),
            Self::Time(time) => CellValue::Time(*time),
            Self::DateTime(date_time) => CellValue::DateTime(*date_time),
            Self::Elapsed(elapsed) => CellValue::Elapsed(*elapsed),
            Self::Shared(index) => CellValue::SharedString(*index),
            Self::Empty => CellValue::Empty,
        }
    }

    /// The index into `cellXfs` of `styles.xml` the cell is written with:
    /// 1 to 4 for a date, a time, a date-time and an elapsed time, none for
    /// the rest.
    pub(super) fn style(&self) -> Option<u32> {
        match self {
            Self::Date(_) => Some(1),
            Self::Time(_) => Some(2),
            Self::DateTime(_) => Some(3),
            Self::Elapsed(_) => Some(4),
            _ => None,
        }
    }
}

/// A row: the text of column A and the sample of column B.
pub(super) struct Row {
    /// What the sample is, written in column A.
    pub(super) label: String,
    /// The value written in column B.
    pub(super) sample: Sample,
}

impl Row {
    fn new(label: impl Into<String>, sample: Sample) -> Self {
        Self {
            label: label.into(),
            sample,
        }
    }
}

/// Every row of the worksheet of a workbook in `system`, in order.
///
/// # Errors
///
/// A text that `Date::from_iso`, `Time::from_iso` or `DateTime::from_iso`
/// refuses, or a serial date-time that `Elapsed::from_serial` returns `None`
/// for.
pub(super) fn rows(system: DateSystem) -> Result<Vec<Row>, ValueError> {
    let mut rows = strings();
    rows.extend(numbers());
    rows.extend(clock::rows(system)?);
    rows.extend((0..).zip(SHARED).map(|(index, text)| {
        Row::new(
            format!("shared string {index} {text:?}"),
            Sample::Shared(index),
        )
    }));
    Ok(rows)
}

/// Strings that are, or could be mistaken for, something other than text.
fn strings() -> Vec<Row> {
    let short = [
        "plain",
        "",
        "\u{1}\u{8}\u{1F}",
        "\u{FFFE}",
        "_x0041_",
        "_xABCD\u{1}",
        "a\rb",
        "a\r\nb",
        "line1\nline2",
        " leading",
        "trailing ",
        "\ttab",
        "=1+2",
        "=SUM(A1:A2)",
        "00123",
        "2024-01-01",
        "TRUE",
        "#N/A",
        "<&>\"'",
        "日本語",
        "😀",
    ];
    let mut rows: Vec<Row> = (short.iter())
        .map(|text| Row::new(format!("string {text:?}"), Sample::Text((*text).to_owned())))
        .collect();
    let long = [
        (
            "string of 40,000 x, cut to 32,767 UTF-16 code units",
            "x".repeat(40_000),
        ),
        (
            "string of a and 16,384 emoji, 32,769 code units, cut to 32,767",
            format!("a{}", "😀".repeat(16_384)),
        ),
        (
            "string of 16,384 emoji, 32,768 code units, cut to 32,766 so that a surrogate pair is not split",
            "😀".repeat(16_384),
        ),
    ];
    rows.extend(long.map(|(label, text)| Row::new(label, Sample::Text(text))));
    rows
}

/// Numbers, integers, booleans, error values and an empty cell, among them
/// the numbers and integers `write_cell` refuses.
fn numbers() -> Vec<Row> {
    let numbers = [
        0.1,
        -0.0,
        f64::MAX,
        5E-324,
        1E-7,
        1.5E-7,
        1E+21,
        123_456_789_012_345_680_000.0,
        f64::NAN,
        f64::INFINITY,
    ];
    let integers = [0, -1, 1 << 53, -(1 << 53), (1 << 53) + 1];
    let errors = [
        ErrorValue::Null,
        ErrorValue::DivZero,
        ErrorValue::Value,
        ErrorValue::Ref,
        ErrorValue::Name,
        ErrorValue::Num,
        ErrorValue::NotAvailable,
        ErrorValue::GettingData,
    ];
    let mut rows: Vec<Row> = (numbers.iter())
        .map(|n| Row::new(format!("number {n:?}"), Sample::Number(*n)))
        .collect();
    rows.extend(integers.map(|n| Row::new(format!("integer {n}"), Sample::Integer(n))));
    rows.extend([true, false].map(|b| Row::new(format!("boolean {b}"), Sample::Bool(b))));
    rows.extend(errors.map(|e| Row::new(format!("error {e:?}"), Sample::Error(e))));
    rows.push(Row::new("empty cell", Sample::Empty));
    rows
}

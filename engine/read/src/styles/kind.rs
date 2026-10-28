//! [`FormatKind`]: what a number format makes of a cell's number, and the
//! kinds of the built-in formats (ECMA-376-1:2016 §18.8.30), whose format
//! code a workbook does not write.

/// What a cell's number format makes of the number it holds: the only part
/// of a format the reader keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    /// The General format: the number as it is.
    General,
    /// A number format: digits, a percentage, a fraction, scientific.
    Number,
    /// The text format (`@`): the number shown as text.
    Text,
    /// A date: years, months or days, and no time of day.
    Date,
    /// A time of day: hours, minutes or seconds, and no date.
    Time,
    /// A date and a time of day.
    DateTime,
    /// Elapsed time (`[h]`, `[mm]`, `[ss]`), which does not wrap at a day.
    Duration,
    /// What the format makes of a number is not known: the format is not
    /// defined, cannot be read, or its sections disagree. A number under it
    /// is best taken as it is.
    Unknown,
}

/// The kind of the built-in format `id`; `None` for an id the standard does
/// not list, which takes its format from the workbook alone.
///
/// The ids 27–36 and 50–58 differ by the language of the application. Each
/// takes its meaning in Japanese and Korean, where the two agree: a date
/// but for 32 and 33, a time of day in every language. Chinese makes 34,
/// 35, 52, 53, 55 and 56 a time of day in one of its two tables or both.
/// The ids 59–62 and 67–81 are Thai only.
#[must_use]
pub(super) fn builtin(id: u32) -> Option<FormatKind> {
    use FormatKind::{Date, DateTime, Duration, General, Number, Text, Time};
    let kind = match id {
        0 => General,
        // 0, 0.00, #,##0, #,##0.00, 0%, 0.00%, 0.00E+00, # ?/?, # ??/??.
        1..=4 | 9..=13 => Number,
        // mm-dd-yy (Excel's m/d/yyyy), d-mmm-yy, d-mmm, mmm-yy.
        14..=17 => Date,
        // h:mm AM/PM, h:mm:ss AM/PM, h:mm, h:mm:ss.
        18..=21 => Time,
        // m/d/yy h:mm.
        22 => DateTime,
        // Japanese: [$-411]ge.m.d, [$-411]ggge"年"m"月"d"日" (28 and 29),
        // m/d/yy, yyyy"年"m"月"d"日".
        27..=31 => Date,
        // h"時"mm"分", h"時"mm"分"ss"秒".
        32 | 33 => Time,
        // yyyy"年"m"月", m"月"d"日", [$-411]ge.m.d; then 50–58 are the
        // formats of 27, 28, 34, 35, 28, 34, 35, 27 and 28.
        34..=36 | 50..=58 => Date,
        // #,##0 ;(#,##0) and the like, with red and with decimals.
        37..=40 => Number,
        // mm:ss.
        45 => Time,
        // [h]:mm:ss.
        46 => Duration,
        // mmss.0 (Excel's mm:ss.0).
        47 => Time,
        // ##0.0E+0.
        48 => Number,
        49 => Text,
        // Thai: t0, t0.00, t#,##0, t#,##0.00, then percentages and
        // fractions.
        59..=62 | 67..=70 => Number,
        // Thai dates: day, month and year.
        71..=74 | 81 => Date,
        // Thai times: hours and minutes, with seconds; minutes and seconds.
        75 | 76 | 78 | 80 => Time,
        // Thai date and time.
        77 => DateTime,
        // Thai [hours]:minutes:seconds.
        79 => Duration,
        _ => return None,
    };
    Some(kind)
}

#[cfg(test)]
mod tests;

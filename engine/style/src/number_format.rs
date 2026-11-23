//! The format codes a caller uses for dates and times, and the codes of the
//! built-in number formats (ECMA-376-1:2016 §18.8.30), which a styles part
//! refers to by id and does not define.

/// A date as `2026-11-23`. `xlsx-graft-read` reads a cell under it as a date.
pub const DATE: &str = "yyyy-mm-dd";

/// A time of day as `13:05:09`. `xlsx-graft-read` reads a cell under it as a
/// time.
pub const TIME: &str = "hh:mm:ss";

/// A date and a time of day as `2026-11-23 13:05:09`. `xlsx-graft-read`
/// reads a cell under it as a date and time.
pub const DATE_TIME: &str = "yyyy-mm-dd hh:mm:ss";

/// Elapsed time as `25:05:09`: the hours do not wrap at a day.
/// `xlsx-graft-read` reads a cell under it as an elapsed time.
pub const ELAPSED: &str = "[h]:mm:ss";

/// The id of the built-in number format whose code is exactly `code`, for
/// eleven of the formats in the "All Languages" table of ECMA-376-1:2016
/// §18.8.30, the table less its dates, times, accounting and other formats
/// (ids 14 to 22, 37 to 40 and 45 to 48): General, the numbers `0`, `0.00`,
/// `#,##0`, `#,##0.00`, `0%`, `0.00%`, `0.00E+00`, `# ?/?` and `# ??/??`, and
/// the text format `@`. A date or time has no id here: its code is written
/// into the styles part.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "StylesEdit uses it in the next pull request")
)]
pub(crate) fn builtin_id(code: &str) -> Option<u32> {
    let id = match code {
        "General" => 0,
        "0" => 1,
        "0.00" => 2,
        "#,##0" => 3,
        "#,##0.00" => 4,
        "0%" => 9,
        "0.00%" => 10,
        "0.00E+00" => 11,
        "# ?/?" => 12,
        "# ??/??" => 13,
        "@" => 49,
        _ => return None,
    };
    Some(id)
}

#[cfg(test)]
mod tests;

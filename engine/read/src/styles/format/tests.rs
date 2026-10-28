//! What a format code a workbook defines reads to.

use super::classify;
use crate::FormatKind::{self, Date, DateTime, Duration, General, Number, Text, Time, Unknown};

/// Asserts each code reads to its kind.
fn assert_kinds(cases: &[(&str, FormatKind)]) {
    for &(code, kind) in cases {
        assert_eq!(classify(code), kind, "{code}");
    }
}

#[test]
fn dates_times_and_elapsed_time_read_by_their_letters() {
    assert_kinds(&[
        ("yyyy-mm-dd", Date),
        ("yyyy/m/d", Date),
        ("YYYY-MM-DD", Date),
        ("dddd, mmmm dd", Date),
        ("mmmmm", Date),
        ("aaa", Date),
        ("yyyy/m/d(aaaa)", Date),
        ("h:mm", Time),
        ("hh:mm:ss.000", Time),
        ("h AM/PM", Time),
        ("h:mm a/p", Time),
        ("yyyy-mm-dd hh:mm:ss", DateTime),
        ("[h]:mm:ss", Duration),
        ("[mm]:ss", Duration),
        ("[ss].00", Duration),
        ("[HH]:MM", Duration),
    ]);
}

#[test]
fn an_m_is_minutes_after_hours_or_before_seconds_and_a_month_otherwise() {
    assert_kinds(&[
        ("mm:ss", Time),
        ("m", Date),
        ("h m", Time),
        ("m s", Time),
        ("mm:[ss]", Duration),
        ("m[ss]", Duration),
        // Literals between do not count: only the date and time letters do.
        (r#"h"時"mm"分""#, Time),
        ("[h]:mm", Duration),
        ("m/d/yy h:mm", DateTime),
        ("mm/dd hh:mm:ss", DateTime),
    ]);
}

#[test]
fn literals_brackets_and_escapes_hide_their_letters() {
    assert_kinds(&[
        (r#""m"0"#, Number),
        (r#"0" days""#, Number),
        (r"\d0", Number),
        (r"0\h", Number),
        ("_m0", Number),
        ("*d0", Number),
        ("[Red]0", Number),
        ("[Color10]#,##0", Number),
        ("[>=100]0;[<100]0.0", Number),
        ("[$€-407] #,##0.00", Number),
        (r#"[$-411]ggge"年"m"月"d"日""#, Date),
        ("[$-F800]dddd, mmmm dd, yyyy", Date),
        ("[DBNum1]yyyy", Date),
        ("[hm]0", Number),
        ("[]0", Number),
    ]);
}

#[test]
fn the_era_and_calendar_letters_are_dates_and_only_e_signs_are_exponents() {
    assert_kinds(&[
        ("e", Date),
        ("ggg", Date),
        ("0.0E+0", Number),
        ("0.0E-0", Number),
        // Office takes only `E+` and `E-` as exponents (MS-OI29500 on
        // §18.8.31, note e).
        ("0.0e+0", Date),
        ("E", Date),
        ("d/m/bb", Date),
        ("b2yyyy", Date),
        ("b10", Number),
        ("b1yyyy", Date),
        ("B20", Number),
    ]);
}

#[test]
fn sections_agree_on_a_kind_or_leave_it_unknown() {
    assert_kinds(&[
        (r#"0;[Red]-0;"zero";@"#, Number),
        ("#,##0;-#,##0", Number),
        ("General;-0", Number),
        ("0;General", Number),
        ("General;General", General),
        (r#""Total: "General"#, General),
        ("GENERAL", General),
        ("G/通用格式", General),
        ("[$-411]G/標準", General),
        (r#"G/通用格式"元""#, General),
        ("G/標準;-G/標準", General),
        ("yyyy/m/d;@", Date),
        // A section of literals alone does not count.
        (r#"yyyy/m/d;;"-""#, Date),
        (r#"#,##0;-#,##0;"-""#, Number),
        ("h:mm;h:mm;h:mm", Time),
        ("h:mm;0", Unknown),
        ("yyyy;h:mm", Unknown),
        ("[h]:mm;h:mm", Unknown),
        ("d [h]", Unknown),
        // The fourth section is for text, `@` or not.
        ("0;0;0;yyyy", Number),
        ("0;0;0;0;0", Unknown),
    ]);
}

#[test]
fn text_sections_alone_are_text_and_nothing_shown_is_general() {
    assert_kinds(&[
        ("@", Text),
        (r#""gross receipts for "@"#, Text),
        (";;;@", Text),
        ("", General),
        (";;;", General),
        // Literals alone show no number.
        ("\"n/a\"", General),
        ("-", General),
    ]);
}

#[test]
fn a_literal_or_brackets_left_open_do_not_read() {
    assert_kinds(&[
        ("\"abc", Unknown),
        ("0\\", Unknown),
        ("0_", Unknown),
        ("0*", Unknown),
        ("[Red0", Unknown),
        ("yyyy;\"m", Unknown),
    ]);
}

#[test]
fn a_fifth_section_stops_the_reading() {
    assert_kinds(&[
        ("0;0;0;@;", Unknown),
        // Read no further: what follows would not read either.
        (format!("{}\"", ";".repeat(100_000)).as_str(), Unknown),
        // However long a section, it is a few flags.
        ("ym".repeat(100_000).as_str(), Date),
        ("hmhm", Time),
    ]);
}

//! The built-in number format ids and the date and time format codes.

use super::{DATE, DATE_TIME, ELAPSED, TIME, builtin_id};

#[test]
fn the_language_independent_number_codes_have_their_ids() {
    let ids = [
        ("General", 0),
        ("0", 1),
        ("0.00", 2),
        ("#,##0", 3),
        ("#,##0.00", 4),
        ("0%", 9),
        ("0.00%", 10),
        ("0.00E+00", 11),
        ("# ?/?", 12),
        ("# ??/??", 13),
        ("@", 49),
    ];
    for (code, id) in ids {
        assert_eq!(builtin_id(code), Some(id), "{code}");
    }
}

#[test]
fn a_date_a_time_and_a_different_spelling_have_no_id() {
    for code in [
        "mm-dd-yy", "m/d/yyyy", "general", "GENERAL", "0.0", " 0", "0 ", "", DATE, TIME, DATE_TIME,
        ELAPSED,
    ] {
        assert_eq!(builtin_id(code), None, "{code:?}");
    }
}

#[test]
fn the_date_and_time_constants_hold_these_format_codes() {
    assert_eq!(
        [DATE, TIME, DATE_TIME, ELAPSED],
        ["yyyy-mm-dd", "hh:mm:ss", "yyyy-mm-dd hh:mm:ss", "[h]:mm:ss"]
    );
}

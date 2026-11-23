//! Tests for `number` and `integer`: the text of each boundary, and that
//! the text of any finite number reads back to it.

use super::{integer, number};
use crate::CellError;
use proptest::prelude::*;

#[test]
fn a_number_is_written_in_the_shortest_form_that_reads_back() {
    for (x, text) in [
        (0.0, "0"),
        (-0.0, "0"),
        (1.0, "1"),
        (-1.5, "-1.5"),
        (0.1, "0.1"),
        (123_456.789, "123456.789"),
        (1e-7, "0.0000001"),
        (-1e-7, "-0.0000001"),
        (1.5e-7, "0.00000015"),
        (9.999_999e-8, "9.999999E-8"),
        (1e-8, "1E-8"),
        (1.5e-10, "1.5E-10"),
        (-1.5e-10, "-1.5E-10"),
        (1e20, "100000000000000000000"),
        (1e21, "1E+21"),
        (-1.5e21, "-1.5E+21"),
        (1e300, "1E+300"),
        (f64::MAX, "1.7976931348623157E+308"),
        (f64::MIN_POSITIVE, "2.2250738585072014E-308"),
        (5e-324, "5E-324"),
    ] {
        assert_eq!(number(x).unwrap(), text, "{x:e}");
    }
}

#[test]
fn a_number_that_is_not_finite_is_refused() {
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(number(x), Err(CellError::NotFinite)));
    }
}

#[test]
fn an_integer_is_written_up_to_two_to_the_53() {
    assert_eq!(integer(0).unwrap(), "0");
    assert_eq!(integer(-42).unwrap(), "-42");
    assert_eq!(integer(9_007_199_254_740_992).unwrap(), "9007199254740992");
    assert_eq!(
        integer(-9_007_199_254_740_992).unwrap(),
        "-9007199254740992"
    );
    for n in [
        9_007_199_254_740_993,
        -9_007_199_254_740_993,
        i64::MAX,
        i64::MIN,
    ] {
        assert!(matches!(integer(n), Err(CellError::InexactInteger)), "{n}");
    }
}

proptest! {
    #[test]
    fn the_text_of_a_finite_number_reads_back_to_it(bits in any::<u64>()) {
        let x = f64::from_bits(bits);
        prop_assume!(x.is_finite());
        let text = number(x).unwrap();
        let read: f64 = text.parse().unwrap();
        prop_assert_eq!(read, x);
        prop_assert!(!text.contains('e') && !text.contains("E+-"));
    }

    #[test]
    fn the_text_of_a_number_near_the_boundaries_reads_back_to_it(
        mantissa in 1.0f64..10.0,
        exponent in prop::sample::select(&[-9, -8, -7, -6, 19, 20, 21, 22][..]),
        negative in any::<bool>(),
    ) {
        let x = mantissa * 10f64.powi(exponent) * if negative { -1.0 } else { 1.0 };
        let read: f64 = number(x).unwrap().parse().unwrap();
        prop_assert_eq!(read, x);
    }
}

//! Unit tests for `unprefixed` and `look_up`: the prefixes removed, and
//! what the table knows of a name in each of its three states.

use super::{Baseline, ECMA_FUNCTIONS, Known, look_up, unprefixed};
use crate::{Danger, Dangerous, Prefix, Safety, function};

#[test]
fn unprefixed_removes_xlfn_then_xlws_in_any_case() {
    assert_eq!(unprefixed("_xlfn._xlws.SORT"), "SORT");
    assert_eq!(unprefixed("_xlws.SORT"), "SORT");
    assert_eq!(unprefixed("_xlfn.SORT"), "SORT");
    assert_eq!(unprefixed("_XLFN.xlookup"), "xlookup");
    assert_eq!(unprefixed("_XlFn._XLWS.sort"), "sort");
}

#[test]
fn unprefixed_removes_nothing_else() {
    assert_eq!(unprefixed("SUM"), "SUM");
    assert_eq!(unprefixed("_xll.FOO"), "_xll.FOO");
    // `_xlfn.` is removed only at the start, before `_xlws.`.
    assert_eq!(unprefixed("_xlws._xlfn.SORT"), "_xlfn.SORT");
    assert_eq!(unprefixed("_xlfn._xlfn.SORT"), "_xlfn.SORT");
    assert_eq!(unprefixed("x_xlfn.SORT"), "x_xlfn.SORT");
}

#[test]
fn unprefixed_accepts_any_string() {
    assert_eq!(unprefixed(""), "");
    assert_eq!(unprefixed("_xlfn."), "");
    assert_eq!(unprefixed("_xlfn._xlws."), "");
    // A multi-byte character that straddles the prefix's length.
    assert_eq!(unprefixed("ÄÄÄÄ"), "ÄÄÄÄ");
    assert_eq!(unprefixed("_xlfÄ.SORT"), "_xlfÄ.SORT");
    assert_eq!(unprefixed("_xlfn.ÄBC"), "ÄBC");
}

#[test]
fn a_function_with_a_row_is_versioned_under_its_unprefixed_upper_case_name() {
    for written in ["XLOOKUP", "_xlfn.xlookup"] {
        let lookup = look_up(written);
        assert_eq!(lookup.name, "XLOOKUP");
        let Known::Versioned { row, safety } = lookup.known else {
            panic!("{written} is not versioned");
        };
        assert_eq!(row, function("XLOOKUP").unwrap());
        assert!(matches!(safety, Safety::ReviewedSafe(_)));
    }
    assert_eq!(look_up("_xlfn._xlws.sort").name, "SORT");
    assert!(matches!(
        look_up("_xlfn._xlws.sort").known,
        Known::Versioned { .. }
    ));
}

#[test]
fn a_function_ecma_376_defines_is_baseline_with_a_bare_prefix_and_its_source() {
    let lookup = look_up("sum");
    assert_eq!(lookup.name, "SUM");
    let Known::Baseline { baseline, safety } = lookup.known else {
        panic!("SUM is not baseline");
    };
    assert_eq!(
        baseline,
        Baseline {
            prefix: Prefix::Bare,
            sources: &[ECMA_FUNCTIONS],
        }
    );
    assert!(matches!(safety, Safety::ReviewedSafe(_)));
}

#[test]
fn a_function_ecma_376_does_not_define_is_baseline_with_an_unknown_prefix_and_no_source() {
    let lookup = look_up("ANCHORARRAY");
    let Known::Baseline { baseline, safety } = lookup.known else {
        panic!("ANCHORARRAY is not baseline");
    };
    assert_eq!(
        baseline,
        Baseline {
            prefix: Prefix::Unknown,
            sources: &[],
        }
    );
    assert!(matches!(safety, Safety::ReviewedSafe(_)));

    let lookup = look_up("_xlfn.call");
    assert_eq!(lookup.name, "CALL");
    assert!(matches!(
        lookup.known,
        Known::Baseline {
            safety: Safety::Dangerous(Dangerous {
                danger: Danger::ExternalCode,
                ..
            }),
            ..
        }
    ));
}

#[test]
fn a_name_the_table_does_not_list_is_unknown() {
    for written in ["NOPE", "", "_xlfn.", "ÄBC", "_xlws._xlfn.SORT", "_xll.FOO"] {
        let lookup = look_up(written);
        assert_eq!(lookup.known, Known::Unknown, "{written}");
    }
    assert_eq!(look_up("_xlws._xlfn.SORT").name, "_XLFN.SORT");
    assert_eq!(look_up("_xll.FOO").name, "_XLL.FOO");
}

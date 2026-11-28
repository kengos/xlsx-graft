//! `set_column_width`: the `cols` written for a part with and without
//! `col`s, the `col`s cut at the edges of the widths, and what is refused.

use super::{run, sheet, text};
use crate::sheet::SheetError;

/// The sheet with the `cols` element `cols` before its `sheetData`.
fn with(cols: &str) -> String {
    sheet(&format!("{cols}<sheetData/>"))
}

#[test]
fn a_part_with_no_cols_gets_one_before_the_sheetdata() {
    let got = text(&sheet("<sheetData/>"), |edit| {
        edit.set_column_width(2, 4, Some(20.0)).unwrap();
        edit.set_column_width(8, 8, Some(12.5)).unwrap();
        edit.set_column_width(10, 11, None).unwrap();
    });
    let expected = with(
        r#"<cols><col min="2" max="4" width="20" customWidth="1"/><col min="8" max="8" width="12.5" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_later_call_overrides_the_columns_of_an_earlier_one() {
    let got = text(&sheet("<sheetData/>"), |edit| {
        edit.set_column_width(1, 10, Some(10.0)).unwrap();
        edit.set_column_width(3, 4, Some(20.0)).unwrap();
    });
    let expected = with(
        r#"<cols><col min="1" max="2" width="10" customWidth="1"/><col min="3" max="4" width="20" customWidth="1"/><col min="5" max="10" width="10" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
    // Overlapping from the left, from the right, and one that removes.
    let got = text(&sheet("<sheetData/>"), |edit| {
        edit.set_column_width(1, 5, Some(1.0)).unwrap();
        edit.set_column_width(4, 8, Some(2.0)).unwrap();
        edit.set_column_width(10, 12, Some(3.0)).unwrap();
        edit.set_column_width(2, 11, None).unwrap();
    });
    let expected = with(
        r#"<cols><col min="1" max="1" width="1" customWidth="1"/><col min="12" max="12" width="3" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
    let got = text(&sheet("<sheetData/>"), |edit| {
        edit.set_column_width(5, 8, Some(1.0)).unwrap();
        edit.set_column_width(1, 6, Some(2.0)).unwrap();
    });
    let expected = with(
        r#"<cols><col min="1" max="6" width="2" customWidth="1"/><col min="7" max="8" width="1" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_col_is_cut_at_the_edges_of_the_widths_and_keeps_its_other_attributes() {
    let xml = with(r#"<cols><col min="1" max="10" width="9" style="3" customWidth="1"/></cols>"#);
    let got = text(&xml, |edit| {
        edit.set_column_width(5, 5, Some(30.0)).unwrap()
    });
    let expected = with(
        r#"<cols><col min="1" max="4" width="9" style="3" customWidth="1"/><col min="5" max="5" width="30" style="3" customWidth="1"/><col min="6" max="10" width="9" style="3" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
    // A width removed leaves the style, and a piece with nothing else goes.
    let got = text(&xml, |edit| edit.set_column_width(5, 5, None).unwrap());
    let expected = with(
        r#"<cols><col min="1" max="4" width="9" style="3" customWidth="1"/><col min="5" max="5" style="3"/><col min="6" max="10" width="9" style="3" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
    let bare = with(r#"<cols><col min="1" max="10" width="9" customWidth="1"/></cols>"#);
    let got = text(&bare, |edit| edit.set_column_width(5, 5, None).unwrap());
    let expected = with(
        r#"<cols><col min="1" max="4" width="9" customWidth="1"/><col min="6" max="10" width="9" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
    let got = text(&bare, |edit| {
        edit.set_column_width(1, 16_384, None).unwrap()
    });
    assert_eq!(got, with("<cols></cols>"));
}

#[test]
fn a_width_is_set_in_a_col_that_has_none_and_over_one_with_customwidth_zero() {
    let xml = with(
        r#"<cols><col min="1" max="3" style="2"/><col min="4" max="4" width="9" customWidth="0"/></cols>"#,
    );
    let got = text(&xml, |edit| edit.set_column_width(1, 4, Some(7.0)).unwrap());
    let expected = with(
        r#"<cols><col min="1" max="3" style="2" width="7" customWidth="1"/><col min="4" max="4" width="7" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_col_no_width_touches_is_written_as_it_was() {
    let xml = with(
        "<cols>\n <col  min='1' max=\"3\"   width=\"5\" />\n <col min=\"5\" max=\"6\" width=\"8\"></col>\n</cols>",
    );
    let got = text(&xml, |edit| {
        edit.set_column_width(20, 21, Some(9.0)).unwrap()
    });
    let expected = with(
        "<cols>\n <col  min='1' max=\"3\"   width=\"5\" />\n <col min=\"5\" max=\"6\" width=\"8\"></col>\n<col min=\"20\" max=\"21\" width=\"9\" customWidth=\"1\"/></cols>",
    );
    assert_eq!(got, expected);
}

#[test]
fn a_width_in_a_gap_is_written_before_the_col_after_it() {
    let xml =
        with(r#"<cols><col min="5" max="6" width="8"/><col min="9" max="9" width="4"/></cols>"#);
    let got = text(&xml, |edit| {
        edit.set_column_width(1, 2, Some(3.0)).unwrap();
        edit.set_column_width(7, 8, Some(6.0)).unwrap();
        edit.set_column_width(5, 5, Some(2.0)).unwrap();
    });
    let expected = with(
        r#"<cols><col min="1" max="2" width="3" customWidth="1"/><col min="5" max="5" width="2" customWidth="1"/><col min="6" max="6" width="8"/><col min="7" max="8" width="6" customWidth="1"/><col min="9" max="9" width="4"/></cols>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_width_over_two_cols_cuts_both_and_fills_the_gap_between() {
    let xml =
        with(r#"<cols><col min="1" max="3" width="9"/><col min="6" max="8" width="9"/></cols>"#);
    let got = text(&xml, |edit| edit.set_column_width(3, 7, Some(1.0)).unwrap());
    let expected = with(
        r#"<cols><col min="1" max="2" width="9"/><col min="3" max="3" width="1" customWidth="1"/><col min="4" max="5" width="1" customWidth="1"/><col min="6" max="7" width="1" customWidth="1"/><col min="8" max="8" width="9"/></cols>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_col_with_an_end_tag_is_replaced_by_its_pieces() {
    let xml = with(r#"<cols><col min="1" max="3" width="9"></col></cols>"#);
    let got = text(&xml, |edit| edit.set_column_width(2, 2, Some(1.0)).unwrap());
    let expected = with(
        r#"<cols><col min="1" max="1" width="9"/><col min="2" max="2" width="1" customWidth="1"/><col min="3" max="3" width="9"/></cols>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn a_col_whose_min_or_max_is_not_a_column_is_left_alone() {
    for col in [
        r#"<col min="x" max="3" width="9"/>"#,
        r#"<col max="3" width="9"/>"#,
        r#"<col min="4" max="3" width="9"/>"#,
        r#"<col min="0" max="3" width="9"/>"#,
        r#"<col min="1" max="16385" width="9"/>"#,
    ] {
        let got = text(&with(&format!("<cols>{col}</cols>")), |edit| {
            edit.set_column_width(1, 2, Some(1.0)).unwrap();
        });
        let expected = with(&format!(
            r#"<cols>{col}<col min="1" max="2" width="1" customWidth="1"/></cols>"#
        ));
        assert_eq!(got, expected, "{col}");
    }
}

#[test]
fn an_empty_cols_is_filled_when_a_width_is_set_and_left_when_none_is() {
    let got = text(&with("<cols/>"), |edit| {
        edit.set_column_width(1, 2, Some(1.0)).unwrap()
    });
    assert_eq!(
        got,
        with(r#"<cols><col min="1" max="2" width="1" customWidth="1"/></cols>"#)
    );
    let got = text(&with("<cols/>"), |edit| {
        edit.set_column_width(1, 2, None).unwrap()
    });
    assert_eq!(got, with("<cols/>"));
}

#[test]
fn several_cols_take_a_width_in_the_gap_before_the_col_it_comes_before() {
    let xml = with(
        r#"<cols><col min="1" max="2" width="9"/></cols><cols><col min="5" max="6" width="9"/></cols>"#,
    );
    let got = text(&xml, |edit| {
        edit.set_column_width(3, 3, Some(1.0)).unwrap();
        edit.set_column_width(6, 6, Some(2.0)).unwrap();
    });
    let expected = with(
        r#"<cols><col min="1" max="2" width="9"/></cols><cols><col min="3" max="3" width="1" customWidth="1"/><col min="5" max="5" width="9"/><col min="6" max="6" width="2" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn the_last_column_is_set_and_text_and_other_children_of_cols_are_kept() {
    let xml =
        with("<cols>\n<col min=\"16384\" max=\"16384\" width=\"9\"/><!--c--><x><y/></x></cols>");
    let got = text(&xml, |edit| {
        edit.set_column_width(16_384, 16_384, Some(1.0)).unwrap();
    });
    let expected = with(
        "<cols>\n<col min=\"16384\" max=\"16384\" width=\"1\" customWidth=\"1\"/><!--c--><x><y/></x></cols>",
    );
    assert_eq!(got, expected);
}

#[test]
fn a_call_that_is_refused_changes_nothing() {
    let xml = with(r#"<cols><col min="1" max="2" width="9"/></cols>"#);
    let got = run(&xml, |edit| {
        let range = SheetError::ColumnRange { first: 0, last: 1 };
        assert_eq!(edit.set_column_width(0, 1, Some(1.0)), Err(range));
        let range = SheetError::ColumnRange { first: 3, last: 2 };
        assert_eq!(edit.set_column_width(3, 2, None), Err(range));
        let range = SheetError::ColumnRange {
            first: 1,
            last: 16_385,
        };
        assert_eq!(edit.set_column_width(1, 16_385, None), Err(range));
        for width in [-1.0, 255.5, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                edit.set_column_width(1, 2, Some(width)),
                Err(SheetError::ColumnWidth { .. })
            ));
        }
        assert_eq!(edit.set_column_width(1, 2, Some(255.0)), Ok(()));
        assert_eq!(edit.set_column_width(1, 2, Some(0.0)), Ok(()));
    });
    assert_eq!(
        got.sheet(),
        with(r#"<cols><col min="1" max="2" width="0" customWidth="1"/></cols>"#)
    );
}

#[test]
fn a_width_set_again_leaves_the_width_of_the_next_column_alone() {
    // The columns set again end where the next width starts.
    let got = text(&sheet("<sheetData/>"), |edit| {
        edit.set_column_width(1, 1, Some(1.0)).unwrap();
        edit.set_column_width(2, 2, Some(2.0)).unwrap();
        edit.set_column_width(1, 1, Some(3.0)).unwrap();
    });
    let expected = with(
        r#"<cols><col min="1" max="1" width="3" customWidth="1"/><col min="2" max="2" width="2" customWidth="1"/></cols>"#,
    );
    assert_eq!(got, expected);
}

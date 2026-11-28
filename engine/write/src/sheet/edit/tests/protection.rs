//! `protect` and `unprotect`: the `sheetProtection` written or left out.

use super::{run, sheet, text};
use crate::sheet::{PasswordHash, SheetProtection};

const OLD: &str = r#"<sheetProtection password="CC1A" sheet="1" objects="1" formatCells="0"/>"#;

#[test]
fn a_part_with_no_sheetprotection_gets_one_where_the_schema_puts_it() {
    let got = text(&sheet("<sheetData/><pageMargins/>"), |edit| {
        edit.protect(SheetProtection::default());
    });
    let expected = sheet(
        r#"<sheetData/><sheetProtection sheet="1" objects="1" scenarios="1"/><pageMargins/>"#,
    );
    assert_eq!(got, expected);
}

#[test]
fn the_sheetprotection_the_part_has_is_replaced_by_the_one_given() {
    let xml = sheet(&format!("<sheetData/>{OLD}<pageMargins/>"));
    let protection = SheetProtection {
        allow_format_cells: true,
        password: Some(PasswordHash::sha512("pw", [1; 16], 100).unwrap()),
        ..SheetProtection::default()
    };
    let expected = {
        let mut out = Vec::new();
        protection.write(&mut out, "").unwrap();
        String::from_utf8(out).unwrap()
    };
    let got = text(&xml, |edit| edit.protect(protection));
    assert_eq!(got, sheet(&format!("<sheetData/>{expected}<pageMargins/>")));
    assert!(got.contains(r#"algorithmName="SHA-512""#));
    assert!(!got.contains("CC1A"));
}

#[test]
fn the_last_call_overrides_the_earlier_ones() {
    let xml = sheet(&format!("<sheetData/>{OLD}"));
    let got = text(&xml, |edit| {
        edit.protect(SheetProtection::default());
        edit.unprotect();
    });
    assert_eq!(got, sheet("<sheetData/>"));
    let got = text(&xml, |edit| {
        edit.unprotect();
        edit.protect(SheetProtection::default());
    });
    assert_eq!(
        got,
        sheet(r#"<sheetData/><sheetProtection sheet="1" objects="1" scenarios="1"/>"#)
    );
}

#[test]
fn unprotect_leaves_out_the_sheetprotection_in_any_form() {
    for old in [
        OLD,
        "<sheetProtection/>",
        "<sheetProtection sheet=\"1\"><x/></sheetProtection>",
    ] {
        let xml = sheet(&format!("<sheetData/>{old}<pageMargins/>"));
        let got = text(&xml, |edit| edit.unprotect());
        assert_eq!(got, sheet("<sheetData/><pageMargins/>"), "{old}");
    }
}

#[test]
fn unprotect_on_a_sheet_that_is_not_protected_changes_nothing() {
    let xml = sheet("<sheetData/>");
    let got = run(&xml, |edit| edit.unprotect());
    assert!(got.holds(&xml));
    let got = run(&xml, |edit| {
        edit.protect(SheetProtection::default());
        edit.unprotect();
    });
    assert!(got.holds(&xml));
}

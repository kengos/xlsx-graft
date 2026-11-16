//! The sheet asked for: by default the first, by name and by index; and how
//! each sheet that cannot be read is refused.

use super::{asking, book, described, rel_type, rows_only, single, workbook};
use crate::sheet::SheetRequest;
use serde_json::{Value, json};
use xlsx_graft_read::SheetChoice;

/// The cell `A1` holding `n`, as a row.
fn one(n: u32) -> String {
    rows_only(&format!(r#"<row r="1"><c r="A1"><v>{n}</v></c></row>"#))
}

/// A request for `choice`.
fn choosing(choice: SheetChoice<'_>) -> SheetRequest {
    SheetRequest::new(Some(choice), None, None, None).unwrap()
}

/// The `{index, name, kind, state}` of a view's JSON as a string, `-` for
/// null, and its `error.code` or `-`.
fn chosen(value: &Value) -> (String, String) {
    let sheet = &value["sheet"];
    let sheet = if sheet.is_null() {
        "-".to_owned()
    } else {
        format!(
            "{} {} {} {}",
            sheet["index"], sheet["name"], sheet["kind"], sheet["state"]
        )
    };
    let code = value["error"]["code"].as_str().unwrap_or("-");
    (sheet, code.to_owned())
}

/// A workbook of the worksheets `One`, `two` (hidden) and `Three` (very
/// hidden), a chartsheet `Chart`, a sheet `Gone` whose relationship is
/// missing, a macro sheet `Macro` and a dialog sheet `Dialog`.
fn assorted() -> Vec<u8> {
    let tags = concat!(
        r#"<sheet name="One" sheetId="1" r:id="rId1"/>"#,
        r#"<sheet name="two" sheetId="2" state="hidden" r:id="rId2"/>"#,
        r#"<sheet name="Three" sheetId="3" state="veryHidden" r:id="rId3"/>"#,
        r#"<sheet name="Chart" sheetId="4" r:id="rId4"/>"#,
        r#"<sheet name="Gone" sheetId="5" r:id="rId5"/>"#,
        r#"<sheet name="Macro" sheetId="6" r:id="rId6"/>"#,
        r#"<sheet name="Dialog" sheetId="7" r:id="rId7"/>"#,
    );
    let worksheet = rel_type("worksheet");
    let mut related = String::new();
    for (id, target) in [("rId1", "s1"), ("rId2", "s2"), ("rId3", "s3")] {
        let target = format!("worksheets/{target}.xml");
        related.push_str(&crate::support::rel_of(id, &worksheet, &target));
    }
    related.push_str(&crate::support::rel_of(
        "rId4",
        &rel_type("chartsheet"),
        "chartsheets/c.xml",
    ));
    related.push_str(&crate::support::rel_of(
        "rId6",
        "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet",
        "macrosheets/m.xml",
    ));
    related.push_str(&crate::support::rel_of(
        "rId7",
        &rel_type("dialogsheet"),
        "dialogsheets/d.xml",
    ));
    let (s1, s2, s3) = (one(1), one(2), one(3));
    let parts = [
        ("xl/worksheets/s1.xml", s1.as_str()),
        ("xl/worksheets/s2.xml", s2.as_str()),
        ("xl/worksheets/s3.xml", s3.as_str()),
        ("xl/chartsheets/c.xml", "<chartsheet/>"),
        ("xl/macrosheets/m.xml", "<macroSheet/>"),
        ("xl/dialogsheets/d.xml", "<dialogsheet/>"),
    ];
    workbook(tags, &related, &parts)
}

#[test]
fn no_sheet_asked_for_is_the_first_whatever_its_state() {
    let value = described(&single(&one(7)), &asking(None, None, None));
    assert_eq!(
        chosen(&value),
        (
            r#"0 "Sheet1" "worksheet" "visible""#.to_owned(),
            "-".to_owned()
        )
    );
    assert_eq!(value["rows"][0]["cells"][0]["value"], 7.0);
}

#[test]
fn a_sheet_is_chosen_by_name_letter_case_aside_or_by_index_with_its_kind_and_state() {
    let bytes = assorted();
    let cases = [
        (
            SheetChoice::Name("ONE"),
            r#"0 "One" "worksheet" "visible""#,
            1.0,
        ),
        (
            SheetChoice::Name("TWO"),
            r#"1 "two" "worksheet" "hidden""#,
            2.0,
        ),
        (
            SheetChoice::Index(2),
            r#"2 "Three" "worksheet" "veryHidden""#,
            3.0,
        ),
    ];
    for (choice, sheet, value) in cases {
        let found = described(&bytes, &choosing(choice));
        assert_eq!(
            chosen(&found),
            (sheet.to_owned(), "-".to_owned()),
            "{choice:?}"
        );
        assert_eq!(found["rows"][0]["cells"][0]["value"], value);
    }
}

#[test]
fn a_sheet_that_is_no_worksheet_or_has_no_part_is_refused_with_the_sheet_named() {
    let bytes = assorted();
    let cases = [
        (
            SheetChoice::Name("chart"),
            r#"3 "Chart" "chartsheet" "visible""#,
            "sheet_not_worksheet",
        ),
        (
            SheetChoice::Index(5),
            r#"5 "Macro" "macrosheet" "visible""#,
            "sheet_not_worksheet",
        ),
        (
            SheetChoice::Name("dialog"),
            r#"6 "Dialog" "dialogsheet" "visible""#,
            "sheet_not_worksheet",
        ),
        (
            SheetChoice::Name("gone"),
            r#"4 "Gone" "unknown" "visible""#,
            "sheet_unreadable",
        ),
    ];
    for (choice, sheet, code) in cases {
        let value = described(&bytes, &choosing(choice));
        assert_eq!(
            chosen(&value),
            (sheet.to_owned(), code.to_owned()),
            "{choice:?}"
        );
        assert_eq!(value["ok"], false);
        assert_eq!(value["shown"], Value::Null);
        assert_eq!(value["used"], Value::Null);
        assert_eq!(value["complete"], false);
        assert_eq!(value["rows"], json!([]));
    }
    let chart = described(&bytes, &choosing(SheetChoice::Index(3)));
    assert_eq!(
        chart["error"]["message"],
        r#"sheet "Chart" is a chartsheet, which holds no cells"#
    );
}

#[test]
fn a_sheet_that_is_not_found_or_ambiguous_leaves_sheet_null() {
    let bytes = book(&[("A", &one(1)), ("a", &one(2)), ("B", &one(3))]);
    let cases = [
        (
            SheetChoice::Name("nope"),
            "sheet_not_found",
            r#"no sheet is named "nope""#,
        ),
        (
            SheetChoice::Index(3),
            "sheet_not_found",
            "there is no sheet at index 3: the workbook has 3, from index 0",
        ),
        (
            SheetChoice::Name("A"),
            "sheet_ambiguous",
            r#"more than one sheet is named "A", letter case aside"#,
        ),
    ];
    for (choice, code, message) in cases {
        let value = described(&bytes, &choosing(choice));
        assert_eq!(
            chosen(&value),
            ("-".to_owned(), code.to_owned()),
            "{choice:?}"
        );
        assert_eq!(value["error"]["message"], message);
        assert_eq!(value["ok"], false);
        assert_eq!(value["shown"], Value::Null);
        assert_eq!(value["rows"], json!([]));
    }
}

#[test]
fn the_names_in_an_error_message_are_written_as_an_echo_does() {
    let bytes = assorted();
    let asked = "x\u{85}\u{202e}[31mred";
    let value = described(&bytes, &choosing(SheetChoice::Name(asked)));
    assert_eq!(value["error"]["message"], r#"no sheet is named "x[31mred""#);
    let long = "L".repeat(300);
    let value = described(&bytes, &choosing(SheetChoice::Name(&long)));
    let message = value["error"]["message"].as_str().unwrap();
    assert!(message.ends_with("…\""), "{message}");
    assert!(message.chars().count() < 260);
}

#[test]
fn a_sheet_name_from_the_workbook_is_written_as_an_echo_does_in_sheet_and_in_the_message() {
    let tags = r#"<sheet name="Ch&#x202e;art&#x85;" sheetId="1" r:id="rId1"/>"#;
    let related = crate::support::rel_of("rId1", &rel_type("chartsheet"), "chartsheets/c.xml");
    let bytes = workbook(tags, &related, &[("xl/chartsheets/c.xml", "<chartsheet/>")]);
    let value = described(&bytes, &asking(None, None, None));
    assert_eq!(value["sheet"]["name"], "Chart");
    assert_eq!(
        value["error"]["message"],
        r#"sheet "Chart" is a chartsheet, which holds no cells"#
    );
    let gone = r#"<sheet name="Gone&#x202e;" sheetId="1" r:id="rId9"/>"#;
    let bytes = workbook(gone, "", &[]);
    let value = described(&bytes, &asking(None, None, None));
    assert_eq!(value["error"]["code"], "sheet_unreadable");
    assert_eq!(
        value["error"]["message"],
        r#"sheet "Gone" cannot be read; the diagnostics say why"#
    );
}

#[test]
fn bytes_that_are_no_workbook_are_refused_with_the_diagnostics_that_say_why() {
    let value = described(b"this is not a zip file", &asking(None, None, None));
    assert_eq!(
        chosen(&value),
        ("-".to_owned(), "workbook_unreadable".to_owned())
    );
    assert_eq!(value["ok"], false);
    assert!(!value["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(
        value["error"]["message"],
        "the workbook cannot be read; the diagnostics say why"
    );
    assert_eq!(value["complete"], false);
}

//! What stops the reading: a file that cannot be opened, a reader or a
//! spool that fails, a sheet that stops at a problem, and the limit of
//! diagnostics; and the entry points that open a file or bytes.

use super::{
    Reader, Unreadable, asking, book, codes, described, filled, rows_only, single, view, view_with,
};
use crate::policy::limits::HostLimits;
use crate::sheet::{describe_sheet, describe_sheet_bytes, describe_sheet_path};
use crate::support::{FIXTURES, json};
use serde_json::{Value, json};
use std::io::{self, Cursor};
use std::path::Path;
use xlsx_graft_diagnostics::Limit;
use xlsx_graft_spool::{MemorySpools, Spool, SpoolFactory};

/// A factory that cannot make a spool.
struct Refusing;

impl SpoolFactory for Refusing {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        Err(io::Error::other("no room for a spool"))
    }
}

/// The limits under which the shared strings table moves to a spool at its
/// first string.
fn spilling() -> HostLimits {
    let mut limits = HostLimits::default();
    limits.read.shared_strings_spill_bytes = Limit::policy_of(0);
    limits
}

/// The package of a workbook of one worksheet, `Sheet1`, whose rows are the
/// XML `rows`.
fn with_strings(rows: &str) -> Vec<u8> {
    single(&rows_only(rows))
}

#[test]
fn a_path_that_cannot_be_opened_is_an_io_error_with_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.xlsx");
    let request = asking(None, None, None);
    for path in [missing.as_path(), dir.path()] {
        let value = json(
            &describe_sheet_path(path, &request, &HostLimits::default())
                .to_json()
                .unwrap(),
        );
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "io");
        assert_eq!(value["sheet"], Value::Null);
        assert_eq!(value["complete"], false);
        assert_eq!(value["rows"], json!([]));
        assert_eq!(value["diagnostics"], json!([]));
    }
    let message = |path: &Path| {
        let view = describe_sheet_path(path, &request, &HostLimits::default());
        view.error.unwrap().message
    };
    assert!(message(&missing).contains("could not be opened"));
    assert!(message(dir.path()).contains("is not a regular file"));
}

#[test]
fn a_file_and_bytes_give_the_same_view_as_a_reader() {
    let bytes = with_strings(r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("book.xlsx");
    std::fs::write(&path, &bytes).unwrap();
    let request = asking(None, None, None);
    let limits = HostLimits::default();
    let expected = view(&bytes, &request).to_json().unwrap();
    assert!(expected.contains(r#""value":"shared""#), "{expected}");
    assert_eq!(
        describe_sheet_path(&path, &request, &limits)
            .to_json()
            .unwrap(),
        expected
    );
    assert_eq!(
        describe_sheet_bytes(&bytes, &request, &limits)
            .to_json()
            .unwrap(),
        expected
    );
    let reader = describe_sheet(Cursor::new(&bytes), &request, &limits);
    assert_eq!(reader.to_json().unwrap(), expected);
}

#[test]
fn shared_strings_that_move_to_a_temporary_file_are_read_from_it() {
    let rows = r#"<row r="1"><c r="A1" t="s"><v>2</v></c><c r="B1" t="s"><v>1</v></c></row>"#;
    let bytes = with_strings(rows);
    let reader = Cursor::new(&bytes);
    let view = describe_sheet(reader, &asking(None, None, None), &spilling());
    let value = json(&view.to_json().unwrap());
    assert_eq!(value["ok"], true);
    assert_eq!(value["rows"][0]["cells"][0]["value"], "line\nbreak");
    assert_eq!(value["rows"][0]["cells"][1]["value"], "a|b");
}

#[test]
fn a_reader_that_fails_opening_the_package_is_an_io_error_and_no_sheet() {
    let reader = Reader {
        inner: Cursor::new(single(&rows_only(""))),
        poisoned: 0..u64::MAX,
    };
    let view = crate::sheet::describe(
        reader,
        &asking(None, None, None),
        &HostLimits::default(),
        &mut MemorySpools,
    );
    let value = json(&view.to_json().unwrap());
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "io");
    assert_eq!(
        value["error"]["message"],
        "the package could not be read: the disk went away"
    );
    assert_eq!(value["sheet"], Value::Null);
}

#[test]
fn a_reader_that_fails_in_the_middle_of_the_sheet_keeps_the_rows_read_and_is_not_complete() {
    let bytes = single(&rows_only(&filled(10_000, 1)));
    let at = bytes.windows(11).position(|w| w == b"<sheetData>").unwrap();
    let start = u64::try_from(at).unwrap();
    let reader = Reader {
        inner: Cursor::new(bytes),
        poisoned: start + 100_000..start + 200_000,
    };
    let request = asking(None, None, None);
    let limits = HostLimits::default();
    let view = crate::sheet::describe(reader, &request, &limits, &mut MemorySpools);
    let value = json(&view.to_json().unwrap());
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "io");
    assert_eq!(
        value["error"]["message"],
        "the package could not be read: the disk went away"
    );
    assert_eq!(value["complete"], false);
    assert_eq!(value["sheet"]["name"], "Sheet1");
    assert_eq!(value["rows"].as_array().unwrap().len(), 100);
    assert_eq!(value["shown"], "A1:A100");
}

#[test]
fn a_spool_that_cannot_be_made_is_an_io_error_and_no_sheet() {
    let bytes = with_strings(r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#);
    let request = asking(None, None, None);
    let view = view_with(&bytes, &request, &spilling(), &mut Refusing);
    let value = json(&view.to_json().unwrap());
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "io");
    let message = value["error"]["message"].as_str().unwrap();
    assert!(
        message.starts_with("the shared strings could not be moved to a temporary file: "),
        "{message}"
    );
    assert!(message.contains("no room for a spool"), "{message}");
    assert_eq!(value["sheet"], Value::Null);
    assert_eq!(value["rows"], json!([]));
}

#[test]
fn a_spool_that_fails_a_lookup_is_an_io_error_that_keeps_the_rows_before_it() {
    let rows = concat!(
        r#"<row r="1"><c r="A1"><v>1</v></c></row>"#,
        r#"<row r="2"><c r="A2" t="s"><v>0</v></c></row>"#,
    );
    let bytes = with_strings(rows);
    let request = asking(None, None, None);
    let view = view_with(&bytes, &request, &spilling(), &mut Unreadable);
    let value = json(&view.to_json().unwrap());
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "io");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("the shared strings could not be moved to a temporary file: ")
    );
    assert_eq!(value["complete"], false);
    assert_eq!(value["sheet"]["index"], 0);
    assert_eq!(value["rows"].as_array().unwrap().len(), 1);
    assert_eq!(value["used"], "A1:A1");
}

#[test]
fn a_sheet_that_stops_at_a_problem_gives_the_rows_before_it_and_is_not_complete() {
    let rows = concat!(
        r#"<row r="1"><c r="A1"><v>1</v></c></row>"#,
        r#"<row r="2"><c r="B2" t="e"><v></v></c></row>"#,
        r#"<row r="3"><c r="C3"><v>3</v></c></row>"#,
    );
    let value = described(&with_strings(rows), &asking(None, None, None));
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"], Value::Null);
    assert_eq!(codes(&value), ["xml_schema_invalid"]);
    assert_eq!(value["complete"], false);
    assert_eq!(value["used"], "A1:A1");
    assert_eq!(value["shown"], "A1:A1");
    assert_eq!(value["rows"].as_array().unwrap().len(), 1);
}

#[test]
fn the_diagnostics_are_listed_up_to_the_limit_and_the_view_says_it_cut_them() {
    let one = rows_only("");
    let bytes = book(&[("A", &one), ("a", &one), ("a", &one)]);
    let limits = HostLimits {
        max_diagnostics: Limit::policy_of(1),
        ..HostLimits::default()
    };
    let request = asking(None, None, None);
    let view = view_with(&bytes, &request, &limits, &mut MemorySpools);
    let value = json(&view.to_json().unwrap());
    assert_eq!(
        codes(&value),
        ["sheet_name_duplicate", "diagnostics_truncated"]
    );
}

#[test]
fn a_workbook_of_the_fixtures_is_read() {
    let path = Path::new(FIXTURES).join("table.xlsx");
    let view = describe_sheet_path(&path, &asking(None, None, None), &HostLimits::default());
    let value = json(&view.to_json().unwrap());
    assert_eq!(value["ok"], true, "{value}");
    assert_eq!(value["complete"], true);
    assert_eq!(value["sheet"]["kind"], "worksheet");
    assert!(!value["rows"].as_array().unwrap().is_empty());
    assert_eq!(value["shown"], value["used"]);
}

#[test]
fn the_merged_fixture_gives_its_merges_dimension_used_range_and_strings() {
    let path = Path::new(FIXTURES).join("merged.xlsx");
    let view = describe_sheet_path(&path, &asking(None, None, None), &HostLimits::default());
    let value = json(&view.to_json().unwrap());
    assert_eq!(value["ok"], true, "{value}");
    assert_eq!(value["dimension"], "A1:E8");
    assert_eq!(value["used"], "A1:E8");
    assert_eq!(value["shown"], "A1:E8");
    assert_eq!(value["merges"], json!(["A1:C1", "A3:A5"]));
    let text = serde_json::to_string(&value["rows"]).unwrap();
    assert!(text.contains(r#""value":"line one\nline two""#), "{text}");
    assert!(text.contains(r#""value":"a|b""#), "{text}");
    assert!(
        text.contains(r#""type":"date","value":"2024-01-15""#),
        "{text}"
    );
    let narrow = asking(Some("A3:B5"), None, None);
    let view = describe_sheet_path(&path, &narrow, &HostLimits::default());
    let value = json(&view.to_json().unwrap());
    assert_eq!(value["merges"], json!(["A3:A5"]));
}

#[test]
fn the_merged_fixture_as_text() {
    let path = Path::new(FIXTURES).join("merged.xlsx");
    let view = describe_sheet_path(&path, &asking(None, None, None), &HostLimits::default());
    let expected = concat!(
        "Sheet1 (index 0, worksheet, visible) — used A1:E8, dimension A1:E8, shown A1:E8\n",
        "|  | A | B | C | D | E |\n",
        "| --- | --- | --- | --- | --- | --- |\n",
        "| 1 | Quarterly report |  |  |  |  |\n",
        "| 2 | Group | Item | Amount |  | after the gap |\n",
        "| 3 | Fruit | apple | 1.5 |  |  |\n",
        "| 4 |  | line one\\nline two | 2 |  |  |\n",
        "| 5 |  | a\\|b | 3 |  |  |\n",
        "| 6 |  | date | 2024-01-15 |  |  |\n",
        "| 7 |  |  |  |  |  |\n",
        "| 8 | Total |  | 6.5 |  |  |\n",
        "Merged: A1:C1, A3:A5\n",
        "16 cells, 0 errors, 0 warnings, 0 notices — ok\n",
    );
    assert_eq!(crate::sheet::GridText(&view).to_string(), expected);
}

//! The defined names: a name a formula cannot name, a `localSheetId` past
//! the last sheet, two of one scope the same, and a reference to a sheet
//! the workbook does not have. Each is reported at the workbook part.

use super::{book, check, name, workbook};
use xlsx_graft_diagnostics::{Diagnostic, Echo, Location};

/// What checking the sound workbook with `names` defined after its own
/// reports.
fn with(names: &[(&str, Option<u32>, &str)]) -> Vec<(Diagnostic, Location)> {
    let mut book = book();
    for &(text, scope, formula) in names {
        book.defined_names.push(name(text, scope, formula));
    }
    check(&book)
}

/// `defined_name_invalid` for `name`, at the workbook part.
fn invalid(name: &str) -> (Diagnostic, Location) {
    let name = Echo::new(name);
    (Diagnostic::DefinedNameInvalid { name }, workbook())
}

/// `defined_name_sheet_missing` for `name` and `sheet`, at the workbook
/// part.
fn missing(name: &str, sheet: &str) -> (Diagnostic, Location) {
    let diagnostic = Diagnostic::DefinedNameSheetMissing {
        name: Echo::new(name),
        sheet: Echo::new(sheet),
    };
    (diagnostic, workbook())
}

#[test]
fn a_name_a_formula_cannot_name_is_refused() {
    // Excel for Mac 16.114.3 shows its repair dialog for each, and drops
    // the defined name.
    for text in [
        "Bad Name",
        "1Bad",
        "",
        "a-b",
        ".ab",
        "?ab",
        "a\u{3000}b",
        "１a",
    ] {
        assert_eq!(with(&[(text, None, "1")]), [invalid(text)], "{text}");
    }
}

#[test]
fn a_name_of_letters_digits_and_the_marks_excel_takes_is_taken() {
    // Excel opens `A1`, `\a`, `a\b`, `a?b`, `名前`, `売上・前年` and `a１`
    // as they are: `A1` has the form of a cell reference, MS-OI29500 says
    // Office does not load `\a`, and the grammar does not list `?`.
    for text in [
        "A1",
        "_xlnm.Print_Area",
        "\\a",
        "\\name",
        "a\\b",
        "a?b",
        "名前",
        "売上・前年",
        "a１",
        "Name.2",
    ] {
        assert_eq!(with(&[(text, None, "1")]), [], "{text}");
    }
}

#[test]
fn a_scope_past_the_last_sheet_is_refused() {
    // Excel shows its repair dialog for `localSheetId="99"` with 8 sheets,
    // and drops the defined name. The sound workbook has 3.
    let past = |local_sheet_id| Diagnostic::DefinedNameSheetIndexInvalid {
        name: Echo::new("Local"),
        local_sheet_id,
        sheets: 3,
    };
    assert_eq!(with(&[("Local", Some(2), "1")]), []);
    assert_eq!(with(&[("Local", Some(3), "1")]), [(past(3), workbook())]);
    assert_eq!(
        with(&[("Local", Some(u32::MAX), "1")]),
        [(past(u64::from(u32::MAX)), workbook())]
    );
}

#[test]
fn every_name_of_a_scope_before_the_last_with_its_name_is_noticed() {
    // Excel opens `BASE_NAME` then `Base_Name` without a word and keeps
    // only the later one.
    let duplicate = |name: &str, other: &str| {
        let diagnostic = Diagnostic::DefinedNameDuplicate {
            name: Echo::new(name),
            other: Echo::new(other),
        };
        (diagnostic, workbook())
    };
    assert_eq!(
        with(&[("BASE_NAME", None, "1"), ("base_name", None, "2")]),
        [
            duplicate("Base_Name", "base_name"),
            duplicate("BASE_NAME", "base_name"),
        ]
    );
    assert_eq!(
        with(&[("Local", Some(1), "1"), ("LOCAL", Some(1), "2")]),
        [duplicate("Local", "LOCAL")]
    );
}

#[test]
fn names_of_other_scopes_or_already_refused_are_no_duplicates() {
    // Excel opens a workbook's `Base_Name` beside a sheet's as it is.
    assert_eq!(
        with(&[("Base_Name", Some(2), "1"), ("Base_Name", Some(1), "1")]),
        []
    );
    let past = Diagnostic::DefinedNameSheetIndexInvalid {
        name: Echo::new("Local"),
        local_sheet_id: 7,
        sheets: 3,
    };
    assert_eq!(
        with(&[
            ("Local", Some(7), "1"),
            ("Local", Some(7), "2"),
            ("Bad Name", None, "1"),
            ("Bad Name", None, "2"),
        ]),
        [
            (past.clone(), workbook()),
            (past, workbook()),
            invalid("Bad Name"),
            invalid("Bad Name"),
        ]
    );
}

#[test]
fn a_reference_to_a_sheet_the_workbook_lacks_is_noticed() {
    // Excel opens `NoSuch!$A$1` without a word and reads it as
    // `[1]NoSuch!$A$1`, a sheet of another workbook.
    assert_eq!(
        with(&[("Gone", None, "NoSuch!$A$1")]),
        [missing("Gone", "NoSuch")]
    );
    assert_eq!(
        with(&[("Gone", None, "SUM('No Such'!A1:B2,NoSuch!A1,nosuch!B:B)")]),
        [missing("Gone", "No Such"), missing("Gone", "NoSuch")]
    );
    assert_eq!(
        with(&[("Gone", None, "SUM(データ:Away!A1,Away!Other)")]),
        [missing("Gone", "Away")]
    );
    assert_eq!(
        with(&[("Gone", None, "SUM('No A:Extra'!A1,'O''Brien'!A1)")]),
        [missing("Gone", "No A"), missing("Gone", "O'Brien")]
    );
}

#[test]
fn a_reference_to_a_sheet_the_workbook_has_or_to_another_workbook_is_taken() {
    assert_eq!(
        with(&[
            (
                "Here",
                None,
                "SUM(extra!A1,'集計'!$B$2,データ:Extra!1:2,Extra!Base_Name)"
            ),
            ("Away", None, "[1]NoSuch!$A$1"),
            ("Path", None, "'C:\\d\\[b.xlsx]NoSuch'!A1"),
            ("Bare", None, "SUM(A1,Base_Name,#REF!)"),
        ]),
        []
    );
}

#[test]
fn a_reference_to_a_sheet_whose_name_holds_a_colon_is_not_parted() {
    // The sheet `a:b` is `sheet_name_invalid` already.
    let mut book = book();
    book.sheets[2].name = "a:b".to_owned();
    book.defined_names.push(name("Odd", None, "'a:b'!A1"));
    let codes: Vec<&str> = (check(&book).iter())
        .map(|(diagnostic, _)| diagnostic.code().as_str())
        .collect();
    assert_eq!(codes, ["sheet_name_invalid"]);
}

#[test]
fn a_formula_the_reader_cannot_read_whole_is_not_looked_into() {
    assert_eq!(with(&[("Open", None, "'NoSuch!A1")]), []);
}

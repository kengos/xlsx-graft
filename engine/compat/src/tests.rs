//! Unit tests for the crate's entry points: looking a function up, and the
//! cells research-compat's examples give, one function at a time.

use super::{
    Added, Danger, Dangerous, Excel, ExcelMac, Fallback, Feature, GoogleSheets, LibreOffice,
    Numbers, OnlyOffice, Row, Safety, Status, function, function_safety, functions, safeties,
};

fn row(name: &str) -> &'static Row {
    function(name).unwrap()
}

#[test]
fn a_function_is_looked_up_by_its_name_in_any_case() {
    assert_eq!(row("xlookup").name, "XLOOKUP");
    assert_eq!(row("Stdev.S").name, "STDEV.S");
    assert!(functions().iter().any(|row| row.name == "FILTER"));
}

#[test]
fn a_function_excel_2007_had_has_no_row() {
    assert_eq!(function("SUM"), None);
    assert_eq!(function("IFERROR"), None);
}

#[test]
fn the_safety_of_a_function_is_looked_up_by_its_name_in_any_case() {
    // SUM has no row of compatibility, but has a safety.
    assert!(matches!(
        function_safety("sum"),
        Some(Safety::ReviewedSafe(_))
    ));
    assert!(matches!(
        function_safety("XLOOKUP"),
        Some(Safety::ReviewedSafe(_))
    ));
    assert_eq!(function_safety("filterxml"), Some(Safety::Unreviewed));
    assert!(matches!(
        function_safety("WebService"),
        Some(Safety::Dangerous(Dangerous {
            danger: Danger::Network,
            ..
        }))
    ));
    assert!(safeties().iter().any(|row| row.name == "REGISTER.ID"));
    // The unchecked functions: 353 of ECMA-376, and ANCHORARRAY, SINGLE,
    // CALL, COPILOT and REGISTER.ID.
    assert_eq!(safeties().len(), functions().len() + 353 + 5);
}

#[test]
fn a_name_the_table_does_not_list_has_no_safety() {
    assert_eq!(function_safety("_xlfn.XLOOKUP"), None);
    assert_eq!(function_safety("EVALUATE"), None);
    assert_eq!(function_safety(""), None);
}

#[test]
fn a_prefixed_name_has_no_row() {
    assert_eq!(function("_xlfn.XLOOKUP"), None);
    assert_eq!(function("_xlfn._xlws.FILTER"), None);
}

#[test]
fn an_empty_name_has_no_row() {
    assert_eq!(function(""), None);
}

#[test]
fn xlookup_came_in_excel_2021_and_libreoffice_24_8() {
    let xlookup = row("XLOOKUP").support;
    assert_eq!(xlookup.excel.added, Added::In(Excel::V2021));
    assert_eq!(xlookup.excel.fallback, Some(Fallback::NameError));
    assert_eq!(xlookup.excel.at(Excel::V2019), Status::Unsupported);
    assert_eq!(xlookup.excel.at(Excel::V2021), Status::Supported);
    assert_eq!(xlookup.excel_mac.added, Added::In(ExcelMac::V2021));
    assert_eq!(xlookup.libreoffice.added, Added::In(LibreOffice::V24_8));
    assert_eq!(
        xlookup.libreoffice.at(LibreOffice::V24_2),
        Status::Unsupported
    );
    assert_eq!(
        xlookup.google_sheets.at(GoogleSheets::Current),
        Status::Supported
    );
    // Numbers for iCloud had it in 2020; which Mac version first did is
    // not known.
    assert_eq!(xlookup.numbers.added, Added::AtMost(Numbers::V14_4));
    assert_eq!(xlookup.numbers.at(Numbers::V14_0), Status::Unverified);
    assert!(xlookup.numbers.notes.is_some());
    assert_eq!(xlookup.onlyoffice.at(OnlyOffice::V8_0), Status::Supported);
}

#[test]
fn filter_needs_dynamic_arrays_too() {
    let filter = row("FILTER");
    assert_eq!(filter.requires, [Feature::DynamicArray]);
    assert_eq!(filter.support.excel.added, Added::In(Excel::V2021));
    assert_eq!(
        filter.support.libreoffice.added,
        Added::In(LibreOffice::V24_8)
    );
    assert_eq!(filter.support.numbers.added, Added::In(Numbers::V14_4));
    assert_eq!(
        filter.support.numbers.at(Numbers::V14_3),
        Status::Unsupported
    );
    let dynamic = Feature::DynamicArray.row().support;
    assert_eq!(dynamic.excel.fallback, Some(Fallback::LegacyArray));
    assert_eq!(dynamic.excel.at(Excel::V2019), Status::Unsupported);
    assert_eq!(
        dynamic.libreoffice.at(LibreOffice::V26_2),
        Status::Unverified
    );
    assert_eq!(dynamic.onlyoffice.added, Added::In(OnlyOffice::V9_3));
}

#[test]
fn switch_is_taken_as_missing_from_excel_2016() {
    let switch = row("SWITCH").support;
    assert_eq!(switch.excel.added, Added::In(Excel::V2019));
    assert_eq!(switch.excel.at(Excel::V2016), Status::Unsupported);
    assert!(switch.excel.notes.is_some());
    assert_eq!(switch.libreoffice.at(LibreOffice::V24_2), Status::Supported);
    assert_eq!(switch.numbers.added, Added::AtMost(Numbers::V14_4));
    assert!(
        row("SWITCH")
            .sources
            .iter()
            .any(|source| source.contains("switch-function"))
    );
}

#[test]
fn lambda_is_missing_from_libreoffice() {
    let lambda = row("LAMBDA").support;
    assert_eq!(lambda.excel.added, Added::In(Excel::V2024));
    assert_eq!(lambda.excel.at(Excel::V2021), Status::Unsupported);
    assert_eq!(lambda.libreoffice.added, Added::Never);
    assert!(
        lambda
            .libreoffice
            .notes
            .is_some_and(|notes| notes.contains("Err:508"))
    );
    assert_eq!(lambda.numbers.added, Added::In(Numbers::V14_4));
    assert_eq!(lambda.onlyoffice.added, Added::Unknown);
    assert_eq!(lambda.onlyoffice.at(OnlyOffice::V10_0), Status::Unverified);
}

#[test]
fn vstack_came_in_libreoffice_25_8() {
    let vstack = row("VSTACK");
    assert_eq!(vstack.requires, [Feature::DynamicArray]);
    assert_eq!(vstack.support.excel.added, Added::In(Excel::V2024));
    assert_eq!(
        vstack.support.libreoffice.at(LibreOffice::V25_2),
        Status::Unsupported
    );
    assert_eq!(
        vstack.support.libreoffice.at(LibreOffice::V25_8),
        Status::Supported
    );
    assert_eq!(vstack.support.numbers.added, Added::Unknown);
}

#[test]
fn groupby_is_only_in_microsoft_365() {
    let groupby = row("GROUPBY").support;
    assert_eq!(groupby.excel.at(Excel::V2024), Status::Unsupported);
    assert_eq!(groupby.excel.at(Excel::V365), Status::Supported);
    assert_eq!(groupby.libreoffice.added, Added::Never);
    assert_eq!(groupby.google_sheets.added, Added::Unknown);
}

#[test]
fn regexextract_differs_in_google_sheets() {
    let regex = row("REGEXEXTRACT").support;
    assert_eq!(regex.excel.added, Added::In(Excel::V365));
    assert_eq!(regex.libreoffice.added, Added::Never);
    assert!(regex.google_sheets.partial);
    assert_eq!(
        regex.google_sheets.at(GoogleSheets::Current),
        Status::Supported
    );
    assert_eq!(regex.onlyoffice.at(OnlyOffice::V9_2), Status::Unsupported);
    assert_eq!(regex.onlyoffice.at(OnlyOffice::V9_3), Status::Supported);
}

#[test]
fn excel_for_mac_lacks_the_functions_that_rely_on_windows() {
    for name in ["ENCODEURL", "FILTERXML", "WEBSERVICE"] {
        assert_eq!(row(name).support.excel_mac.added, Added::Never, "{name}");
        assert_eq!(
            row(name).support.excel.added,
            Added::In(Excel::V2013),
            "{name}"
        );
    }
}

#[test]
fn a_threaded_comment_is_a_note_in_ltsc_2021_and_unverified_in_2024() {
    let threaded = Feature::ThreadedComment.row().support;
    assert_eq!(threaded.excel.at(Excel::V2021), Status::Unsupported);
    assert_eq!(threaded.excel.at(Excel::V2024), Status::Unverified);
    assert_eq!(threaded.excel.at(Excel::V365), Status::Supported);
    assert_eq!(threaded.excel.fallback, Some(Fallback::AsNote));
}

#[test]
fn excel_2007_lacks_the_functions_the_list_gives_no_version_for() {
    let aggregate = row("AGGREGATE").support.excel;
    assert_eq!(aggregate.at(Excel::V2007), Status::Unsupported);
    assert_eq!(aggregate.at(Excel::V2013), Status::Unverified);
    assert_eq!(aggregate.at(Excel::V2016), Status::Supported);
    let arabic = row("ARABIC").support.excel;
    assert_eq!(arabic.at(Excel::V2007), Status::Unsupported);
    assert_eq!(arabic.at(Excel::V2024), Status::Unverified);
    assert_eq!(arabic.at(Excel::V365), Status::Supported);
}

//! Unit tests for the applications and their version lists.

use super::{App, Excel, ExcelMac, GoogleSheets, LibreOffice, Numbers, OnlyOffice, Version};

/// The versions of `V`, checked to be in order and named as `NAMES` says.
fn names_in_order<V: Version>() -> Vec<&'static str> {
    assert!(
        V::ALL.windows(2).all(|pair| pair[0] < pair[1]),
        "{:?} out of order",
        V::APP
    );
    assert_eq!(V::ALL.len(), V::NAMES.len());
    assert_eq!(V::APP.version_names(), V::NAMES);
    V::ALL.iter().map(|version| version.as_str()).collect()
}

#[test]
fn each_application_lists_its_versions_oldest_first() {
    assert_eq!(
        names_in_order::<Excel>(),
        [
            "2007", "2010", "2013", "2016", "2019", "2021", "2024", "365"
        ]
    );
    assert_eq!(
        names_in_order::<ExcelMac>(),
        ["2016", "2019", "2021", "2024", "365"]
    );
    assert_eq!(
        names_in_order::<LibreOffice>(),
        ["24.2", "24.8", "25.2", "25.8", "26.2", "26.8"]
    );
    assert_eq!(names_in_order::<GoogleSheets>(), ["current"]);
    assert_eq!(
        names_in_order::<Numbers>(),
        [
            "11.0", "11.1", "11.2", "12.0", "12.1", "12.2", "13.0", "13.1", "13.2", "14.0", "14.1",
            "14.2", "14.3", "14.4"
        ]
    );
    assert_eq!(
        names_in_order::<OnlyOffice>(),
        [
            "8.0", "8.1", "8.2", "8.3", "9.0", "9.1", "9.2", "9.3", "9.4", "10.0"
        ]
    );
}

#[test]
fn each_version_type_belongs_to_its_application() {
    assert_eq!(
        [
            Excel::APP,
            ExcelMac::APP,
            LibreOffice::APP,
            GoogleSheets::APP,
            Numbers::APP,
            OnlyOffice::APP
        ],
        App::ALL
    );
}

#[test]
fn applications_are_named_as_targets_writes_them() {
    let names: Vec<_> = App::ALL.iter().map(|app| app.as_str()).collect();
    assert_eq!(
        names,
        [
            "excel",
            "excel-mac",
            "libreoffice",
            "google-sheets",
            "numbers",
            "onlyoffice"
        ]
    );
}

#[test]
fn excel_365_is_newer_than_every_perpetual_version() {
    assert!(Excel::V2024 < Excel::V365);
    assert!(ExcelMac::V2024 < ExcelMac::V365);
}

#[test]
fn versions_compare_as_numbers_not_as_text() {
    assert!(OnlyOffice::V9_4 < OnlyOffice::V10_0);
}

#[test]
fn only_google_sheets_has_no_versions() {
    let versionless: Vec<_> = App::ALL.iter().filter(|app| !app.has_versions()).collect();
    assert_eq!(versionless, [&App::GoogleSheets]);
}

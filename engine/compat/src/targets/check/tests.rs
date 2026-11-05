//! Unit tests for checking a row against the targets: the versions that
//! lack it, those not confirmed, the features it needs, and the
//! diagnostic each [`Unmet`] makes.

use super::Unmet;
use super::wire::{app, fallback};
use crate::entry::Entry;
use crate::row::{Row, Support};
use crate::{
    App, Excel, ExcelMac, Fallback, Feature, GoogleSheets, LibreOffice, Numbers, OnlyOffice,
};
use crate::{Targets, function};
use xlsx_graft_diagnostics::{
    Diagnostic, Diagnostics, Echo, TargetApp, TargetFallback, TargetList,
};

/// The targets of the two lists, which hold no problem.
fn targets(targets: &[&str], best_effort: &[&str]) -> Targets {
    let mut diagnostics = Diagnostics::new();
    let read = Targets::parse(targets, best_effort, &mut diagnostics).unwrap();
    assert!(diagnostics.finish().is_empty());
    read
}

/// What `targets` say of the function `name`.
fn check(targets: &Targets, name: &str) -> Vec<Unmet> {
    targets.check(function(name).unwrap())
}

/// An [`Unmet`] of `name` with no feature to blame and no fallback.
fn unmet(name: &'static str, list: TargetList, app: App) -> Unmet {
    Unmet {
        name,
        list,
        app,
        unsupported: Vec::new(),
        unverified: Vec::new(),
        requires: Vec::new(),
        fallback: None,
    }
}

#[test]
fn no_targets_check_the_default() {
    assert_eq!(check(&Targets::default(), "XLOOKUP"), []);
    let unmet: Vec<(TargetList, App)> = check(&Targets::default(), "WEBSERVICE")
        .iter()
        .map(|unmet| (unmet.list, unmet.app))
        .collect();
    assert_eq!(unmet, [(TargetList::Targets, App::ExcelMac)]);
}

#[test]
fn every_version_the_targets_hold_is_checked() {
    assert_eq!(check(&targets(&["excel >= 2021"], &[]), "XLOOKUP"), []);
    assert_eq!(
        check(&targets(&["excel >= 2016"], &[]), "XLOOKUP"),
        [Unmet {
            unsupported: vec!["2016", "2019"],
            fallback: Some(Fallback::NameError),
            ..unmet("XLOOKUP", TargetList::Targets, App::Excel)
        }]
    );
}

#[test]
fn a_version_no_source_settles_is_unverified() {
    assert_eq!(
        check(&targets(&[], &["numbers >= 14.2"]), "XLOOKUP"),
        [Unmet {
            unverified: vec!["14.2", "14.3"],
            ..unmet("XLOOKUP", TargetList::BestEffort, App::Numbers)
        }]
    );
    assert_eq!(
        check(&targets(&["google-sheets"], &[]), "FORECAST.ETS"),
        [Unmet {
            unverified: vec!["current"],
            ..unmet("FORECAST.ETS", TargetList::Targets, App::GoogleSheets)
        }]
    );
}

#[test]
fn a_feature_the_function_needs_counts_where_the_function_is_there() {
    let both = targets(
        &["libreoffice >= 24.2", "excel 365"],
        &["onlyoffice <= 9.3", "numbers 14.4"],
    );
    assert_eq!(
        check(&both, "FILTER"),
        [
            Unmet {
                unsupported: vec!["24.2"],
                unverified: vec!["24.8", "25.2", "25.8", "26.2", "26.8"],
                requires: vec![Feature::DynamicArray],
                ..unmet("FILTER", TargetList::Targets, App::LibreOffice)
            },
            Unmet {
                unsupported: vec!["8.0", "8.1", "8.2", "8.3", "9.0", "9.1", "9.2"],
                requires: vec![Feature::DynamicArray],
                ..unmet("FILTER", TargetList::BestEffort, App::OnlyOffice)
            },
        ]
    );
}

#[test]
fn a_feature_gives_its_fallback_when_the_function_itself_is_everywhere() {
    static ROW: Row = Row {
        name: "ANYWHERE",
        support: Support::new(
            Entry::added(Excel::V2007),
            Entry::added(ExcelMac::V2016),
            Entry::added(LibreOffice::V24_2),
            Entry::added(GoogleSheets::Current),
            Entry::added(Numbers::V11_0),
            Entry::added(OnlyOffice::V8_0),
        ),
        requires: &[Feature::DynamicArray, Feature::DynamicArray],
        sources: &["https://example.com"],
    };
    assert_eq!(
        targets(&["excel >= 2016"], &[]).check(&ROW),
        [Unmet {
            unsupported: vec!["2016", "2019"],
            requires: vec![Feature::DynamicArray],
            fallback: Some(Fallback::LegacyArray),
            ..unmet("ANYWHERE", TargetList::Targets, App::Excel)
        }]
    );
}

#[test]
fn each_list_makes_its_own_code() {
    let found = Unmet {
        unsupported: vec!["2016", "2019"],
        unverified: vec!["2013"],
        requires: vec![Feature::DynamicArray],
        fallback: Some(Fallback::NameError),
        ..unmet("XLOOKUP", TargetList::Targets, App::Excel)
    };
    assert_eq!(
        found.function_diagnostic(3),
        Diagnostic::TargetFunctionUnsupported {
            name: Echo::new("XLOOKUP"),
            app: TargetApp::Excel,
            unsupported: Echo::new("2016, 2019"),
            unverified: Echo::new("2013"),
            requires: Echo::new("dynamic_array"),
            fallback: TargetFallback::NameError,
            count: 3,
        }
    );
    let found = unmet("FILTER", TargetList::BestEffort, App::LibreOffice);
    assert_eq!(
        found.function_diagnostic(1),
        Diagnostic::BestEffortFunctionUnsupported {
            name: Echo::new("FILTER"),
            app: TargetApp::LibreOffice,
            unsupported: Echo::new(""),
            unverified: Echo::new(""),
            requires: Echo::new(""),
            fallback: TargetFallback::Unknown,
            count: 1,
        }
    );
}

#[test]
fn every_fallback_keeps_its_wire_string_in_the_diagnostic() {
    for &table in Fallback::ALL {
        assert_eq!(fallback(Some(table)).to_string(), table.as_str());
    }
    assert_eq!(fallback(None), TargetFallback::Unknown);
}

#[test]
fn every_application_keeps_its_name_in_the_diagnostic() {
    for &table in App::ALL {
        assert_eq!(app(table).to_string(), table.as_str().replace('-', "_"));
    }
    assert_eq!(app(App::ExcelMac), TargetApp::ExcelMac);
}

#[test]
fn a_function_not_there_itself_gives_its_own_fallback() {
    static ROW: Row = Row {
        name: "UNDATED",
        support: Support::new(
            Entry::between(Excel::V2007, Excel::V2016),
            Entry::added(ExcelMac::V2016),
            Entry::added(LibreOffice::V24_2),
            Entry::added(GoogleSheets::Current),
            Entry::added(Numbers::V11_0),
            Entry::added(OnlyOffice::V8_0),
        ),
        requires: &[Feature::DynamicArray],
        sources: &["https://example.com"],
    };
    assert_eq!(
        targets(&["excel >= 2010"], &[]).check(&ROW),
        [Unmet {
            unsupported: vec!["2010", "2013", "2016", "2019"],
            requires: vec![Feature::DynamicArray],
            ..unmet("UNDATED", TargetList::Targets, App::Excel)
        }]
    );
}

#[test]
fn a_feature_missing_where_the_function_is_not_confirmed_is_missing() {
    assert_eq!(
        check(&targets(&["numbers >= 11.0"], &[]), "RANDARRAY"),
        [Unmet {
            unsupported: vec![
                "11.0", "11.1", "11.2", "12.0", "12.1", "12.2", "13.0", "13.1", "13.2", "14.0",
                "14.1", "14.2", "14.3",
            ],
            unverified: vec!["14.4"],
            requires: vec![Feature::DynamicArray],
            ..unmet("RANDARRAY", TargetList::Targets, App::Numbers)
        }]
    );
}

#[test]
fn a_feature_found_on_its_own_is_left_out_of_what_a_function_needs() {
    let both = targets(&["libreoffice >= 24.8"], &["onlyoffice <= 9.3"]);
    let filter = function("FILTER").unwrap();
    assert_eq!(both.check_besides(filter, &[Feature::DynamicArray]), []);
    assert_eq!(
        both.check_besides(filter, &[Feature::Sparkline]),
        both.check(filter)
    );
    let old = targets(&["libreoffice 24.2"], &[]);
    assert_eq!(
        old.check_besides(filter, &[Feature::DynamicArray]),
        [Unmet {
            unsupported: vec!["24.2"],
            ..unmet("FILTER", TargetList::Targets, App::LibreOffice)
        }]
    );
}

#[test]
fn a_feature_row_is_checked_as_a_function_row_is() {
    assert_eq!(
        targets(&["excel >= 2019"], &["numbers 14.4"]).check(Feature::ThreadedComment.row()),
        [
            Unmet {
                unsupported: vec!["2019", "2021"],
                unverified: vec!["2024"],
                fallback: Some(Fallback::AsNote),
                ..unmet("threaded_comment", TargetList::Targets, App::Excel)
            },
            Unmet {
                unverified: vec!["14.4"],
                ..unmet("threaded_comment", TargetList::BestEffort, App::Numbers)
            },
        ]
    );
}

#[test]
fn each_list_makes_its_own_feature_code() {
    let found = Unmet {
        unsupported: vec!["2019"],
        unverified: vec!["2024"],
        fallback: Some(Fallback::AsNote),
        ..unmet("threaded_comment", TargetList::Targets, App::Excel)
    };
    assert_eq!(
        found.feature_diagnostic(2),
        Diagnostic::TargetFeatureUnsupported {
            name: Echo::new("threaded_comment"),
            app: TargetApp::Excel,
            unsupported: Echo::new("2019"),
            unverified: Echo::new("2024"),
            requires: Echo::new(""),
            fallback: TargetFallback::AsNote,
            count: 2,
        }
    );
    let found = Unmet {
        unverified: vec!["24.8"],
        ..unmet("dynamic_array", TargetList::BestEffort, App::LibreOffice)
    };
    assert_eq!(
        found.feature_diagnostic(1),
        Diagnostic::BestEffortFeatureUnsupported {
            name: Echo::new("dynamic_array"),
            app: TargetApp::LibreOffice,
            unsupported: Echo::new(""),
            unverified: Echo::new("24.8"),
            requires: Echo::new(""),
            fallback: TargetFallback::Unknown,
            count: 1,
        }
    );
}

//! A function the table schedules a later version of xlsx-graft to deny:
//! accepted with a notice, or denied as dangerous when the policy denies
//! the scheduled functions now.

use super::{at, judge_by};
use crate::tests::excel_365;
use crate::{FunctionClass, FunctionTable, Names, Place, Rules};
use xlsx_graft_diagnostics::{Diagnostic, Echo, Location};

/// A table that schedules `ENCODEURL` and `OFFSET` to be denied from
/// xlsx-graft 0.9.0, both for the made-up reason "it builds a URL", marks
/// `SUM` reviewed safe, and knows nothing else.
struct Scheduling;

impl FunctionTable for Scheduling {
    fn classify(&self, name: &str) -> Option<FunctionClass<'_>> {
        match name {
            "ENCODEURL" | "OFFSET" => Some(FunctionClass::Scheduled {
                from: "0.9.0",
                reason: "it builds a URL",
            }),
            "SUM" => Some(FunctionClass::ReviewedSafe),
            _ => None,
        }
    }
}

/// What judging `formula` in A1 of the first sheet by `rules` and the
/// [`Scheduling`] table reports.
fn scheduled(rules: &Rules, formula: &str) -> Vec<(Diagnostic, Location)> {
    let place = Place {
        part: super::SHEET1,
        at: "A1",
        sheet: Some(0),
    };
    let names = Names::default();
    judge_by(
        rules,
        &Scheduling,
        &excel_365(),
        &names,
        &[(place, formula)],
    )
}

/// The notice for `name`, scheduled to be denied from 0.9.0, found
/// `count` times.
fn notice(name: &str, count: u64) -> Diagnostic {
    Diagnostic::FunctionDenyScheduled {
        name: Echo::new(name),
        from: Echo::new("0.9.0"),
        reason: Echo::new("it builds a URL"),
        count,
    }
}

#[test]
fn a_scheduled_function_is_accepted_with_one_notice_that_counts_each_call() {
    let formula = "ENCODEURL(\"a\")&ENCODEURL(\"b\")&SUM(1)";
    assert_eq!(
        scheduled(&Rules::default(), formula),
        [(notice("ENCODEURL", 2), at("A1"))]
    );
}

#[test]
fn a_scheduled_function_the_template_allows_gets_no_notice() {
    let rules = Rules::default().allow_function("encodeurl");
    assert_eq!(scheduled(&rules, "ENCODEURL(\"a\")"), []);
}

#[test]
fn the_policy_denies_a_scheduled_function_now_as_dangerous_unless_the_template_allows_it() {
    let rules = Rules::default().deny_scheduled();
    let dangerous = Diagnostic::FunctionDangerous {
        name: Echo::new("ENCODEURL"),
        reason: Echo::new(
            "it builds a URL; xlsx-graft 0.9.0 is to deny it, and the host's policy denies it now",
        ),
        count: 1,
    };
    assert_eq!(
        scheduled(&rules, "ENCODEURL(\"a\")"),
        [(dangerous, at("A1"))]
    );
    let rules = rules.allow_function("ENCODEURL");
    assert_eq!(scheduled(&rules, "ENCODEURL(\"a\")"), []);
}

#[test]
fn a_scheduled_function_that_returns_an_unchecked_reference_gets_both_notices() {
    let unchecked = Diagnostic::FunctionReferenceUnchecked {
        name: Echo::new("OFFSET"),
        count: 1,
    };
    assert_eq!(
        scheduled(&Rules::default(), "SUM(OFFSET(B1,1,1))"),
        [(notice("OFFSET", 1), at("A1")), (unchecked, at("A1"))]
    );
}

#[test]
fn the_policys_forbid_wins_over_denying_the_scheduled_functions_now() {
    let rules = Rules::default()
        .deny_scheduled()
        .forbid_function("ENCODEURL", "no URLs");
    let forbidden = Diagnostic::FunctionForbidden {
        name: Echo::new("ENCODEURL"),
        reason: Echo::new("no URLs"),
        count: 1,
    };
    assert_eq!(
        scheduled(&rules, "ENCODEURL(\"a\")"),
        [(forbidden, at("A1"))]
    );
}

#[test]
fn a_scheduled_offset_the_template_allows_keeps_its_unchecked_notice_when_the_policy_denies_now() {
    let rules = Rules::default().deny_scheduled().allow_function("OFFSET");
    let unchecked = Diagnostic::FunctionReferenceUnchecked {
        name: Echo::new("OFFSET"),
        count: 1,
    };
    assert_eq!(
        scheduled(&rules, "SUM(OFFSET(B1,1,1))"),
        [(unchecked, at("A1"))]
    );
}

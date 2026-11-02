//! Unit tests for `Rules`: the template allows, the policy forbids, and
//! the policy wins.

use super::{Rules, Ruling};
use xlsx_graft_diagnostics::PartKind;

#[test]
fn nothing_is_ruled_by_default() {
    let rules = Rules::default();
    assert_eq!(rules.part(PartKind::Macro), Ruling::Unruled);
    assert_eq!(rules.function("SUM"), Ruling::Unruled);
}

#[test]
fn the_template_allows_a_part_kind_and_a_function_in_any_letter_case() {
    let rules = Rules::default()
        .allow_part(PartKind::Ole)
        .allow_function("WebService");
    assert_eq!(rules.part(PartKind::Ole), Ruling::Allowed);
    assert_eq!(rules.part(PartKind::Activex), Ruling::Unruled);
    assert_eq!(rules.function("WEBSERVICE"), Ruling::Allowed);
    assert_eq!(rules.function("webservice"), Ruling::Allowed);
    assert_eq!(rules.function("RTD"), Ruling::Unruled);
}

#[test]
fn the_policy_forbids_over_what_the_template_allows() {
    let rules = Rules::default()
        .allow_part(PartKind::Macro)
        .forbid_part(PartKind::Macro, "no macros")
        .allow_function("rtd")
        .forbid_function("RTD", "no servers");
    assert_eq!(
        rules.part(PartKind::Macro),
        Ruling::Forbidden {
            reason: "no macros"
        }
    );
    assert_eq!(
        rules.function("Rtd"),
        Ruling::Forbidden {
            reason: "no servers"
        }
    );
}

#[test]
fn a_later_forbid_replaces_the_reason_of_an_earlier_one() {
    let rules = Rules::default()
        .forbid_part(PartKind::Dde, "first")
        .forbid_part(PartKind::Dde, "second")
        .forbid_function("call", "first")
        .forbid_function("CALL", "second");
    assert_eq!(
        rules.part(PartKind::Dde),
        Ruling::Forbidden { reason: "second" }
    );
    assert_eq!(
        rules.function("call"),
        Ruling::Forbidden { reason: "second" }
    );
}

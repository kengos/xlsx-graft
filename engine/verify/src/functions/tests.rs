//! Tests for [`DefaultFunctions`]: the classes it gives the rows of the
//! compatibility table's safety column, and the reasons it writes.

use super::{Class, DefaultFunctions, FunctionClass, FunctionTable, dangerous_reason};
use xlsx_graft_compat::{Danger, Safety, Scheduled};

#[test]
fn the_default_table_classifies_every_row_of_the_safety_column_by_its_safety() {
    for row in xlsx_graft_compat::safeties() {
        let class = DefaultFunctions.classify(row.name);
        match row.safety {
            Safety::ReviewedSafe(safe) => {
                // Class::reviewed is tested on its own below; this arm
                // checks that the look-up finds the class of each row.
                let expected = Class::reviewed(safe.scheduled);
                assert_eq!(class, Some(expected.as_function_class()), "{}", row.name);
            }
            Safety::Dangerous(found) => {
                let reason = dangerous_reason(found.danger, found.denied_since, found.advisory);
                let reason = reason.as_str();
                assert_eq!(
                    class,
                    Some(FunctionClass::Dangerous { reason }),
                    "{}",
                    row.name
                );
            }
            _ => assert_eq!(class, Some(FunctionClass::Unreviewed), "{}", row.name),
        }
    }
}

#[test]
fn the_default_table_ignores_letter_case_and_knows_no_prefixed_user_defined_or_unlisted_name() {
    assert_eq!(
        DefaultFunctions.classify("Sum"),
        Some(FunctionClass::ReviewedSafe)
    );
    assert_eq!(DefaultFunctions.classify("_XLFN.XLOOKUP"), None);
    assert_eq!(DefaultFunctions.classify("_XLUDF.SUM"), None);
    assert_eq!(DefaultFunctions.classify("MYLAMBDA"), None);
}

#[test]
fn a_dangerous_functions_reason_is_the_english_of_its_kind() {
    assert_eq!(
        DefaultFunctions.classify("WEBSERVICE"),
        Some(FunctionClass::Dangerous {
            reason: "it sends data to, or reads data from, a service on the network"
        })
    );
    assert_eq!(
        DefaultFunctions.classify("CALL"),
        Some(FunctionClass::Dangerous {
            reason: "it runs code outside the workbook"
        })
    );
}

#[test]
fn a_dangerous_functions_reason_adds_the_denying_version_and_the_advisory_when_given() {
    assert_eq!(
        dangerous_reason(
            Danger::ExternalCode,
            Some("0.3.1"),
            Some("GHSA-abcd-efgh-ijkl")
        ),
        "it runs code outside the workbook; denied since xlsx-graft 0.3.1; advisory GHSA-abcd-efgh-ijkl"
    );
    assert_eq!(
        dangerous_reason(Danger::OpensLink, Some("0.4.0"), None),
        "clicking its cell opens a link whose target the formula computes; denied since xlsx-graft 0.4.0"
    );
}

#[test]
fn a_reviewed_safe_function_is_scheduled_with_its_dangers_english_or_safe_when_not_scheduled() {
    let class = Class::reviewed(Some(Scheduled::new("0.5.0", Danger::MachineInfo)));
    assert_eq!(
        class.as_function_class(),
        FunctionClass::Scheduled {
            from: "0.5.0",
            reason: "it returns information about the machine that computes it or the path of the workbook's file",
        }
    );
    assert_eq!(Class::reviewed(None), Class::Safe);
}

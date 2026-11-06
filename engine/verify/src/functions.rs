//! [`FunctionTable`]: how the engine classifies a function a formula
//! calls, which the formula check judges it by, and [`DefaultFunctions`],
//! the engine's own table.

use std::collections::HashMap;
use std::sync::OnceLock;
use xlsx_graft_compat::{Danger, Safety, Scheduled};

/// How a [`FunctionTable`] classifies a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionClass<'a> {
    /// Reviewed and found safe: it neither reaches outside the workbook nor
    /// reads the machine it runs on.
    ReviewedSafe,
    /// Reviewed and found safe, but a danger found later makes the version
    /// `from` of xlsx-graft deny it, for `reason`. Until then it is
    /// accepted with a notice, unless
    /// [`Rules::deny_scheduled`](crate::Rules::deny_scheduled) denies it
    /// now.
    Scheduled {
        /// The version of xlsx-graft that is to deny it, as
        /// `MAJOR.MINOR.PATCH`.
        from: &'a str,
        /// Why that version is to deny it, in English.
        reason: &'a str,
    },
    /// Reviewed and found dangerous, for `reason`.
    Dangerous {
        /// Why, in English.
        reason: &'a str,
    },
    /// Known, but not reviewed yet.
    Unreviewed,
}

/// The table of functions the engine knows. A function it does not know
/// is denied, as is one it does not classify
/// [`ReviewedSafe`](FunctionClass::ReviewedSafe) or
/// [`Scheduled`](FunctionClass::Scheduled), unless the template allows it;
/// a `Scheduled` one is denied too when the policy denies the scheduled
/// functions now ([`Rules::deny_scheduled`](crate::Rules::deny_scheduled)).
///
/// The table lists every function, by its name without its prefixes,
/// reviewed or not ([`Unreviewed`](FunctionClass::Unreviewed)): a
/// workbook may define a name or a table of a function's name, and a word
/// naming it is judged as that function only when the rules or the table
/// know one.
pub trait FunctionTable {
    /// How the table classifies the function `name`, upper-cased as the
    /// formula check looks it up; `None` for a function it does not know.
    fn classify(&self, name: &str) -> Option<FunctionClass<'_>>;
}

/// The engine's own table: the safety column of the function table of
/// `xlsx-graft-compat` ([`xlsx_graft_compat::safeties`]).
///
/// A reviewed-safe function is [`ReviewedSafe`](FunctionClass::ReviewedSafe),
/// or [`Scheduled`](FunctionClass::Scheduled) when a later version of
/// xlsx-graft is to deny it. A dangerous function's reason is the English
/// of its kind of [`Danger`], with the version of xlsx-graft that began to
/// deny it and the ID of the advisory that announced it when the table
/// gives them. A function the table has not reviewed, and any kind of
/// safety a later version of `xlsx-graft-compat` adds, is
/// [`Unreviewed`](FunctionClass::Unreviewed), so it is denied.
///
/// # Examples
///
/// ```
/// use xlsx_graft_verify::{DefaultFunctions, FunctionClass, FunctionTable};
///
/// assert_eq!(DefaultFunctions.classify("SUM"), Some(FunctionClass::ReviewedSafe));
/// assert_eq!(DefaultFunctions.classify("FILTERXML"), Some(FunctionClass::Unreviewed));
/// assert!(matches!(
///     DefaultFunctions.classify("WEBSERVICE"),
///     Some(FunctionClass::Dangerous { .. })
/// ));
/// assert_eq!(DefaultFunctions.classify("MYFUNCTION"), None);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DefaultFunctions;

impl FunctionTable for DefaultFunctions {
    fn classify(&self, name: &str) -> Option<FunctionClass<'_>> {
        // The formula check passes the name upper-cased; a caller of the
        // public trait may pass any letter case, so classify upper-cases
        // the name before the look-up.
        let class = classes().get(name.to_ascii_uppercase().as_str())?;
        Some(class.as_function_class())
    }
}

/// A function's class in [`DefaultFunctions`], its reason written out.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Class {
    Safe,
    Scheduled {
        from: &'static str,
        reason: &'static str,
    },
    Dangerous(String),
    Unreviewed,
}

impl Class {
    /// The class of a function of `safety`.
    fn of(safety: Safety) -> Self {
        match safety {
            Safety::ReviewedSafe(safe) => Self::reviewed(safe.scheduled),
            Safety::Dangerous(found) => Self::Dangerous(dangerous_reason(
                found.danger,
                found.denied_since,
                found.advisory,
            )),
            _ => Self::Unreviewed,
        }
    }

    /// The class of a reviewed-safe function, which a later version of
    /// xlsx-graft is to deny when `scheduled` is set.
    fn reviewed(scheduled: Option<Scheduled>) -> Self {
        match scheduled {
            None => Self::Safe,
            Some(scheduled) => Self::Scheduled {
                from: scheduled.from,
                reason: scheduled.danger.reason(),
            },
        }
    }

    /// This class as a [`FunctionTable`] gives it.
    fn as_function_class(&self) -> FunctionClass<'_> {
        match self {
            Self::Safe => FunctionClass::ReviewedSafe,
            Self::Scheduled { from, reason } => FunctionClass::Scheduled { from, reason },
            Self::Dangerous(reason) => FunctionClass::Dangerous { reason },
            Self::Unreviewed => FunctionClass::Unreviewed,
        }
    }
}

/// The reason of a function dangerous for `danger`: the English of the
/// kind, then the version of xlsx-graft that began to deny it and the ID
/// of the advisory that announced it, each when given.
fn dangerous_reason(danger: Danger, denied_since: Option<&str>, advisory: Option<&str>) -> String {
    let since = denied_since.map_or_else(String::new, |version| {
        format!("; denied since xlsx-graft {version}")
    });
    let advisory = advisory.map_or_else(String::new, |id| format!("; advisory {id}"));
    format!("{}{since}{advisory}", danger.reason())
}

/// Every function's class, by its name, built at the first look-up.
fn classes() -> &'static HashMap<&'static str, Class> {
    static CLASSES: OnceLock<HashMap<&'static str, Class>> = OnceLock::new();
    CLASSES.get_or_init(|| {
        (xlsx_graft_compat::safeties().iter())
            .map(|row| (row.name, Class::of(row.safety)))
            .collect()
    })
}

#[cfg(test)]
mod tests;

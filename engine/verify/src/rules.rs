//! [`Rules`]: what the template allows and the host's policy forbids, over
//! the engine's defaults.
//!
//! A template only allows and a policy only forbids. Where both name the
//! same thing, the policy wins: the host sets the ceiling a template works
//! under.

use std::collections::{BTreeMap, BTreeSet};
use xlsx_graft_diagnostics::PartKind;

/// The template's `allow` lists and the policy's forbids.
///
/// # Examples
///
/// ```
/// use xlsx_graft_verify::{PartKind, Rules, Ruling};
///
/// let rules = Rules::default()
///     .allow_part(PartKind::Macro)
///     .allow_function("webservice")
///     .forbid_function("WEBSERVICE", "no workbook may reach the network");
/// assert_eq!(rules.part(PartKind::Macro), Ruling::Allowed);
/// assert_eq!(rules.part(PartKind::Ole), Ruling::Unruled);
/// assert_eq!(
///     rules.function("WebService"),
///     Ruling::Forbidden { reason: "no workbook may reach the network" }
/// );
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rules {
    /// The functions the template allows, upper-cased.
    allowed_functions: BTreeSet<String>,
    /// The part kinds the template allows.
    allowed_parts: BTreeSet<PartKind>,
    /// The functions the policy forbids, upper-cased, with why.
    forbidden_functions: BTreeMap<String, String>,
    /// The part kinds the policy forbids, with why.
    forbidden_parts: BTreeMap<PartKind, String>,
}

/// What [`Rules`] say of a part kind or a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ruling<'a> {
    /// The policy forbids it, for `reason`, whatever the template says.
    Forbidden {
        /// Why, in the policy's own words.
        reason: &'a str,
    },
    /// The template allows it, and the policy does not forbid it.
    Allowed,
    /// Neither says anything: the engine's defaults decide.
    Unruled,
}

impl Rules {
    /// These rules, with the template allowing content of `kind`.
    #[must_use]
    pub fn allow_part(mut self, kind: PartKind) -> Self {
        self.allowed_parts.insert(kind);
        self
    }

    /// These rules, with the template allowing the function `name`, in any
    /// letter case.
    #[must_use]
    pub fn allow_function(mut self, name: &str) -> Self {
        self.allowed_functions.insert(name.to_ascii_uppercase());
        self
    }

    /// These rules, with the policy forbidding content of `kind` for
    /// `reason`, in place of any reason it gave before.
    #[must_use]
    pub fn forbid_part(mut self, kind: PartKind, reason: impl Into<String>) -> Self {
        self.forbidden_parts.insert(kind, reason.into());
        self
    }

    /// These rules, with the policy forbidding the function `name`, in any
    /// letter case, for `reason`, in place of any reason it gave before.
    #[must_use]
    pub fn forbid_function(mut self, name: &str, reason: impl Into<String>) -> Self {
        (self.forbidden_functions).insert(name.to_ascii_uppercase(), reason.into());
        self
    }

    /// What these rules say of content of `kind`.
    #[must_use]
    pub fn part(&self, kind: PartKind) -> Ruling<'_> {
        ruling(
            self.forbidden_parts.get(&kind),
            self.allowed_parts.contains(&kind),
        )
    }

    /// What these rules say of the function `name`, in any letter case.
    #[must_use]
    pub fn function(&self, name: &str) -> Ruling<'_> {
        let name = name.to_ascii_uppercase();
        ruling(
            self.forbidden_functions.get(&name),
            self.allowed_functions.contains(&name),
        )
    }
}

/// The ruling of a policy that forbids for `forbidden`, if it does, and a
/// template that allows or not.
fn ruling(forbidden: Option<&String>, allowed: bool) -> Ruling<'_> {
    match forbidden {
        Some(reason) => Ruling::Forbidden { reason },
        None if allowed => Ruling::Allowed,
        None => Ruling::Unruled,
    }
}

#[cfg(test)]
mod tests;

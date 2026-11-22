//! [`describe_function`]: what the compatibility table says of one function,
//! and the JSON every host prints it as.
//!
//! The JSON of a function with a row of versions:
//!
//! ```json
//! {
//!   "name": "SEQUENCE",
//!   "known": "versioned",
//!   "fileName": "_xlfn.SEQUENCE",
//!   "prefix": "_xlfn.",
//!   "support": {
//!     "excel": {"added": "2021", "removed": null, "partial": false, "fallback": "name_error", "notes": null},
//!     "excel-mac": {"added": "2021", "removed": null, "partial": false, "fallback": "name_error", "notes": null},
//!     "libreoffice": {"added": "24.8", "removed": null, "partial": false, "fallback": null, "notes": null},
//!     "google-sheets": {"added": "current", "removed": null, "partial": false, "fallback": null, "notes": null},
//!     "numbers": {"added": null, "removed": null, "partial": false, "fallback": null, "notes": null},
//!     "onlyoffice": {"added": "8.0", "removed": null, "partial": false, "fallback": null, "notes": null}
//!   },
//!   "requires": [
//!     {"feature": "dynamic_array", "support": {"excel": {"...": "..."}}, "sources": ["https://..."]}
//!   ],
//!   "safety": {"class": "reviewed_safe", "danger": null},
//!   "sources": ["https://..."]
//! }
//! ```
//!
//! `known` says which state the name is in. `versioned` has every key above.
//! `baseline` is a function the table gives a safety but no row of versions
//! (`SUM`): it has `name`, `known`, `fileName`, `prefix`, `safety` and
//! `sources`, and no `support` and no `requires`. `unknown` is a name the
//! table does not list, which verify denies: it has `name` and `known` only.
//!
//! `name` is the name without a leading `_xlfn.` and `_xlws.`, in upper
//! case. `fileName` is `prefix` and `name`, how Excel writes the function in
//! a file; both are `null` when the table does not say which prefix the
//! function has. `support` has a key per application (`excel`, `excel-mac`,
//! `libreoffice`, `google-sheets`, `numbers`, `onlyoffice`). `added` is the
//! version that added the function (`"2021"`), the version that has it when
//! the versions before it are not known (`"<=14.4"`), a version that lacks it
//! and a later version that has it, the versions between not known
//! (`">2007,<=2016"`), `false` when no version has
//! it, and `null` when no source says. `safety.class` is `reviewed_safe`,
//! `dangerous` or `unreviewed`; `safety.danger` names the kind of danger of a
//! dangerous function and is `null` otherwise. A baseline function ECMA-376
//! defines has `prefix` `""` and `sources` `["ECMA-376-1:2016 §18.17.7"]`.
//! A baseline function ECMA-376 does not define (ANCHORARRAY, SINGLE, CALL,
//! COPILOT, REGISTER.ID) has `prefix` and `fileName` `null` and `sources`
//! `[]`.
//!
//! The keys and their types are the contract; a key may be added, never
//! renamed or retyped.

mod json;
mod text;

use json::Body;
use serde::{Serialize, Serializer};
use xlsx_graft_compat::{Lookup, look_up};

pub use text::FunctionText;

/// What the table says of one function: what [`describe_function`] returns,
/// which [`FunctionDescription::to_json`] writes as the JSON each host
/// returns and [`FunctionText`] writes as the text a person reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDescription(Lookup);

/// Looks up the function `name`, written as a formula writes it, with or
/// without its `_xlfn.` and `_xlws.` prefixes and in any case.
#[must_use]
pub fn describe_function(name: &str) -> FunctionDescription {
    FunctionDescription(look_up(name))
}

impl FunctionDescription {
    /// The function's name as the JSON's `name` gives it: without a leading
    /// `_xlfn.` or `_xlws.`, in upper case.
    #[must_use]
    pub fn name(&self) -> &str {
        self.0.name.as_str()
    }

    /// The description as the JSON every host prints, in one line.
    ///
    /// # Errors
    ///
    /// An error does not happen: every field serializes. `serde_json`'s own
    /// error is returned rather than hidden.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

impl Serialize for FunctionDescription {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Body::of(&self.0).serialize(serializer)
    }
}

#[cfg(test)]
mod tests;

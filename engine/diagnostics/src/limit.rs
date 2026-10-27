//! [`Limit`]: the value of one of the engine's limits, with who set it.

use serde::{Serialize, Serializer};
use std::fmt;

/// The value of a limit in force, and who set it.
///
/// A code that a limit decides takes it as an argument: the argument is
/// written as the bare number, and the row hands the whole `Limit` to
/// [`DecidedBy::limit`](crate::DecidedBy::limit), so `decidedBy` names its
/// source without the row knowing where the value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Limit {
    /// The value in force.
    pub value: u64,
    /// Who set it.
    pub source: LimitSource,
}

impl Limit {
    /// `value` as the engine's built-in default.
    #[must_use]
    pub fn default_of(value: u64) -> Self {
        Self {
            value,
            source: LimitSource::Default,
        }
    }
}

/// Who set the value of a limit.
///
/// Only the engine's defaults exist until a caller can pass a policy; a
/// limit set from a policy will be its own source then.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LimitSource {
    /// The engine's built-in default.
    Default,
}

impl fmt::Display for Limit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl Serialize for Limit {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.value)
    }
}

#[cfg(test)]
mod tests;

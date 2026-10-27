//! [`Record`]: one collected diagnostic, and the JSON object it serializes as.

use crate::table::Args;
use crate::{Diagnostic, Location};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// A [`Diagnostic`] as [`Diagnostics`](crate::Diagnostics) recorded it:
/// where in the package it points, and which engine line reported it.
///
/// Serializes as the JSON every host prints:
///
/// ```json
/// {
///   "code": "diagnostics_truncated",
///   "severity": "notice",
///   "message": "...",
///   "args": { "dropped": 3, "droppedErrors": 0, "limit": 1000 },
///   "location": {},
///   "decidedBy": { "kind": "limit", "name": "maxDiagnostics", "value": 1000, "source": "default" },
///   "fix": null,
///   "origin": "<file>:<line>"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// What was found.
    pub diagnostic: Diagnostic,
    /// Where in the package it points.
    pub location: Location,
    origin: &'static std::panic::Location<'static>,
}

impl Record {
    pub(crate) fn new(
        diagnostic: Diagnostic,
        location: Location,
        origin: &'static std::panic::Location<'static>,
    ) -> Self {
        Self {
            diagnostic,
            location,
            origin,
        }
    }

    /// The engine source line (`file:line`) that reported it. Outside the
    /// contract: it changes with any edit to the engine.
    #[must_use]
    pub fn origin(&self) -> String {
        format!("{}:{}", self.origin.file(), self.origin.line())
    }
}

impl Serialize for Record {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let diagnostic = &self.diagnostic;
        let mut record = serializer.serialize_struct("Diagnostic", 8)?;
        record.serialize_field("code", &diagnostic.code())?;
        record.serialize_field("severity", &diagnostic.severity())?;
        record.serialize_field("message", &diagnostic.message())?;
        record.serialize_field("args", &Args(diagnostic))?;
        record.serialize_field("location", &self.location)?;
        record.serialize_field("decidedBy", &diagnostic.decided_by())?;
        record.serialize_field("fix", &diagnostic.fix())?;
        record.serialize_field("origin", &self.origin())?;
        record.end()
    }
}

#[cfg(test)]
mod tests;

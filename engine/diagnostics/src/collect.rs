//! [`Diagnostics`]: the collector a reader fills as it goes, bounded so a
//! hostile workbook cannot make the report as large as itself.

use crate::{Diagnostic, Limit, Location, Record, Severity};

/// Every diagnostic found, in the order it was found.
///
/// Keeps at most [`limit`](Self::with_limit) records ([`DEFAULT_LIMIT`]
/// unless set). Past that it only counts, and [`finish`](Self::finish) adds
/// one `diagnostics_truncated` notice that says how many were dropped.
///
/// [`DEFAULT_LIMIT`]: Self::DEFAULT_LIMIT
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    records: Vec<Record>,
    limit: Limit,
    /// `limit` as a length; a limit past `usize` keeps everything.
    keep: usize,
    dropped: u64,
    dropped_errors: u64,
    has_errors: bool,
}

impl Diagnostics {
    /// How many records a collector keeps unless told otherwise.
    pub const DEFAULT_LIMIT: u64 = 1_000;

    /// An empty collector that keeps [`DEFAULT_LIMIT`](Self::DEFAULT_LIMIT)
    /// records.
    #[must_use]
    pub fn new() -> Self {
        Self::with_limit(Limit::default_of(Self::DEFAULT_LIMIT))
    }

    /// An empty collector that keeps `limit` records.
    #[must_use]
    pub fn with_limit(limit: Limit) -> Self {
        Self {
            records: Vec::new(),
            limit,
            keep: usize::try_from(limit.value).unwrap_or(usize::MAX),
            dropped: 0,
            dropped_errors: 0,
            has_errors: false,
        }
    }

    /// Records `diagnostic` at `location`, with the caller as its origin, or
    /// counts it as dropped once the limit is reached.
    ///
    /// A helper that pushes on its callers' behalf is `#[track_caller]` too,
    /// so the origin names the line that found the problem.
    #[track_caller]
    pub fn push(&mut self, diagnostic: Diagnostic, location: Location) {
        let is_error = diagnostic.severity() == Severity::Error;
        self.has_errors |= is_error;
        if self.records.len() < self.keep {
            self.records.push(Record::new(
                diagnostic,
                location,
                std::panic::Location::caller(),
            ));
        } else {
            self.dropped = self.dropped.saturating_add(1);
            self.dropped_errors = self.dropped_errors.saturating_add(u64::from(is_error));
        }
    }

    /// The records kept so far, without the truncation notice.
    #[must_use]
    pub fn records(&self) -> &[Record] {
        &self.records
    }

    /// How many diagnostics were dropped past the limit.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Whether any error was pushed, kept or dropped.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.has_errors
    }

    /// The records kept, followed by a `diagnostics_truncated` notice when
    /// any were dropped. Its origin is the caller.
    #[track_caller]
    #[must_use]
    pub fn finish(self) -> Vec<Record> {
        let mut records = self.records;
        if self.dropped > 0 {
            let truncated = Diagnostic::DiagnosticsTruncated {
                dropped: self.dropped,
                dropped_errors: self.dropped_errors,
                limit: self.limit,
            };
            records.push(Record::new(
                truncated,
                Location::default(),
                std::panic::Location::caller(),
            ));
        }
        records
    }
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;

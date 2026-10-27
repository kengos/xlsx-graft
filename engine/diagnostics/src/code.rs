//! The code table: one row per diagnostic code, read by `diagnostics!`
//! (`table.rs`, which shows the shape of a row).
//!
//! A shipped code and its argument names are never renamed or repurposed,
//! and a removed code's wire string is never reused: the tests list every
//! removed one and refuse a row that brings it back. Every row needs a
//! golden JSON example in the tests.
//!
//! line-budget-exempt: the code table is data, one row per code, and grows by
//! a row with every code; splitting it would split the one place a reader
//! scans for a code.

use crate::table::diagnostics;
use crate::{DecidedBy, Limit};

diagnostics! {
    /// More diagnostics were found than the collector keeps. The extra ones
    /// were counted and dropped; this one, added last, says how many, and
    /// how many of them were errors, since the records kept may hold none.
    DiagnosticsTruncated {
        /// How many diagnostics were dropped.
        dropped: u64,
        /// How many of the dropped diagnostics were errors.
        dropped_errors: u64,
        /// How many the collector keeps.
        limit: Limit,
    } => "diagnostics_truncated", Notice,
        message: format!("{dropped} more diagnostic(s), {dropped_errors} of them error(s), were found after the first {limit} and are not listed"),
        decided_by: Some(DecidedBy::limit("maxDiagnostics", *limit)),
        fix: None;
}

#[cfg(test)]
mod tests;

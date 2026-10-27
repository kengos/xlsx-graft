//! xlsx-graft diagnostics: what the engine reports about a workbook, and the
//! JSON every host (CLI, MCP, SDKs) prints it as.
//!
//! A workbook is untrusted input, and reading one reports EVERY problem it
//! finds rather than stopping at the first. Each problem is a [`Diagnostic`]:
//! one variant per closed [`Code`], whose fields are the code's arguments.
//! The code's row in the table (`code.rs`) decides the rest: its
//! [`Severity`], its English message, where the rule comes from
//! ([`DecidedBy`]) and how to fix it ([`Fix`]). [`Diagnostics`] collects them
//! in the order they were found, each as a [`Record`] that adds the
//! [`Location`] in the package and the engine line that reported it.
//!
//! The contract with consumers is the `code`, the `severity`, the `args` (each
//! code's keys and types), the `location` keys and the shapes of `decidedBy`
//! and `fix`. An echoed value is for display, not a key to look a name up
//! by. The message and fix texts are English defaults and may change: a
//! translating consumer builds its own text from `code` and `args`. The
//! `origin` is outside the contract. An integer argument is a full unsigned
//! 64-bit value, written as a JSON number without clipping: a consumer reads
//! it as a 64-bit integer, not as a double. A shipped code and its argument
//! names are never renamed, and a removed code's wire string is never reused.
//!
//! **Where this differs from shojiku's diagnostics** (the design this follows
//! for codes, echoes and origins): shojiku keeps `args` as a map from string
//! keys to values, so a mistyped key or a value of the wrong type surfaces
//! only at run time, if a test happens to emit that code. Here each code is a
//! variant whose fields ARE its arguments, so the compiler holds every
//! emitter to the code's keys and types, and an argument can only be a
//! `u64`, a `bool`, an [`Echo`], a [`Limit`] or a reason enum.

mod arg;
mod code;
mod collect;
mod decided;
mod echo;
mod limit;
mod location;
mod reason;
mod record;
mod table;

pub use code::{Code, Diagnostic};
pub use collect::Diagnostics;
pub use decided::{DecidedBy, Fix};
pub use echo::{Echo, MAX_ECHO};
pub use limit::{Limit, LimitSource};
pub use location::Location;
pub use reason::{EncodingReason, InflateReason, ZipDirectoryReason, ZipHeaderField};
pub use record::Record;

use serde::Serialize;

/// How a diagnostic weighs on what the engine does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The engine does not write or accept the workbook.
    Error,
    /// The engine writes the workbook and says what it could not honour.
    Warn,
    /// A fact the engine decides; fixing the workbook does not change it.
    Notice,
}

#[cfg(test)]
mod tests;

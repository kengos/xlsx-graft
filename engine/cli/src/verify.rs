//! `verify`: checks a workbook under the host's policy and the caller's
//! rules, through `xlsx_graft_host`, the same check the C ABI and the MCP
//! server run. The long help in `help.rs` says what each format prints.
//!
//! Unlike `sheets` and `read`, `verify` prints its result whatever
//! happened: a file that cannot be read (the workbook, the policy) is the
//! result's `error`, and a fault of the policy or the rules is its
//! diagnostics. Only a command line clap refuses, and an output that cannot
//! be written, print no result.

mod help;

use crate::{CliError, Format, Outcome, Report, emit};
use clap::Args;
pub(crate) use help::{AFTER_LONG_HELP, LONG_ABOUT};
use serde_json::{Map, Value};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use xlsx_graft_host::{
    FailOn, Input, MAX_SETTING_BYTES, Prepared, ReportText, prepare_parts, verify_path,
};

/// The arguments of `verify`.
#[derive(Debug, Args)]
pub struct VerifyArgs {
    /// The host's policy: a JSON file of limits, forbidden functions and
    /// parts, `failOn` and `denyScheduled` (the help's end says how).
    #[arg(long, value_name = "FILE")]
    policy: Option<PathBuf>,
    /// A version the workbook must work in, such as 'excel >= 2016'; what it
    /// does not support is an error. Repeat for each.
    #[arg(long = "target", value_name = "ENTRY")]
    targets: Vec<String>,
    /// A version the workbook should work in; what it does not support is a
    /// warning. Repeat for each.
    #[arg(long = "best-effort", value_name = "ENTRY")]
    best_effort: Vec<String>,
    /// Let through a function the check denies by default, named without a
    /// prefix such as `_xlfn.`. Repeat for each.
    #[arg(long = "allow-function", value_name = "NAME")]
    allow_functions: Vec<String>,
    /// Let through a kind of part the check denies by default: macro,
    /// external_link, dde, ole, activex, data_connection or web_publish.
    /// Repeat for each.
    #[arg(long = "allow-part", value_name = "KIND")]
    allow_parts: Vec<String>,
    /// Make a warning fail the check too. A stricter `failOn` in the policy
    /// wins.
    #[arg(long, value_name = "LEVEL", value_parser = ["error", "warn"])]
    fail_on: Option<String>,
    /// The workbook to verify.
    path: PathBuf,
}

/// Verifies the workbook `args` names, printing the result in `format` to
/// `out`.
pub(crate) fn run(
    args: &VerifyArgs,
    format: Format,
    out: &mut dyn Write,
) -> Result<Outcome, CliError> {
    let (report, invalid) = check(args);
    emit(format, &report, out)?;
    Ok(outcome(&report, invalid))
}

/// The result, and whether the policy or the rules were not valid.
fn check(args: &VerifyArgs) -> (xlsx_graft_host::Report, bool) {
    let policy = match args.policy.as_deref().map(read_policy) {
        Some(Err(error)) => {
            let path = args.policy.as_deref().unwrap_or(Path::new(""));
            let message = format!("the policy {} could not be read: {error}", path.display());
            return (
                xlsx_graft_host::Report::io_failure(message, fail_on(args)),
                false,
            );
        }
        Some(Ok(bytes)) => Some(bytes),
        None => None,
    };
    let policy = policy.as_deref().map_or(Input::Absent, Input::Bytes);
    let prepared = prepare_parts(policy, Input::Value(&rules(args)));
    let invalid = matches!(prepared, Prepared::Invalid { .. });
    (verify_path(&args.path, &prepared), invalid)
}

/// The exit: 1 when a file could not be read; otherwise 2 when the policy or
/// the rules were not valid, 0 when the result is ok and 1 when it is not.
fn outcome(report: &xlsx_graft_host::Report, invalid: bool) -> Outcome {
    if report.error.is_some() {
        Outcome::Failure
    } else if invalid {
        Outcome::Invalid
    } else if report.ok {
        Outcome::Success
    } else {
        Outcome::Failure
    }
}

/// The policy file's bytes, up to one past what the host reads, so that a
/// larger file is reported as too large without being read whole.
fn read_policy(path: &Path) -> io::Result<Vec<u8>> {
    let limit = u64::try_from(MAX_SETTING_BYTES).map_or(u64::MAX, |max| max.saturating_add(1));
    let mut bytes = Vec::new();
    File::open(path)?.take(limit).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The rules the arguments give, with the template's keys and only the keys
/// given: `targets`, `bestEffort`, `functions.allow`, `parts.allow` and
/// `failOn`.
fn rules(args: &VerifyArgs) -> Value {
    let mut rules = Map::new();
    let mut put = |key: &str, value: Value| rules.insert(key.to_owned(), value);
    if !args.targets.is_empty() {
        put("targets", strings(&args.targets));
    }
    if !args.best_effort.is_empty() {
        put("bestEffort", strings(&args.best_effort));
    }
    if !args.allow_functions.is_empty() {
        put("functions", allow(&args.allow_functions));
    }
    if !args.allow_parts.is_empty() {
        put("parts", allow(&args.allow_parts));
    }
    if let Some(fail_on) = &args.fail_on {
        put("failOn", Value::String(fail_on.clone()));
    }
    Value::Object(rules)
}

fn strings(list: &[String]) -> Value {
    Value::Array(list.iter().cloned().map(Value::String).collect())
}

/// `{"allow": [...]}`.
fn allow(list: &[String]) -> Value {
    let mut map = Map::new();
    map.insert("allow".to_owned(), strings(list));
    Value::Object(map)
}

/// The `failOn` of a result the policy could not be read for: the caller's.
fn fail_on(args: &VerifyArgs) -> FailOn {
    if args.fail_on.as_deref() == Some("warn") {
        FailOn::Warn
    } else {
        FailOn::Error
    }
}

impl Report for xlsx_graft_host::Report {
    fn write_text(&self, out: &mut dyn Write) -> io::Result<()> {
        write!(out, "{}", ReportText(self))
    }
}

#[cfg(test)]
mod tests;

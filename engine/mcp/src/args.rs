//! The server's command line, `xlsx-graft-mcp [--policy FILE]`, read into
//! the [`Context`] the server starts with.
//!
//! `--policy FILE` is the only argument, written as two arguments:
//! `--policy=FILE`, a second `--policy`, a `--policy` without a file and any
//! other argument are a usage error. The file is the policy JSON the host
//! reads (`xlsx_graft_host::Policy`); a server whose policy cannot be read or
//! is not valid does not start, so that a mistake never leaves it running
//! with limits the operator did not choose.

use crate::Context;
use crate::rpc::clip;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use thiserror::Error;
use xlsx_graft_host::{DiagnosticsText, Input, Policy, Record, read_setting};

/// The flag that names the policy file.
const POLICY_FLAG: &str = "--policy";

/// Why the server did not start.
#[derive(Debug, Error)]
pub enum StartError {
    /// The command line is not `xlsx-graft-mcp [--policy FILE]`; the string
    /// says what is wrong with it.
    #[error("{0}\nusage: xlsx-graft-mcp [--policy FILE]")]
    Usage(String),
    /// The policy file could not be opened or read.
    #[error("the policy {} could not be read: {source}", .path.display())]
    PolicyUnreadable {
        /// The file `--policy` names.
        path: PathBuf,
        /// Why it could not be read.
        source: io::Error,
    },
    /// The policy file is not valid: its `policy_invalid` diagnostics, every
    /// fault of it.
    #[error("the policy {} is not valid:\n{}", .path.display(), DiagnosticsText(.records))]
    PolicyInvalid {
        /// The file `--policy` names.
        path: PathBuf,
        /// The `policy_invalid` records, one for each fault.
        records: Vec<Record>,
    },
}

/// The [`Context`] the command line `args` (without the program name) asks
/// for: the default policy without `--policy`, and the policy of the file it
/// names with it.
///
/// # Errors
///
/// [`StartError::Usage`] for a command line that is not
/// `[--policy FILE]`, [`StartError::PolicyUnreadable`] for a file that
/// cannot be read and [`StartError::PolicyInvalid`] for a policy with a
/// fault.
pub fn start(args: impl IntoIterator<Item = OsString>) -> Result<Context, StartError> {
    let Some(path) = policy_path(args)? else {
        return Ok(Context::default());
    };
    let bytes = read_setting(&path).map_err(|source| StartError::PolicyUnreadable {
        path: path.clone(),
        source,
    })?;
    match Policy::read(Input::Bytes(&bytes)) {
        Ok(policy) => Ok(Context::new(policy)),
        Err(records) => Err(StartError::PolicyInvalid { path, records }),
    }
}

/// The file `--policy` names, or `None` for no arguments.
fn policy_path(args: impl IntoIterator<Item = OsString>) -> Result<Option<PathBuf>, StartError> {
    let mut args = args.into_iter();
    let Some(flag) = args.next() else {
        return Ok(None);
    };
    if flag != POLICY_FLAG {
        return Err(unexpected(&flag));
    }
    let Some(file) = args.next() else {
        return Err(StartError::Usage(format!("{POLICY_FLAG} needs a FILE")));
    };
    match args.next() {
        None => Ok(Some(PathBuf::from(file))),
        Some(extra) => Err(unexpected(&extra)),
    }
}

/// The usage error of the argument `arg`, which is not accepted where it is.
fn unexpected(arg: &OsString) -> StartError {
    let arg = arg.to_string_lossy();
    StartError::Usage(format!("unexpected argument: {}", clip(&arg)))
}

#[cfg(test)]
mod tests;

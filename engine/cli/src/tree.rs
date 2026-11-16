//! `tree`: the parts of a workbook and the relationships between them,
//! through `xlsx_graft_host`, the same view the C ABI and the MCP server
//! give. The long help in `help.rs` says what each format prints.
//!
//! Like `verify`, `tree` prints its view whatever happened: a file that
//! cannot be read is the view's `error`. Only a command line clap refuses,
//! and an output that cannot be written, print no view.

mod help;

use crate::view::outcome;
use crate::{CliError, Format, Outcome, emit};
use clap::Args;
pub(crate) use help::{AFTER_LONG_HELP, LONG_ABOUT};
use std::io::Write;
use std::path::PathBuf;
use xlsx_graft_host::{HostLimits, tree_path};

/// The arguments of `tree`.
#[derive(Debug, Args)]
pub struct TreeArgs {
    /// The workbook whose parts to list.
    path: PathBuf,
}

/// Lists the parts of the workbook `args` names, printing the view in
/// `format` to `out`, under the host's default limits.
pub(crate) fn run(
    args: &TreeArgs,
    format: Format,
    out: &mut dyn Write,
) -> Result<Outcome, CliError> {
    let view = tree_path(&args.path, &HostLimits::default());
    emit(format, &view, out)?;
    Ok(outcome(&view))
}

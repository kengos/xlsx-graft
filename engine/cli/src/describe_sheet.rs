//! `describe-sheet`: the cells of one sheet, through `xlsx_graft_host`, the
//! same view the C ABI, the SDKs and the MCP server give. The long help in
//! `help.rs` says what each format prints.
//!
//! Like `verify`, `describe-sheet` prints its view whatever happened to the
//! workbook: a file that cannot be read, and a sheet that cannot be chosen
//! or read, are the view's `error`. A request the host does not take (a
//! range that is no A1 range, a number of rows or columns out of its
//! bounds) is a command line not taken: exit 2, the reason on stderr and no
//! view, as for a command line clap refuses.

mod help;

use crate::view::outcome;
use crate::{CliError, Format, Outcome, emit};
use clap::Args;
pub(crate) use help::{AFTER_LONG_HELP, LONG_ABOUT};
use std::io::Write;
use std::path::PathBuf;
use xlsx_graft_host::{
    HostLimits, SheetChoice, SheetRequest, SheetRequestError, describe_sheet_path,
};

/// The arguments of `describe-sheet`.
#[derive(Debug, Args)]
pub struct DescribeSheetArgs {
    /// The workbook to read.
    path: PathBuf,
    /// The sheet, by name, letter case aside. Without it and --sheet-index,
    /// the first sheet (index 0), whatever its kind.
    #[arg(long, value_name = "NAME", conflicts_with = "sheet_index")]
    sheet: Option<String>,
    /// The sheet, by its place in the workbook's list of sheets, hidden ones
    /// included: 0 is the first.
    #[arg(long, value_name = "N")]
    sheet_index: Option<usize>,
    /// The cells to show, as an A1 range such as A1:H50, or one cell such as
    /// B2 to start from; the whole sheet without it.
    #[arg(long, value_name = "RANGE")]
    range: Option<String>,
    /// The most rows to show: 1 to 10000, 100 without it.
    #[arg(long, value_name = "N")]
    max_rows: Option<u64>,
    /// The most columns to show: 1 to 1000, 30 without it.
    #[arg(long, value_name = "N")]
    max_columns: Option<u64>,
}

/// Describes the sheet `args` names, printing the view in `format` to `out`,
/// under the host's default limits. A request the host does not take is
/// written to `err`, and nothing to `out`.
pub(crate) fn run(
    args: &DescribeSheetArgs,
    format: Format,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Outcome, CliError> {
    let sheet = match (&args.sheet, args.sheet_index) {
        (Some(name), _) => Some(SheetChoice::Name(name)),
        (None, Some(index)) => Some(SheetChoice::Index(index)),
        (None, None) => None,
    };
    let request = SheetRequest::new(
        sheet,
        args.range.as_deref(),
        args.max_rows,
        args.max_columns,
    );
    let request = match request {
        Ok(request) => request,
        Err(error) => {
            // An error writing the reason to stderr is ignored: the status
            // still says the command line was not taken.
            let _ = writeln!(err, "xlsx-graft: {}", reason(&error));
            return Ok(Outcome::Invalid);
        }
    };
    let view = describe_sheet_path(&args.path, &request, &HostLimits::default());
    emit(format, &view, out)?;
    Ok(outcome(&view))
}

/// Why the request is not one, in the CLI's own words: the host's message
/// names the arguments of the MCP server, and the CLI's are `--max-rows` and
/// `--max-columns`.
fn reason(error: &SheetRequestError) -> String {
    match error {
        SheetRequestError::MaxRows(rows) => format!(
            "--max-rows {rows} is not between 1 and {}",
            SheetRequest::HARD_MAX_ROWS
        ),
        SheetRequestError::MaxColumns(columns) => format!(
            "--max-columns {columns} is not between 1 and {}",
            SheetRequest::HARD_MAX_COLUMNS
        ),
        SheetRequestError::Range(text) => format!(
            "--range \"{text}\" is not a range of cells such as A1:H50 \
             (columns A to XFD, rows 1 to 1048576)"
        ),
    }
}

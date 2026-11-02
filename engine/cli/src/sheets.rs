//! `sheets`: the workbook's sheets, in the order its workbook part lists
//! them, and what opening it came to. What each format prints is `help.rs`,
//! the help that says it.
//!
//! An SDK that runs the CLI lists a book's sheets with it, before reading
//! one with `read`: the same open, so the same diagnostics of the workbook.

mod help;

use crate::text::{escape, problems};
use crate::workbook::{self, Refused};
use crate::{CliError, Format, Outcome};
use clap::Args;
pub(crate) use help::{AFTER_LONG_HELP, LONG_ABOUT};
use serde::Serialize;
use std::fmt::Write as _;
use std::io::Write;
use std::path::PathBuf;
use xlsx_graft_diagnostics::{Diagnostics, Record};
use xlsx_graft_read::{Refusal, Sheet, SheetKind, SheetState};

/// The arguments of `sheets`.
#[derive(Debug, Args)]
pub struct SheetsArgs {
    /// The workbook whose sheets to list.
    path: PathBuf,
}

/// What `sheets` prints: in JSON, one line.
#[derive(Debug, Serialize)]
struct Listing {
    /// No `error`, and no diagnostic an error.
    ok: bool,
    /// The sheets, hidden ones and every kind included.
    sheets: Vec<Listed>,
    /// Why there is no list, when there is none.
    error: Option<Refused>,
    /// Every problem found in opening the workbook.
    diagnostics: Vec<Record>,
}

/// One sheet.
#[derive(Debug, Serialize)]
struct Listed {
    /// Its index, as `read --sheet-index` takes it.
    index: usize,
    /// Its name.
    name: String,
    /// What kind of sheet it is; `unknown` when its part is not known.
    kind: &'static str,
    /// Whether it shows.
    state: &'static str,
}

impl Listed {
    fn of(index: usize, sheet: &Sheet) -> Self {
        let kind = match sheet.part().map(|part| part.kind()) {
            Some(SheetKind::Worksheet) => "worksheet",
            Some(SheetKind::Chartsheet) => "chartsheet",
            Some(SheetKind::Dialogsheet) => "dialogsheet",
            Some(SheetKind::Macrosheet) => "macrosheet",
            None => "unknown",
        };
        let state = match sheet.state() {
            SheetState::Visible => "visible",
            SheetState::Hidden => "hidden",
            SheetState::VeryHidden => "veryHidden",
        };
        Self {
            index,
            name: sheet.name().to_owned(),
            kind,
            state,
        }
    }
}

/// Lists the sheets of the workbook `args` names, printing in `format` the
/// list to `out` and, in text, the problems to `err`.
pub(crate) fn run(
    args: &SheetsArgs,
    format: Format,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Outcome, CliError> {
    let mut diagnostics = Diagnostics::new();
    let book = workbook::open(&args.path, &mut diagnostics)?;
    let error = book
        .is_none()
        .then_some(Refused(Refusal::WorkbookUnreadable));
    let sheets = book.as_ref().map_or(&[][..], |book| book.sheets());
    let listing = Listing {
        ok: error.is_none() && !diagnostics.has_errors(),
        sheets: (sheets.iter().enumerate())
            .map(|(index, sheet)| Listed::of(index, sheet))
            .collect(),
        error,
        diagnostics: diagnostics.finish(),
    };
    match format {
        Format::Json => {
            let json = serde_json::to_string(&listing)?;
            writeln!(out, "{json}").map_err(CliError::Write)?;
        }
        Format::Text => text(&listing, out, err).map_err(CliError::Write)?,
    }
    Ok(if listing.ok {
        Outcome::Success
    } else {
        Outcome::Failure
    })
}

/// Writes each sheet as a line to `out` (its index, name, kind and state,
/// separated by tabs, the name escaped as `read` escapes a string), then
/// the problems to `err`.
fn text(listing: &Listing, out: &mut dyn Write, err: &mut dyn Write) -> std::io::Result<()> {
    let mut line = String::new();
    for sheet in &listing.sheets {
        line.clear();
        // Writing to a String does not fail.
        let _ = write!(line, "{}\t", sheet.index);
        escape(&mut line, &sheet.name);
        let _ = writeln!(line, "\t{}\t{}", sheet.kind, sheet.state);
        out.write_all(line.as_bytes())?;
    }
    out.flush()?;
    problems(err, &listing.diagnostics, listing.error.as_ref());
    Ok(())
}

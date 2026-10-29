//! `read`: a sheet's rows, printed a line each as they are read, and what
//! the reading came to. What each format prints is `help.rs`, the help
//! that says it; choosing the sheet is `select.rs`, and the two formats
//! `text.rs` and `json.rs`.
//!
//! Nothing holds the rows: each is printed before the next is read, so
//! the memory taken does not grow with the sheet. A write that fails stops
//! the reading there.

mod help;
mod json;
mod select;
mod text;

use crate::{CliError, Format, Outcome};
use clap::Args;
pub(crate) use help::{AFTER_LONG_HELP, LONG_ABOUT};
use select::Refusal;
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use xlsx_graft_diagnostics::{Diagnostics, Record};
use xlsx_graft_package::Limits;
use xlsx_graft_read::{Book, ReadLimits, Row};
use xlsx_graft_spool::TempFiles;

/// The arguments of `read`.
#[derive(Debug, Args)]
pub struct ReadArgs {
    /// The workbook to read.
    path: PathBuf,
    /// The sheet to read, by name, letter case aside. Without it or
    /// --sheet-index, the first sheet (index 0), whatever its kind.
    #[arg(long, value_name = "NAME", conflicts_with = "sheet_index")]
    sheet: Option<String>,
    /// The sheet to read, by its place in the workbook's list of sheets,
    /// hidden ones included: 0 is the first.
    #[arg(long, value_name = "N")]
    sheet_index: Option<usize>,
}

/// Where the rows and the end go, in one format.
trait Sink {
    /// Prints a row.
    fn row(&mut self, row: &Row) -> Result<(), CliError>;
    /// Prints what the reading came to.
    fn end(&mut self, end: &End) -> Result<(), CliError>;
}

/// What the reading came to: in JSON, the last line.
#[derive(Debug, Default, Serialize)]
struct End {
    /// No `error`, and no diagnostic an error.
    ok: bool,
    /// The sheet chosen, once it is.
    sheet: Option<Chosen>,
    /// How many rows were printed.
    rows: u64,
    /// Why no sheet was read to its end, when none was.
    error: Option<Refusal>,
    /// Every problem found in the workbook.
    diagnostics: Vec<Record>,
}

/// The sheet chosen.
#[derive(Debug, Serialize)]
struct Chosen {
    /// Its index in the workbook's list of sheets.
    index: usize,
    /// Its name.
    name: String,
}

/// Reads the sheet `args` asks for, printing in `format` its rows to `out`
/// and, in text, the problems to `err`.
pub(crate) fn run(
    args: &ReadArgs,
    format: Format,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Outcome, CliError> {
    let file = File::open(&args.path).map_err(|source| CliError::Open {
        path: args.path.clone(),
        source,
    })?;
    let mut diagnostics = Diagnostics::new();
    let book = Book::open(
        BufReader::new(file),
        &Limits::default(),
        &ReadLimits::default(),
        &mut TempFiles::new(),
        &mut diagnostics,
    )?;
    let (mut as_text, mut as_json);
    let sink: &mut dyn Sink = match format {
        Format::Text => {
            as_text = text::Text::new(out, err);
            &mut as_text
        }
        Format::Json => {
            as_json = json::Json::new(out);
            &mut as_json
        }
    };
    let mut end = End::default();
    match book {
        Some(book) => read(&book, args, sink, &mut end, &mut diagnostics)?,
        None => end.error = Some(Refusal::WorkbookUnreadable),
    }
    end.ok = end.error.is_none() && !diagnostics.has_errors();
    end.diagnostics = diagnostics.finish();
    sink.end(&end)?;
    Ok(if end.ok {
        Outcome::Success
    } else {
        Outcome::Failure
    })
}

/// Reads the sheet of `book` that `args` asks for to its end, printing its
/// rows to `sink`, and notes in `end` which it was, how many rows came,
/// and why it was not read, if it was not.
fn read(
    book: &Book<BufReader<File>>,
    args: &ReadArgs,
    sink: &mut dyn Sink,
    end: &mut End,
    diagnostics: &mut Diagnostics,
) -> Result<(), CliError> {
    let (index, sheet) = match select::sheet(book, args.sheet.as_deref(), args.sheet_index) {
        Ok(chosen) => chosen,
        Err(refusal) => {
            end.error = Some(refusal);
            return Ok(());
        }
    };
    let name = sheet.name().to_owned();
    end.sheet = Some(Chosen { index, name });
    if let Err(refusal) = select::worksheet(sheet) {
        end.error = Some(refusal);
        return Ok(());
    }
    let Some(mut rows) = book.rows(index) else {
        end.error = Some(Refusal::SheetUnreadable(sheet.name().to_owned()));
        return Ok(());
    };
    let mut row = Row::new();
    while rows.next_row(&mut row, diagnostics)? {
        sink.row(&row)?;
        end.rows = end.rows.saturating_add(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

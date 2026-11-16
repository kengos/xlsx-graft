//! The entry points of the sheet view: opening the workbook, choosing the
//! sheet and reading its rows into a [`Grid`].

use super::collect::Collector;
use super::request::{Selection, SheetRequest};
use super::{Grid, GridSheet, Span, Truncated};
use crate::open::open_regular;
use crate::policy::limits::HostLimits;
use crate::report::Failure;
use crate::view::View;
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use xlsx_graft_diagnostics::{Diagnostics, Echo};
use xlsx_graft_package::PackageError;
use xlsx_graft_read::{Book, ReadError, Refusal, Row, SheetChoice, SheetRows};
use xlsx_graft_spool::{SpoolFactory, TempFiles};

/// The cells of the sheet `request` asks for, in the workbook `reader`
/// holds, under `limits`.
///
/// A workbook or a sheet that cannot be read gives the diagnostics and an
/// `error` (`workbook_unreadable`, `sheet_not_found`, `sheet_ambiguous`,
/// `sheet_not_worksheet`, `sheet_unreadable`); an I/O error of `reader`, or
/// of the temporary file a large shared strings table moves to, is the
/// view's `error` (`io`). A sheet that stops at a problem gives the rows
/// before it and `complete: false`.
pub fn describe_sheet<R: Read + Seek>(
    reader: R,
    request: &SheetRequest,
    limits: &HostLimits,
) -> View<Grid> {
    describe(reader, request, limits, &mut TempFiles::new())
}

/// [`describe_sheet`] of the workbook at `path`. A file that cannot be
/// opened, and a path that is not a regular file, are the view's `error`
/// (`io`), as for [`verify_path`](crate::verify_path).
pub fn describe_sheet_path(path: &Path, request: &SheetRequest, limits: &HostLimits) -> View<Grid> {
    match open_regular(path) {
        Ok(reader) => describe_sheet(reader, request, limits),
        Err(failure) => View::finished(Grid::default(), Diagnostics::new(), Some(failure)),
    }
}

/// [`describe_sheet`] of the workbook `bytes` holds, without copying it.
pub fn describe_sheet_bytes(
    bytes: &[u8],
    request: &SheetRequest,
    limits: &HostLimits,
) -> View<Grid> {
    describe_sheet(Cursor::new(bytes), request, limits)
}

/// [`describe_sheet`] with the factory of the spools the shared strings
/// move to, which a test makes fail.
pub(crate) fn describe<R: Read + Seek>(
    reader: R,
    request: &SheetRequest,
    limits: &HostLimits,
    spools: &mut dyn SpoolFactory,
) -> View<Grid> {
    let mut diagnostics = Diagnostics::with_limit(limits.max_diagnostics);
    let opened = Book::open(
        reader,
        &limits.verify.package,
        &limits.read_limits(),
        spools,
        &mut diagnostics,
    );
    let book = match opened {
        Ok(Some(book)) => book,
        Ok(None) => return stopped(Grid::default(), diagnostics, Refusal::WorkbookUnreadable),
        Err(error) => return View::finished(Grid::default(), diagnostics, Some(failed(error))),
    };
    let choice = match &request.sheet {
        Selection::First => SheetChoice::Index(0),
        Selection::Name(name) => SheetChoice::Name(name),
        Selection::Index(index) => SheetChoice::Index(*index),
    };
    let (index, sheet) = match book.choose(choice) {
        Ok(chosen) => chosen,
        Err(refusal) => return stopped(Grid::default(), diagnostics, refusal),
    };
    let mut grid = Grid {
        sheet: Some(GridSheet::of(index, sheet)),
        ..Grid::default()
    };
    let rows = match book.open_rows(index) {
        Ok(rows) => rows,
        Err(refusal) => return stopped(grid, diagnostics, refusal),
    };
    let error = read_rows(rows, request, &mut diagnostics, &mut grid);
    View::finished(grid, diagnostics, error)
}

/// Reads the rows of the sheet into `grid`, pushing the problem that stops
/// them to `diagnostics`; the failure of the host that stops them, if one
/// does, is returned, with the rows read before it kept.
fn read_rows<R: Read + Seek>(
    mut rows: SheetRows<R>,
    request: &SheetRequest,
    diagnostics: &mut Diagnostics,
    grid: &mut Grid,
) -> Option<Failure> {
    let mut collector = Collector::new(request);
    let mut row = Row::new();
    let error = loop {
        match rows.next_row(&mut row, diagnostics) {
            Ok(true) => collector.row(&row),
            Ok(false) => break None,
            Err(error) => break Some(failed(error)),
        }
    };
    let collected = collector.finish();
    rows.push_merge_limit(diagnostics);
    grid.dimension = rows.dimension().map(Echo::new);
    let mut overlapping = (rows.merges().iter())
        .filter(|merged| (collected.shown).is_some_and(|shown| shown.overlaps(**merged)));
    grid.merges = (overlapping.by_ref().take(SheetRequest::MAX_MERGES))
        .map(|merged| Span(*merged))
        .collect();
    grid.merges_over_max = overlapping.next().is_some();
    let merges_cut = rows.merges_cut() || grid.merges_over_max;
    grid.used = collected.used.map(Span);
    grid.shown = collected.shown.map(Span);
    grid.truncated = Truncated {
        merges: merges_cut,
        ..collected.truncated
    };
    grid.rows = collected.rows;
    grid.rows_below = collected.rows_below;
    grid.complete = rows.complete();
    error
}

/// The view of a run that `refusal` stopped, with `grid` as far as it got.
fn stopped(grid: Grid, diagnostics: Diagnostics, refusal: Refusal) -> View<Grid> {
    View::finished(grid, diagnostics, Some(refused(refusal)))
}

/// The failure a [`Refusal`] is. Its names are the workbook's or the
/// caller's, and the message goes back to the caller (the MCP server returns
/// it), so each is written as an [`Echo`] writes it first.
fn refused(refusal: Refusal) -> Failure {
    let echoed = |name: String| Echo::new(&name).as_str().to_owned();
    let refusal = match refusal {
        Refusal::NoSuchName(name) => Refusal::NoSuchName(echoed(name)),
        Refusal::Duplicate(name) => Refusal::Duplicate(echoed(name)),
        Refusal::NotWorksheet { name, kind } => Refusal::NotWorksheet {
            name: echoed(name),
            kind,
        },
        Refusal::SheetUnreadable(name) => Refusal::SheetUnreadable(echoed(name)),
        other => other,
    };
    Failure::new(refusal.code(), refusal.to_string())
}

/// The `io` failure of the host that `error` is.
fn failed(error: ReadError) -> Failure {
    match error {
        ReadError::Package(PackageError::Io(error)) => {
            Failure::io(format!("the package could not be read: {error}"))
        }
        ReadError::Spool(error) => Failure::io(format!(
            "the shared strings could not be moved to a temporary file: {error}"
        )),
    }
}

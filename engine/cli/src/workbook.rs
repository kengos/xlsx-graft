//! What every command that reads a workbook shares: opening it with the
//! same limits, and the JSON of why it read no sheet ([`Refused`]).

use crate::CliError;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::Limits;
use xlsx_graft_read::{Book, ReadLimits, Refusal};
use xlsx_graft_spool::TempFiles;

/// The workbook at `path`; `None` when it is no workbook, which
/// `diagnostics` then says why.
pub(crate) fn open(
    path: &Path,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Book<BufReader<File>>>, CliError> {
    let file = File::open(path).map_err(|source| CliError::Open {
        path: path.to_owned(),
        source,
    })?;
    Ok(Book::open(
        BufReader::new(file),
        &Limits::default(),
        &ReadLimits::default(),
        &mut TempFiles::new(),
        diagnostics,
    )?)
}

/// A [`Refusal`] as the JSON prints it: `{"code": ..., "message": ...}`.
#[derive(Debug)]
pub(crate) struct Refused(pub(crate) Refusal);

impl Serialize for Refused {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("code", self.0.code())?;
        map.serialize_entry("message", &self.0.to_string())?;
        map.end()
    }
}

#[cfg(test)]
mod tests;

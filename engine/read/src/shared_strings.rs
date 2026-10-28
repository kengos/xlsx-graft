//! [`SharedStrings`]: a workbook's shared strings table (ECMA-376-1:2016
//! §18.4), found by the workbook's relationship, read as a stream
//! (`sst.rs`) into a [`StringTable`] (`table.rs`), as the crate root says.

mod escape;
mod sst;
mod table;

use crate::{ReadError, ReadLimits, Workbook};
use std::io::{self, BufReader, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_package::{Package, PackageError, Presence};
use xlsx_graft_spool::{SpoolError, SpoolFactory};

pub use table::StringTable;

/// The `sharedStrings` relationship types, Transitional and Strict, of the
/// workbook's relationship to its shared strings table.
pub const SHARED_STRINGS: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings",
    "http://purl.oclc.org/ooxml/officeDocument/relationships/sharedStrings",
];

/// A workbook's shared strings table, as far as it is known.
#[derive(Debug)]
pub enum SharedStrings {
    /// The workbook has no shared strings table, and that is known.
    Absent,
    /// Whether the workbook has one, or what it holds, is not known, for a
    /// reason already reported. A string looked up in it is not a problem
    /// of its own.
    Unknown,
    /// The table, read whole.
    Table(StringTable),
}

/// What looking a string up found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Lookup {
    /// The string is there.
    Found,
    /// The table holds no string at that index, or there is no table.
    OutOfRange,
    /// The table is [`SharedStrings::Unknown`].
    Unknown,
}

/// Why reading the table stopped before its end.
#[derive(Debug)]
enum Halt {
    /// A problem in the part, to push unless the entry turns out damaged.
    Refused(Diagnostic),
    /// The reader of the part failed: the zip entry, which reported why
    /// when it stopped, or the host.
    Io(io::Error),
    /// The spool failed.
    Spool(SpoolError),
}

impl From<SpoolError> for Halt {
    fn from(error: SpoolError) -> Self {
        Self::Spool(error)
    }
}

impl SharedStrings {
    /// Reads the shared strings table of `workbook` from `package`, under
    /// `limits`, spilling to a spool from `spools`, and pushing each
    /// problem found.
    ///
    /// # Errors
    ///
    /// [`ReadError::Package`] when reading the package fails,
    /// [`ReadError::Spool`] when the spool does.
    pub fn read<R: Read + Seek>(
        package: &mut Package<R>,
        workbook: &Workbook,
        limits: &ReadLimits,
        spools: &mut dyn SpoolFactory,
        diagnostics: &mut Diagnostics,
    ) -> Result<Self, ReadError> {
        let index = match workbook.related(package, &SHARED_STRINGS, diagnostics) {
            Presence::Part(index) => index,
            Presence::Absent => return Ok(Self::Absent),
            Presence::Unknown => return Ok(Self::Unknown),
        };
        let location = (package.parts().get(index))
            .map(|part| Location::default().with_part(part.name().as_str()))
            .unwrap_or_default();
        let Some(entry) = package.open_part(index, diagnostics)? else {
            return Ok(Self::Unknown);
        };
        let mut entry = BufReader::new(entry);
        let refused = match sst::read(&mut entry, limits, spools) {
            Ok(table) => return Ok(Self::Table(table)),
            Err(Halt::Spool(error)) => return Err(error.into()),
            Err(Halt::Io(error)) if !entry.get_ref().is_stopped() => {
                return Err(PackageError::Io(error).into());
            }
            Err(Halt::Io(_)) => None,
            Err(Halt::Refused(diagnostic)) => Some(diagnostic),
        };
        // Read to the end, so a damaged entry reports itself in place of
        // what its damage made the part look like.
        let drained = io::copy(&mut entry, &mut io::sink());
        let stopped = entry.get_ref().is_stopped();
        drop(entry);
        if let Err(error) = drained
            && !stopped
        {
            return Err(PackageError::Io(error).into());
        }
        if let Some(diagnostic) = refused
            && !stopped
        {
            diagnostics.push(diagnostic, location);
        }
        Ok(Self::Unknown)
    }

    /// Puts the string at `index` in `out`, in place of what it held, as
    /// [`StringTable::get`] does. `out` is left empty unless it is found.
    ///
    /// # Errors
    ///
    /// When the spool fails, or memory for the string cannot be allocated.
    pub fn get(&mut self, index: u64, out: &mut String) -> Result<Lookup, ReadError> {
        out.clear();
        match self {
            Self::Absent => Ok(Lookup::OutOfRange),
            Self::Unknown => Ok(Lookup::Unknown),
            Self::Table(table) => Ok(table.get(index, out)?),
        }
    }
}

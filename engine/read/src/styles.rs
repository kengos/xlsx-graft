//! [`Styles`]: a workbook's styles (ECMA-376-1:2016 §18.8), found by the
//! workbook's relationship and read in memory (`sheet.rs`) into the
//! [`FormatKind`] of each cell format: the built-in number formats
//! (`kind.rs`) and those the part defines (`format.rs`), as the crate root
//! says.

mod format;
mod kind;
mod sheet;

use crate::{ReadError, ReadLimits, Workbook};
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostics, Location};
use xlsx_graft_package::{Package, Presence};

pub use kind::FormatKind;

/// The `styles` relationship types, Transitional and Strict, of the
/// workbook's relationship to its styles.
pub const STYLES: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles",
    "http://purl.oclc.org/ooxml/officeDocument/relationships/styles",
];

/// A workbook's styles, as far as they are known: the kind of each cell
/// format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Styles {
    /// The workbook has no styles, and that is known: every cell is
    /// General.
    Absent,
    /// Whether the workbook has styles, or what they hold, is not known,
    /// for a reason already reported.
    Unknown,
    /// The kind of each cell format (`cellXfs`), in order.
    Formats(Vec<FormatKind>),
}

impl Styles {
    /// Reads the styles of `workbook` from `package`, under `limits`,
    /// pushing each problem found.
    ///
    /// # Errors
    ///
    /// [`ReadError::Package`] when reading the package fails.
    pub fn read<R: Read + Seek>(
        package: &mut Package<R>,
        workbook: &Workbook,
        limits: &ReadLimits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Self, ReadError> {
        let index = match workbook.related(package, &STYLES, diagnostics) {
            Presence::Part(index) => index,
            Presence::Absent => return Ok(Self::Absent),
            Presence::Unknown => return Ok(Self::Unknown),
        };
        // Read whole, so the entry's CRC-32 is checked before a byte of the
        // XML is: a damaged entry reports itself alone.
        let Some(bytes) = package.read_xml_part(index, limits.xml.max_bytes, diagnostics)? else {
            return Ok(Self::Unknown);
        };
        let location = (package.parts().get(index))
            .map(|part| Location::default().with_part(part.name().as_str()))
            .unwrap_or_default();
        match sheet::read(&bytes, limits) {
            Ok(formats) => Ok(Self::Formats(formats)),
            Err(diagnostic) => {
                diagnostics.push(diagnostic, location);
                Ok(Self::Unknown)
            }
        }
    }

    /// The kind of the cell format at `index` (a cell's `s`): General when
    /// there are no styles, and [`FormatKind::Unknown`] when they are not
    /// known or hold no format at `index`.
    #[must_use]
    pub fn kind(&self, index: u64) -> FormatKind {
        match self {
            Self::Absent => FormatKind::General,
            Self::Unknown => FormatKind::Unknown,
            Self::Formats(formats) => usize::try_from(index)
                .ok()
                .and_then(|index| formats.get(index))
                .copied()
                .unwrap_or(FormatKind::Unknown),
        }
    }
}

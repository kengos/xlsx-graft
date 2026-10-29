//! [`WorkbookPart`]: what the workbook part (ECMA-376-1:2016 §18.2) says
//! of the workbook, read in memory: its sheets, in order, each with the
//! part its relationship targets (`link.rs`), and its date system. The
//! walk of the part's XML is `walk.rs`.

mod link;
mod walk;

use crate::{ReadError, ReadLimits, Workbook};
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostics, Location};
use xlsx_graft_package::Package;

/// What the workbook part says of the workbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookPart {
    sheets: Vec<Sheet>,
    date_system: DateSystem,
}

/// One sheet of the workbook (`sheets/sheet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    name: String,
    sheet_id: u32,
    state: SheetState,
    part: Option<SheetPart>,
}

/// Whether a sheet shows (ST_SheetState).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetState {
    /// It shows, the default.
    Visible,
    /// It is hidden, and the user may show it.
    Hidden,
    /// It is hidden, and only a program may show it.
    VeryHidden,
}

/// The part a sheet is, and its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheetPart {
    index: usize,
    kind: SheetKind,
}

/// What kind of sheet a part is, as its relationship's type says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetKind {
    /// A worksheet: cells (§12.3.24).
    Worksheet,
    /// A chartsheet: one chart (§12.3.2).
    Chartsheet,
    /// A dialogsheet: an old dialog box (§12.3.7).
    Dialogsheet,
    /// An Excel 4.0 macro sheet, international or not ([MS-OFFMACRO2]
    /// 2.2.1.1 and 2.2.1.2).
    Macrosheet,
}

/// How a serial date-time counts days (§18.17.4.1), as Excel takes it
/// (MS-OI29500 on §18.2.28, note j): `date1904` decides between 1900 and
/// 1904, and a workbook whose `conformance` is `strict` counts 1900 as the
/// standard has it. `dateCompatibility`, which Excel ignores, is not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateSystem {
    /// Day 1 is 1900-01-01, and day 60 the 1900-02-29 that never was, as
    /// in Lotus 1-2-3: the default.
    Compatibility1900,
    /// Day 0 is 1899-12-30: `conformance="strict"`.
    Base1900,
    /// Day 0 is 1904-01-01: `date1904`.
    Base1904,
}

impl WorkbookPart {
    /// Reads the part of `workbook` from `package`, under the XML limits of
    /// `limits`, pushing each problem found. `None` when the part cannot be
    /// read or is refused, which is pushed already.
    ///
    /// # Errors
    ///
    /// [`ReadError::Package`] when reading the package fails.
    pub fn read<R: Read + Seek>(
        package: &mut Package<R>,
        workbook: &Workbook,
        limits: &ReadLimits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Self>, ReadError> {
        let index = workbook.part();
        // Read whole, so the entry's CRC-32 is checked before a byte of the
        // XML is: a damaged entry reports itself alone.
        let Some(bytes) = package.read_xml_part(index, limits.xml.max_bytes, diagnostics)? else {
            return Ok(None);
        };
        let location = (package.parts().get(index))
            .map(|part| Location::default().with_part(part.name().as_str()))
            .unwrap_or_default();
        match walk::read(&bytes, limits) {
            Ok(read) => {
                let sheets = link::link(package, workbook, read.sheets, &location, diagnostics);
                Ok(Some(Self {
                    sheets,
                    date_system: read.date_system,
                }))
            }
            Err(diagnostic) => {
                diagnostics.push(diagnostic, location);
                Ok(None)
            }
        }
    }

    /// The sheets, in the order the part lists them.
    #[must_use]
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }

    /// How the workbook counts days.
    #[must_use]
    pub fn date_system(&self) -> DateSystem {
        self.date_system
    }
}

impl Sheet {
    /// Its name, its `_xHHHH_` escapes undone.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its `sheetId`.
    #[must_use]
    pub fn sheet_id(&self) -> u32 {
        self.sheet_id
    }

    /// Whether it shows.
    #[must_use]
    pub fn state(&self) -> SheetState {
        self.state
    }

    /// Its part, or `None` when which part it is is not known, for a
    /// reason already reported.
    #[must_use]
    pub fn part(&self) -> Option<SheetPart> {
        self.part
    }
}

impl SheetPart {
    /// The index of the part in
    /// [`Package::parts`](xlsx_graft_package::Package::parts).
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// What kind of sheet it is.
    #[must_use]
    pub fn kind(&self) -> SheetKind {
        self.kind
    }
}

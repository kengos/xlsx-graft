//! A row of the table: one function or feature, how each application
//! supports it, what else it needs and where that was read.

use crate::app::{Excel, ExcelMac, GoogleSheets, LibreOffice, Numbers, OnlyOffice};
use crate::entry::Entry;
use crate::table::Feature;

/// One function or feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Row {
    /// A function's name in upper case without its `_xlfn.` or `_xlws.`
    /// prefix (`XLOOKUP`, `FILTER`), or a feature's name
    /// ([`Feature::as_str`]).
    pub name: &'static str,
    /// How each application supports it.
    pub support: Support,
    /// The features it needs as well: an application that lacks one of
    /// them lacks this too, as a function that returns an array needs
    /// dynamic arrays to spill.
    pub requires: &'static [Feature],
    /// Where each cell was read: the URLs of the sources, never empty.
    pub sources: &'static [&'static str],
}

/// How each application supports one function or feature: one column per
/// application, each typed by that application's versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Support {
    /// Excel for Windows.
    pub excel: Entry<Excel>,
    /// Excel for Mac.
    pub excel_mac: Entry<ExcelMac>,
    /// LibreOffice.
    pub libreoffice: Entry<LibreOffice>,
    /// Google Sheets: added in [`GoogleSheets::Current`] when it has it.
    pub google_sheets: Entry<GoogleSheets>,
    /// Numbers.
    pub numbers: Entry<Numbers>,
    /// ONLYOFFICE.
    pub onlyoffice: Entry<OnlyOffice>,
}

impl Support {
    /// The columns in the table's order.
    pub(crate) const fn new(
        excel: Entry<Excel>,
        excel_mac: Entry<ExcelMac>,
        libreoffice: Entry<LibreOffice>,
        google_sheets: Entry<GoogleSheets>,
        numbers: Entry<Numbers>,
        onlyoffice: Entry<OnlyOffice>,
    ) -> Self {
        Self {
            excel,
            excel_mac,
            libreoffice,
            google_sheets,
            numbers,
            onlyoffice,
        }
    }

    /// The same columns with Excel for Mac's replaced.
    pub(crate) const fn with_excel_mac(self, excel_mac: Entry<ExcelMac>) -> Self {
        Self { excel_mac, ..self }
    }

    /// The same columns with LibreOffice's replaced.
    pub(crate) const fn with_libreoffice(self, libreoffice: Entry<LibreOffice>) -> Self {
        Self {
            libreoffice,
            ..self
        }
    }

    /// The same columns with Google Sheets' replaced.
    pub(crate) const fn with_google_sheets(self, google_sheets: Entry<GoogleSheets>) -> Self {
        Self {
            google_sheets,
            ..self
        }
    }

    /// The same columns with Numbers' replaced.
    pub(crate) const fn with_numbers(self, numbers: Entry<Numbers>) -> Self {
        Self { numbers, ..self }
    }

    /// The same columns with ONLYOFFICE's replaced.
    pub(crate) const fn with_onlyoffice(self, onlyoffice: Entry<OnlyOffice>) -> Self {
        Self { onlyoffice, ..self }
    }
}

//! The applications the table knows, and each one's versions in order.
//!
//! Each application has a version enum of its own, so a table row that
//! writes a LibreOffice version in the Excel column, or a version an
//! application never had, does not compile. A version enum's variants are
//! declared oldest first, and its `Ord` is that order.
//!
//! A list starts at the oldest version the table judges. A feature that an
//! application had before that is written as added in the list's first
//! version, and the row's sources say when it really came.

/// An application a workbook may be opened in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum App {
    /// Excel for Windows.
    Excel,
    /// Excel for Mac.
    ExcelMac,
    /// LibreOffice Calc.
    LibreOffice,
    /// Google Sheets, which has no versions: only the current one.
    GoogleSheets,
    /// Apple Numbers for Mac.
    Numbers,
    /// The ONLYOFFICE spreadsheet editor.
    OnlyOffice,
}

impl App {
    /// Every application, in the table's column order.
    pub const ALL: &'static [App] = &[
        App::Excel,
        App::ExcelMac,
        App::LibreOffice,
        App::GoogleSheets,
        App::Numbers,
        App::OnlyOffice,
    ];

    /// The name a template's `targets:` writes the application by.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            App::Excel => "excel",
            App::ExcelMac => "excel-mac",
            App::LibreOffice => "libreoffice",
            App::GoogleSheets => "google-sheets",
            App::Numbers => "numbers",
            App::OnlyOffice => "onlyoffice",
        }
    }

    /// Whether the application has versions: every one but Google Sheets,
    /// which a template's `targets:` names alone.
    #[must_use]
    pub fn has_versions(self) -> bool {
        self != App::GoogleSheets
    }

    /// The application's versions as written, oldest first.
    #[must_use]
    pub fn version_names(self) -> &'static [&'static str] {
        match self {
            App::Excel => Excel::NAMES,
            App::ExcelMac => ExcelMac::NAMES,
            App::LibreOffice => LibreOffice::NAMES,
            App::GoogleSheets => GoogleSheets::NAMES,
            App::Numbers => Numbers::NAMES,
            App::OnlyOffice => OnlyOffice::NAMES,
        }
    }
}

/// A version of one application.
pub trait Version: Copy + Ord + std::fmt::Debug + 'static {
    /// The application the versions belong to.
    const APP: App;
    /// Every version, oldest first.
    const ALL: &'static [Self];
    /// Every version as written, in the order of [`Version::ALL`].
    const NAMES: &'static [&'static str];

    /// The version as a template's `targets:` writes it.
    fn as_str(self) -> &'static str;
}

/// Declares one version enum per application.
macro_rules! versions {
    ($(
        $(#[$meta:meta])*
        $ty:ident for $app:ident { $( $variant:ident => $name:literal ),* $(,)? }
    )*) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $ty {
            $( #[doc = concat!("`", $name, "`")] $variant, )*
        }

        impl Version for $ty {
            const APP: App = App::$app;
            const ALL: &'static [Self] = &[ $( Self::$variant, )* ];
            const NAMES: &'static [&'static str] = &[ $( $name, )* ];

            fn as_str(self) -> &'static str {
                match self { $( Self::$variant => $name, )* }
            }
        }
    )*};
}

versions! {
    /// A version of Excel for Windows. `365` is the subscription as it is
    /// today, newer than every perpetual version; `2021` is the LTSC
    /// version, which lacks some features the retail 2021 has.
    Excel for Excel {
        V2007 => "2007", V2010 => "2010", V2013 => "2013", V2016 => "2016",
        V2019 => "2019", V2021 => "2021", V2024 => "2024", V365 => "365",
    }

    /// A version of Excel for Mac, `365` as for Windows.
    ExcelMac for ExcelMac {
        V2016 => "2016", V2019 => "2019", V2021 => "2021", V2024 => "2024", V365 => "365",
    }

    /// A version of LibreOffice, from the first release of 2024 to 26.8,
    /// the newest stable release in November 2026.
    LibreOffice for LibreOffice {
        V24_2 => "24.2", V24_8 => "24.8", V25_2 => "25.2", V25_8 => "25.8", V26_2 => "26.2",
        V26_8 => "26.8",
    }

    /// Google Sheets, which is only ever the current one.
    GoogleSheets for GoogleSheets {
        Current => "current",
    }

    /// A version of Numbers for Mac: every one Apple's version history for
    /// Numbers 14.4 lists, read on 2026-11-04. A later version is added
    /// with the source that names it.
    Numbers for Numbers {
        V11_0 => "11.0", V11_1 => "11.1", V11_2 => "11.2",
        V12_0 => "12.0", V12_1 => "12.1", V12_2 => "12.2",
        V13_0 => "13.0", V13_1 => "13.1", V13_2 => "13.2",
        V14_0 => "14.0", V14_1 => "14.1", V14_2 => "14.2", V14_3 => "14.3", V14_4 => "14.4",
    }

    /// A version of ONLYOFFICE Docs, from the first release of 2024 as for
    /// LibreOffice: every minor release its changelog lists since, 10.0
    /// the newest in November 2026.
    OnlyOffice for OnlyOffice {
        V8_0 => "8.0", V8_1 => "8.1", V8_2 => "8.2", V8_3 => "8.3",
        V9_0 => "9.0", V9_1 => "9.1", V9_2 => "9.2", V9_3 => "9.3", V9_4 => "9.4",
        V10_0 => "10.0",
    }
}

#[cfg(test)]
mod tests;

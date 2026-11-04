//! The reasons of the template's `targets:` and `bestEffort:` lists: the
//! list an entry is in, why an entry names no versions, and what a version
//! that lacks a function or feature shows.

use super::{ArgValue, Serialize, Serializer, fmt};

reasons! {
    /// The template list a target entry is written in. Its wire string is
    /// snake_case like every reason's; the message names the list as the
    /// template spells it (`bestEffort`).
    TargetList {
        /// `targets:`, the applications the workbook must work in.
        Targets => "targets",
        /// `bestEffort:`, the applications it should work in.
        BestEffort => "best_effort",
    }

    /// Why a template's `targets:` or `bestEffort:` entry names no versions
    /// the engine can check against: an entry is `name`, `name version` or
    /// `name OP version`, OP one of `>=`, `>`, `<=` and `<`.
    TargetReason {
        /// The entry is none of the three forms: it is empty, has no name,
        /// an operator other than the four (`=`, `<>`), an operator with no
        /// version, or more after the version.
        Malformed => "malformed",
        /// The name is no application the engine knows.
        UnknownApp => "unknown_app",
        /// The version is not one the engine knows the application by: a
        /// later one, or one older than the oldest it checks.
        UnknownVersion => "unknown_version",
        /// The entry names an application that has versions, but no version.
        VersionRequired => "version_required",
        /// The entry gives a version, with or without an operator, to an
        /// application that has no versions (Google Sheets), which is
        /// written alone.
        VersionNotAllowed => "version_not_allowed",
        /// The operator and version hold none of the versions the engine
        /// knows, such as `excel < 2007`.
        Empty => "empty",
        /// A `bestEffort:` entry holds a version a `targets:` entry holds
        /// too, which leaves it both required and best effort.
        Overlap => "overlap",
    }

    /// An application a template's `targets:` or `bestEffort:` names. Its
    /// wire string is snake_case like every reason's; the message names it
    /// as the template spells it (`excel-mac`).
    TargetApp {
        /// Excel for Windows, `excel`.
        Excel => "excel",
        /// Excel for Mac, `excel-mac`.
        ExcelMac => "excel_mac",
        /// LibreOffice, `libreoffice`.
        LibreOffice => "libreoffice",
        /// Google Sheets, `google-sheets`.
        GoogleSheets => "google_sheets",
        /// Numbers, `numbers`.
        Numbers => "numbers",
        /// ONLYOFFICE, `onlyoffice`.
        OnlyOffice => "onlyoffice",
    }

    /// What a version that lacks a function or feature shows instead, as
    /// the compatibility table's sources say: the table's vocabulary, and
    /// `unknown` where no source says.
    TargetFallback {
        /// No source the table cites says.
        Unknown => "unknown",
        /// Skipped and not shown, as an unknown extension is.
        Ignored => "ignored",
        /// Only the value saved with the cell is shown.
        ValueOnly => "value_only",
        /// The formula evaluates to `#NAME?`.
        NameError => "name_error",
        /// The cell shows `#VALUE!`.
        ValueError => "value_error",
        /// A legacy array formula of a fixed size.
        LegacyArray => "legacy_array",
        /// A legacy note (comment).
        AsNote => "as_note",
        /// A shape that stands in for the object.
        PlaceholderShape => "placeholder_shape",
        /// Part of it works.
        Degraded => "degraded",
        /// The application offers to repair the workbook.
        RepairPrompt => "repair_prompt",
        /// The workbook does not open.
        Unopenable => "unopenable",
    }
}

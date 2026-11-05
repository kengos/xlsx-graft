//! The macros the table's data is written with, and the short helpers its
//! rows use. The rows themselves are in `table/functions.rs` and
//! `table/features.rs`; the URLs they cite in `table/sources.rs`.
//!
//! A function's row:
//!
//! ```text
//! "TEXTJOIN" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), on(O::V8_0)),
//!     requires: [], sources: S2019, safety: unreviewed();
//! ```
//!
//! The six cells are the columns of [`Support`] in order, each typed by its
//! application's versions. `safety:` is the function's [`Safety`]:
//! `safe(review)`, `dangerous(Danger::Network)` or `unreviewed()`. A
//! function the table does not check against a template's targets yet
//! goes in the `unchecked { ... }` block at the end, with its safety only:
//! `"SUM" => safety: unreviewed();`. A feature's row starts with its
//! variant and its name instead: `DynamicArray "dynamic_array" => s(...),
//! requires: [], sources: &[...];`, and the rows declare [`Feature`]
//! itself.

use crate::app::{Excel, ExcelMac, GoogleSheets, LibreOffice, Numbers, OnlyOffice, Version};
use crate::entry::{Entry, Fallback};
use crate::row::Support;
use crate::safety::{Danger, Dangerous, Review, Safe, Safety};

/// Expands the function rows into the `FUNCTIONS` slice, and every
/// function's safety, the `unchecked` block's included, into `SAFETY`.
macro_rules! functions {
    (
        $(
            $name:literal => $support:expr,
                requires: [$($requires:ident),* $(,)?],
                sources: $sources:expr,
                safety: $safety:expr;
        )*
        unchecked {
            $( $unchecked:literal => safety: $unchecked_safety:expr; )*
        }
    ) => {
        /// Every function the table checks against a template's targets,
        /// in the table's order.
        pub(crate) static FUNCTIONS: &[Row] = &[$(
            Row {
                name: $name,
                support: $support,
                requires: &[$(Feature::$requires),*],
                sources: $sources,
            },
        )*];

        /// Every function's safety: those of `FUNCTIONS` in its order,
        /// then those of the `unchecked` block.
        pub(crate) static SAFETY: &[SafetyRow] = &[
            $( SafetyRow { name: $name, safety: $safety }, )*
            $( SafetyRow { name: $unchecked, safety: $unchecked_safety }, )*
        ];
    };
}

/// Expands the feature rows into [`Feature`] and its rows.
macro_rules! features {
    ($(
        $(#[$meta:meta])*
        $variant:ident $name:literal => $support:expr,
            requires: [$($requires:ident),* $(,)?],
            sources: $sources:expr;
    )*) => {
        /// A feature of a workbook that not every application has. Its order
        /// is the order of the rows, which orders what is reported of them.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Feature {
            $( $(#[$meta])* $variant, )*
        }

        impl Feature {
            /// Every feature, in the table's order.
            pub const ALL: &'static [Feature] = &[ $( Feature::$variant, )* ];

            /// The stable snake_case name.
            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self { $( Feature::$variant => $name, )* }
            }

            /// The feature's row.
            #[must_use]
            pub fn row(self) -> &'static Row {
                match self { $( Feature::$variant => {
                    static ROW: Row = Row {
                        name: $name,
                        support: $support,
                        requires: &[$(Feature::$requires),*],
                        sources: $sources,
                    };
                    &ROW
                } )* }
            }
        }
    };
}

// After the macros: a `macro_rules!` is in scope only below its definition.
mod features;
mod functions;
mod sources;

pub use features::Feature;
pub(crate) use functions::{FUNCTIONS, SAFETY};

/// A row's six cells, in column order.
const fn s(
    excel: Entry<Excel>,
    excel_mac: Entry<ExcelMac>,
    libreoffice: Entry<LibreOffice>,
    google_sheets: Entry<GoogleSheets>,
    numbers: Entry<Numbers>,
    onlyoffice: Entry<OnlyOffice>,
) -> Support {
    Support::new(
        excel,
        excel_mac,
        libreoffice,
        google_sheets,
        numbers,
        onlyoffice,
    )
}

/// Added in `version`.
const fn on<V: Version>(version: V) -> Entry<V> {
    Entry::added(version)
}

/// An Excel function added in `version`: in an older Excel the formula
/// evaluates to `#NAME?`, shown with its `_xlfn.` prefix where Excel
/// writes one.
const fn xl<V: Version>(version: V) -> Entry<V> {
    Entry::added(version).fallback(Fallback::NameError)
}

/// Present in `version`, not known before it.
const fn le<V: Version>(version: V) -> Entry<V> {
    Entry::at_most(version)
}

/// Missing through `missing`, present from `present` on.
const fn between<V: Version>(missing: V, present: V) -> Entry<V> {
    Entry::between(missing, present)
}

/// In no version.
const fn no<V: Version>() -> Entry<V> {
    Entry::never()
}

/// Not known.
const fn unk<V: Version>() -> Entry<V> {
    Entry::unknown()
}

/// Google Sheets has it.
const YES: Entry<GoogleSheets> = Entry::added(GoogleSheets::Current);

/// Reviewed safe by `review`, with no denial scheduled.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the rows are reviewed in a later change")
)]
const fn safe(review: Review) -> Safety {
    Safety::ReviewedSafe(Safe {
        review,
        scheduled: None,
    })
}

/// Dangerous for `danger`, denied by every version of xlsx-graft.
const fn dangerous(danger: Danger) -> Safety {
    Safety::Dangerous(Dangerous {
        danger,
        denied_since: None,
        advisory: None,
    })
}

/// Not reviewed yet.
const fn unreviewed() -> Safety {
    Safety::Unreviewed
}

#[cfg(test)]
mod safety_tests;
#[cfg(test)]
mod tests;

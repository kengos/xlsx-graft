//! The embedded pages: every file in the crate's `pages/` directory,
//! compiled in with `include_str!`. A binary that links this crate reads
//! no page file at run time, so a reference URI never becomes a file path.
//!
//! `crate::tests` compares this table with the `pages/` directory in both
//! directions, so a page added to one and not the other fails the tests,
//! and checks that [`crate::pages`] returns every page in this table.

/// One page's name (its file name without `.md`) and its markdown source.
pub(crate) struct PageSource {
    /// The file name without `.md`; the last segment of the page's URI.
    pub(crate) name: &'static str,
    /// The whole file: the front matter, then the markdown body.
    pub(crate) source: &'static str,
}

/// Embeds `pages/<name>.md`.
macro_rules! page {
    ($name:literal) => {
        PageSource {
            name: $name,
            source: include_str!(concat!("../pages/", $name, ".md")),
        }
    };
}

/// Every page, ordered by name. The list order is the order
/// [`crate::pages`] returns them in.
pub(crate) const PAGES: &[PageSource] = &[
    page!("diagnostics-content"),
    page!("diagnostics-host"),
    page!("diagnostics-package"),
    page!("diagnostics-targets"),
    page!("diagnostics-workbook"),
    page!("diagnostics-zip"),
    page!("reference"),
    page!("verify"),
    page!("version"),
];

//! [`FunctionTable`]: how the engine classifies a function a formula
//! calls, which the formula check judges it by.

/// How a [`FunctionTable`] classifies a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionClass<'a> {
    /// Reviewed and found safe: it neither reaches outside the workbook nor
    /// reads the machine it runs on.
    ReviewedSafe,
    /// Reviewed and found dangerous, for `reason`.
    Dangerous {
        /// Why, in English.
        reason: &'a str,
    },
    /// Known, but not reviewed yet.
    Unreviewed,
}

/// The table of functions the engine knows. A function it does not know
/// is denied, as is one it does not classify
/// [`ReviewedSafe`](FunctionClass::ReviewedSafe), unless the template
/// allows it.
pub trait FunctionTable {
    /// How the table classifies the function `name`, upper-cased as the
    /// formula check looks it up; `None` for a function it does not know.
    fn classify(&self, name: &str) -> Option<FunctionClass<'_>>;
}

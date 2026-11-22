//! How Excel writes a function's name in a formula it saves to a file.

/// How Excel writes a function's name in a formula it saves to a file, as
/// [MS-XLSX] 2.2.3 Functions lists it for a row of the table; a function
/// ECMA-376-1:2016 §18.17.7 defines is [`Prefix::Bare`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prefix {
    /// Written without a prefix (`ISO.CEILING`).
    Bare,
    /// Written after `_xlfn.` (`_xlfn.XLOOKUP`).
    Xlfn,
    /// Written after `_xlfn._xlws.` (`_xlfn._xlws.SORT`).
    XlfnXlws,
    /// [MS-XLSX] 2.2.3 Functions does not list the function, so the table does not say.
    Unknown,
}

impl Prefix {
    /// Every prefix, in the order the variants are declared.
    pub const ALL: &'static [Prefix] = &[
        Prefix::Bare,
        Prefix::Xlfn,
        Prefix::XlfnXlws,
        Prefix::Unknown,
    ];

    /// The text written before the function's name: empty for
    /// [`Prefix::Bare`], `None` for [`Prefix::Unknown`].
    #[must_use]
    pub fn as_str(self) -> Option<&'static str> {
        match self {
            Prefix::Bare => Some(""),
            Prefix::Xlfn => Some("_xlfn."),
            Prefix::XlfnXlws => Some("_xlfn._xlws."),
            Prefix::Unknown => None,
        }
    }
}

#[cfg(test)]
mod tests;

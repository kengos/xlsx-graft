//! How safe each function is to accept in a workbook xlsx-graft fills:
//! reviewed safe, dangerous for a stated kind of reason, or not reviewed
//! yet. The table's rule is that verify accepts only a reviewed-safe
//! function by default, and denies every other function and every name
//! [`crate::function_safety`] gives `None` for.

/// One function's entry in the safety column of the function table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SafetyRow {
    /// The function's name in upper case without its `_xlfn.` or `_xlws.`
    /// prefix, as [`crate::Row::name`] writes it.
    pub name: &'static str,
    /// How safe the function is.
    pub safety: Safety,
}

/// How safe a function is to accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Safety {
    /// The review read the pages the function's row in the function table
    /// cites, and found that the function's result depends only on values
    /// in workbooks, the clock, random numbers and the application's locale
    /// settings. Values in workbooks are, for example, the function's
    /// arguments, the cells a reference points to, the values of the
    /// defined names the formula uses, the sheets of the workbook that
    /// holds the formula (SHEETS) and the position of the cell that holds
    /// the formula (ROW and COLUMN without arguments). NOW and TODAY read
    /// the clock, RAND returns random numbers, and the locale settings
    /// change how a function such as TEXT or VALUE writes numbers and dates
    /// as text or reads them from text.
    ///
    /// The function does none of the things a [`Danger`] names: it does not
    /// use a service on the network ([`Danger::Network`]), read data from a
    /// source other than workbooks, such as a real-time data server or an
    /// OLAP cube ([`Danger::ExternalData`]), run code outside the workbook
    /// ([`Danger::ExternalCode`]), return information about the machine
    /// that computes it or the path of the workbook's file
    /// ([`Danger::MachineInfo`]), or make its cell open, when clicked, a
    /// link whose target the formula computes ([`Danger::OpensLink`]).
    ///
    /// The review does not judge which cells a reference points to, nor
    /// whether the cells are in another workbook: verify judges the
    /// reference.
    ReviewedSafe(Safe),
    /// It is denied for the kind of [`Danger`] given.
    Dangerous(Dangerous),
    /// Nobody has reviewed it yet, or the review could not settle it.
    Unreviewed,
}

/// What a reviewed-safe function's review read, and whether a later
/// version of xlsx-graft is to deny it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Safe {
    /// The review that found it safe.
    pub review: Review,
    /// Set when a later version of xlsx-graft is to deny the function:
    /// until that version, the function is accepted with a notice.
    pub scheduled: Option<Scheduled>,
}

/// The review that found a function safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Review {
    /// The day the definition was read, as `YYYY-MM-DD`.
    pub on: &'static str,
    /// The Excel for Microsoft 365 current in the month the definition was
    /// read, as `Excel 365, YYYY-MM`. The field does not say that a formula
    /// was computed in that Excel.
    pub version: &'static str,
    /// The definitions read, never empty. A function ECMA-376-1:2016
    /// §18.17.7 defines cites its own section
    /// (`ECMA-376-1:2016 §18.17.7.306` for SUM); any function may also
    /// cite pages at `support.microsoft.com` or `learn.microsoft.com`.
    pub sources: &'static [&'static str],
}

/// The version of xlsx-graft that is to deny a reviewed-safe function,
/// and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scheduled {
    /// The version of xlsx-graft that is to deny it, as `MAJOR.MINOR.PATCH`.
    /// The table's checks keep it after the crate's own version: the
    /// change that raises the crate's version to `from` moves the row to
    /// [`Safety::Dangerous`] with `denied_since` set to `from`. A function
    /// with a [`Scheduled`] is therefore always accepted with a notice,
    /// whatever version runs.
    pub from: &'static str,
    /// The kind of danger found in the function: the [`Dangerous::danger`]
    /// it is to be denied for.
    pub danger: Danger,
}

impl Scheduled {
    /// A [`Scheduled`] saying that the version `from` of xlsx-graft is to
    /// deny a function for `danger`. The table's tests check that `from` is
    /// later than the crate's version only for the rows of the table;
    /// `Scheduled::new` does not check `from`.
    #[must_use]
    pub const fn new(from: &'static str, danger: Danger) -> Self {
        Self { from, danger }
    }
}

/// A dangerous function's reason, and when xlsx-graft began to deny a
/// function an older version let through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Dangerous {
    /// The kind of reason it is dangerous. A function that fits more than
    /// one kind takes the kind of the first thing it does when Excel
    /// computes it: PY sends the formula to Microsoft's service, which
    /// then runs it as code, so PY is [`Danger::Network`].
    pub danger: Danger,
    /// The version of xlsx-graft that began to deny it, as
    /// `MAJOR.MINOR.PATCH`; `None` when every version has denied it.
    pub denied_since: Option<&'static str>,
    /// The ID of the security advisory that announced the denial
    /// (`GHSA-xxxx-xxxx-xxxx`); set only with `denied_since`.
    pub advisory: Option<&'static str>,
}

/// The kind of reason a function is dangerous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Danger {
    /// It sends data to, or reads data from, a service on the network.
    Network,
    /// It reads data from a source other than workbooks, such as a
    /// real-time data server or an OLAP cube.
    ExternalData,
    /// It runs code outside the workbook: a procedure of a dynamic link
    /// library.
    ExternalCode,
    /// It returns information about the machine that computes it or the
    /// path of the workbook's file.
    MachineInfo,
    /// Clicking its cell opens a link whose target the formula computes.
    OpensLink,
}

impl Danger {
    /// Every kind, in declaration order.
    pub const ALL: &'static [Danger] = &[
        Danger::Network,
        Danger::ExternalData,
        Danger::ExternalCode,
        Danger::MachineInfo,
        Danger::OpensLink,
    ];

    /// The stable snake_case name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Danger::Network => "network",
            Danger::ExternalData => "external_data",
            Danger::ExternalCode => "external_code",
            Danger::MachineInfo => "machine_info",
            Danger::OpensLink => "opens_link",
        }
    }

    /// The reason in English, to follow "the function is dangerous:".
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Danger::Network => "it sends data to, or reads data from, a service on the network",
            Danger::ExternalData => "it reads data from a source outside the workbook",
            Danger::ExternalCode => "it runs code outside the workbook",
            Danger::MachineInfo => {
                "it returns information about the machine that computes it or the path of the workbook's file"
            }
            Danger::OpensLink => "clicking its cell opens a link whose target the formula computes",
        }
    }
}

#[cfg(test)]
mod tests;

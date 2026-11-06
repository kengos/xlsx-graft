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
    /// Its definition was read, and its result depends only on its
    /// arguments, the workbook's cells and names, the clock (NOW, TODAY),
    /// random numbers (RAND) and the application's locale settings (the
    /// decimal separator TEXT and VALUE use). None of these sends anything
    /// out of the workbook, and none names the machine or the file: those
    /// are [`Danger::MachineInfo`].
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
    /// It reads data from a source outside the workbook, such as a
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

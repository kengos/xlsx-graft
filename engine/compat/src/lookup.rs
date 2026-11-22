//! Looking up a function by the name a formula writes: what the table
//! knows of it, whichever of its three states it is in ([`Known`]), and its
//! [`Safety`].
//!
//! [`crate::function`] and [`crate::function_safety`] answer for a name
//! already written without its prefixes and say nothing of why they have no
//! answer; [`look_up`] takes the name as a formula writes it
//! (`_xlfn._xlws.SORT`) and gives everything the table holds of it.

use crate::prefix::Prefix;
use crate::row::Row;
use crate::safety::Safety;
use crate::table::UNDEFINED;
use crate::{function, function_safety};

/// The source of every function the table gives a safety but no row of
/// versions, other than the functions of the undefined block:
/// ECMA-376-1:2016 §18.17.7 defines them, and Excel writes them without a
/// prefix.
pub const ECMA_FUNCTIONS: &str = "ECMA-376-1:2016 §18.17.7";

/// What the table knows of a function name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Known {
    /// The table has a row of versions for it, and a safety.
    Versioned {
        /// The row of versions.
        row: &'static Row,
        /// The function's safety.
        safety: Safety,
    },
    /// The table gives the function a safety but no row of versions. The
    /// functions ECMA-376-1:2016 §18.17.7 defines are written without a
    /// prefix and cite [`ECMA_FUNCTIONS`]; the others (ANCHORARRAY, SINGLE,
    /// CALL, COPILOT and REGISTER.ID) have an unknown prefix and no source.
    Baseline {
        /// What the table says of the function besides its safety.
        baseline: Baseline,
        /// The function's safety.
        safety: Safety,
    },
    /// The table does not list the name; verify denies it.
    Unknown,
}

/// What the table says of a function it gives a safety but no row of
/// versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Baseline {
    /// How Excel writes the function's name in a file: [`Prefix::Bare`] for
    /// a function ECMA-376-1:2016 §18.17.7 defines, [`Prefix::Unknown`]
    /// for one it does not.
    pub prefix: Prefix,
    /// The specification that defines the function: [`ECMA_FUNCTIONS`], or
    /// none for a function ECMA-376-1:2016 §18.17.7 does not define.
    pub sources: &'static [&'static str],
}

/// A function name looked up in the table.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Lookup {
    /// The name without its leading `_xlfn.` and `_xlws.` prefixes, in
    /// upper case.
    pub name: String,
    /// What the table knows of the name.
    pub known: Known,
}

/// `name` without a leading `_xlfn.`, then without a leading `_xlws.`, each
/// matched in any ASCII case (`_xlfn._xlws.SORT` is `SORT`, `_xlws.SORT` is
/// `SORT`, `_XLFN.xlookup` is `xlookup`). Nothing else is removed: a
/// `_xlfn.` that follows `_xlws.` stays.
#[must_use]
pub fn unprefixed(name: &str) -> &str {
    strip(strip(name, "_xlfn."), "_xlws.")
}

/// `name` without a leading `prefix`, matched in any ASCII case.
fn strip<'a>(name: &'a str, prefix: &str) -> &'a str {
    // `split_at_checked` is `None` when the split is inside a character, as
    // when `name` is shorter than `prefix` or has a multi-byte character
    // there.
    match name.split_at_checked(prefix.len()) {
        Some((head, rest)) if head.eq_ignore_ascii_case(prefix) => rest,
        _ => name,
    }
}

/// Looks `name` up: the name is unprefixed ([`unprefixed`]) and put in upper
/// case, then [`function`] and [`function_safety`] are asked.
#[must_use]
pub fn look_up(name: &str) -> Lookup {
    let name = unprefixed(name).to_ascii_uppercase();
    // A row without a safety cannot happen: `functions!` builds `FUNCTIONS`
    // and `SAFETY` from the same rows, so the `_` arm matches only a name that
    // is in neither `FUNCTIONS` nor `SAFETY`.
    let known = match (function(&name), function_safety(&name)) {
        (Some(row), Some(safety)) => Known::Versioned { row, safety },
        (None, Some(safety)) => Known::Baseline {
            baseline: baseline(&name),
            safety,
        },
        _ => Known::Unknown,
    };
    Lookup { name, known }
}

/// The baseline entry of `name`, an upper-case name the table gives a safety
/// but no row.
fn baseline(name: &str) -> Baseline {
    if UNDEFINED.contains(&name) {
        Baseline {
            prefix: Prefix::Unknown,
            sources: &[],
        }
    } else {
        Baseline {
            prefix: Prefix::Bare,
            sources: &[ECMA_FUNCTIONS],
        }
    }
}

#[cfg(test)]
mod tests;

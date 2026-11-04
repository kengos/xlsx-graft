//! [`Versions`]: a set of one application's versions, held as bits.

use crate::Version;

/// A set of one application's versions: bit `i` is its `i`-th version,
/// oldest first. An application has at most 32 versions (the tests hold
/// every list to that); a position past 32 is never in the set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Versions(u32);

impl Versions {
    /// The versions from position `start` up to, but not including, `end`.
    pub(super) fn range(start: usize, end: usize) -> Self {
        Self((start..end).fold(0, |bits, at| bits | bit(at)))
    }

    /// Whether the set holds no version.
    pub(super) fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The versions both sets hold.
    pub(super) fn and(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// The versions either set holds.
    pub(super) fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether the set holds the version at position `at`.
    fn contains(self, at: usize) -> bool {
        self.0 & bit(at) != 0
    }

    /// The versions of `V` the set holds, oldest first.
    pub(super) fn iter<V: Version>(self) -> impl Iterator<Item = V> {
        V::ALL
            .iter()
            .enumerate()
            .filter(move |(at, _)| self.contains(*at))
            .map(|(_, version)| *version)
    }

    /// The set as written in a message: `names` are the application's
    /// versions, and those the set holds are joined by `, `.
    pub(super) fn names(self, names: &[&str]) -> String {
        let held: Vec<&str> = names
            .iter()
            .enumerate()
            .filter(|(at, _)| self.contains(*at))
            .map(|(_, name)| *name)
            .collect();
        held.join(", ")
    }
}

/// The bit of position `at`, or none past the 32nd.
fn bit(at: usize) -> u32 {
    u32::try_from(at)
        .ok()
        .and_then(|at| 1_u32.checked_shl(at))
        .unwrap_or(0)
}

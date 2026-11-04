//! One cell of the table: how one application supports one function or
//! feature, and what an older version shows instead.

use crate::app::Version;

mod fallback;

pub use fallback::Fallback;

/// When an application added a function or feature. The shape follows
/// MDN's browser-compat-data `version_added`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Added<V> {
    /// Added in this version, and missing from every older one.
    In(V),
    /// Present in this version; whether an older one has it is not known
    /// (browser-compat-data's `"≤v"`), as when a source lists only the
    /// versions it still covers.
    AtMost(V),
    /// Missing from the first version and every older one, present from
    /// the second on; the versions between are not known.
    Between(V, V),
    /// No version has it.
    Never,
    /// No source the table cites says.
    Unknown,
}

/// Whether one version of an application has a function or feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// It has it, perhaps in part ([`Entry::partial`]).
    Supported,
    /// It does not have it.
    Unsupported,
    /// No source the table cites says whether it has it.
    Unverified,
}

/// How one application supports one function or feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entry<V> {
    /// When it was added.
    pub added: Added<V>,
    /// The first version without it again, when one dropped it.
    pub removed: Option<V>,
    /// Whether the application's version differs from Excel's (its
    /// arguments, its results, what it writes back); the notes say how.
    pub partial: bool,
    /// What a version that lacks it shows, when a source says.
    pub fallback: Option<Fallback>,
    /// What the table's reader needs to know about this cell, in English.
    pub notes: Option<&'static str>,
}

impl<V: Version> Entry<V> {
    /// Added in `version`.
    #[must_use]
    pub const fn added(version: V) -> Self {
        Self::new(Added::In(version))
    }

    /// Present in `version`, not known before it.
    #[must_use]
    pub const fn at_most(version: V) -> Self {
        Self::new(Added::AtMost(version))
    }

    /// Missing through `missing`, present from `present` on, not known
    /// between.
    #[must_use]
    pub const fn between(missing: V, present: V) -> Self {
        Self::new(Added::Between(missing, present))
    }

    /// In no version.
    #[must_use]
    pub const fn never() -> Self {
        Self::new(Added::Never)
    }

    /// Not known.
    #[must_use]
    pub const fn unknown() -> Self {
        Self::new(Added::Unknown)
    }

    const fn new(added: Added<V>) -> Self {
        Self {
            added,
            removed: None,
            partial: false,
            fallback: None,
            notes: None,
        }
    }

    /// The same entry, dropped again from `version` on.
    #[must_use]
    pub const fn removed(self, version: V) -> Self {
        Self {
            removed: Some(version),
            ..self
        }
    }

    /// The same entry, with what a version that lacks it shows.
    #[must_use]
    pub const fn fallback(self, fallback: Fallback) -> Self {
        Self {
            fallback: Some(fallback),
            ..self
        }
    }

    /// The same entry, with a note.
    #[must_use]
    pub const fn note(self, notes: &'static str) -> Self {
        Self {
            notes: Some(notes),
            ..self
        }
    }

    /// The same entry, differing from Excel's as `notes` says.
    #[must_use]
    pub const fn partial(self, notes: &'static str) -> Self {
        Self {
            partial: true,
            notes: Some(notes),
            ..self
        }
    }

    /// Whether `version` has it.
    #[must_use]
    pub fn at(&self, version: V) -> Status {
        if self.removed.is_some_and(|removed| version >= removed) {
            return Status::Unsupported;
        }
        match self.added {
            Added::In(added) if version >= added => Status::Supported,
            Added::In(_) | Added::Never => Status::Unsupported,
            Added::AtMost(present) | Added::Between(_, present) if version >= present => {
                Status::Supported
            }
            Added::Between(missing, _) if version <= missing => Status::Unsupported,
            Added::AtMost(_) | Added::Between(..) | Added::Unknown => Status::Unverified,
        }
    }
}

#[cfg(test)]
mod tests;

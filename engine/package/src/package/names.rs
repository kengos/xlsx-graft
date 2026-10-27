//! [`map`]: the zip's entries mapped to part names (ECMA-376-2:2021 §7.3.5),
//! with the content types stream found among them.
//!
//! An entry is not a part when its name is the content types stream's, when
//! it is a folder (a name ending in `/` with no data; Annex B, Table B.3),
//! or when its name is not a part name (`entry_name_invalid`; 4th edition
//! M2.16). Entries whose names are the same part (`part_name_duplicate`)
//! are none of them parts: readers differ on which one they would take. A
//! part whose name is derived from another's (`part_name_derived`) is not a
//! part either; the shorter one is. The two content types streams of a zip
//! that has two are refused the same way.
//!
//! The derived name's base may itself be a name refused as a duplicate.
//! Each name that is not a part is kept as unknown, so what would look it
//! up does not report it missing: the key of a refused part name, or the
//! shadow key of an entry name that is not one (`part_name::shadow_key`).
//!
//! Nothing is reported at an entry that is not readable, whose own
//! diagnostic stands for it; its name still counts, so it can make others
//! duplicates and be the part another relationship targets.

use crate::part_name::shadow_key;
use crate::{Entry, PartName};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location, PartNameReason};

/// The zip entry name of the content types stream (§7.3.7).
const CONTENT_TYPES: &str = "[Content_Types].xml";

/// The shadow key of an entry that may have been the content types stream,
/// such as `/[Content_Types].xml`: `[` and `]` are percent-encoded.
const CONTENT_TYPES_KEY: &str = "/%5bcontent_types%5d.xml";

/// The content types stream's entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContentTypesEntry {
    /// No entry has its name.
    Missing,
    /// The entry at this index.
    At(usize),
    /// More than one entry has its name, or one that may have had it was
    /// refused, which is reported.
    Refused,
}

/// What the entries map to.
#[derive(Debug)]
pub(super) struct Names {
    /// Each part's name and entry index, in entry order.
    pub(super) parts: Vec<(PartName, usize)>,
    /// The keys of the names that are not parts because they were refused,
    /// and the shadow keys of the entry names that are not part names:
    /// whether a part of one of them is there is not known.
    pub(super) unknown: BTreeSet<String>,
    pub(super) content_types: ContentTypesEntry,
}

/// Maps `entries` to parts, pushing why an entry is not one.
pub(super) fn map(entries: &[Entry], diagnostics: &mut Diagnostics) -> Names {
    let mut named = Vec::new();
    let mut streams = Vec::new();
    let mut unknown = BTreeSet::new();
    let mut lost_stream = false;
    for (index, entry) in entries.iter().enumerate() {
        let refused = match std::str::from_utf8(entry.name()) {
            Err(_) => Some(PartNameReason::NotUtf8),
            Ok(name) if name.eq_ignore_ascii_case(CONTENT_TYPES) => {
                streams.push(index);
                None
            }
            Ok(name) if name.ends_with('/') && entry.size() == 0 => None,
            Ok(name) => match PartName::from_entry(name) {
                Ok(part) => {
                    named.push((part, index));
                    None
                }
                Err(reason) => Some(reason),
            },
        };
        if let Some(reason) = refused {
            let key = shadow_key(entry.name());
            lost_stream |= key == CONTENT_TYPES_KEY;
            unknown.insert(key);
            refuse(entry, reason, diagnostics);
        }
    }
    let content_types = match streams.as_slice() {
        // An entry that may have been the stream was refused.
        [] if lost_stream => ContentTypesEntry::Refused,
        [] => ContentTypesEntry::Missing,
        [index] => ContentTypesEntry::At(*index),
        [first, rest @ ..] => {
            for &later in rest {
                duplicate(entries, later, *first, diagnostics);
            }
            ContentTypesEntry::Refused
        }
    };
    let (parts, refused) = unique(entries, named, diagnostics);
    unknown.extend(refused);
    Names {
        parts,
        unknown,
        content_types,
    }
}

/// `named` without the names that are the same part as another or derived
/// from another, with the keys of those it drops.
fn unique(
    entries: &[Entry],
    named: Vec<(PartName, usize)>,
    diagnostics: &mut Diagnostics,
) -> (Vec<(PartName, usize)>, BTreeSet<String>) {
    let mut first: BTreeMap<&str, (&PartName, usize)> = BTreeMap::new();
    let mut refused = BTreeSet::new();
    for (name, entry) in &named {
        match first.get(name.key()) {
            Some(&(_, earlier)) => {
                duplicate(entries, *entry, earlier, diagnostics);
                refused.insert(name.key().to_owned());
            }
            None => {
                first.insert(name.key(), (name, *entry));
            }
        }
    }
    let tree = Tree::of(first.values().map(|&(name, _)| name));
    for (name, entry) in &named {
        let key = name.key();
        // A duplicate is reported as one already.
        if refused.contains(key) {
            continue;
        }
        if let Some(base) = tree.ancestor(key) {
            let diagnostic = Diagnostic::PartNameDerived {
                other: Echo::new(base.as_str()),
            };
            push(
                entries,
                *entry,
                diagnostic,
                Location::default().with_part(name.as_str()),
                diagnostics,
            );
            refused.insert(key.to_owned());
        }
    }
    let parts = (named.iter())
        .filter(|(name, _)| !refused.contains(name.key()))
        .cloned()
        .collect();
    (parts, refused)
}

/// The part names as a tree of their segments, so the names a name is
/// derived from are found in one walk down its segments: comparing each of
/// its prefixes with every name would cost the square of its length.
struct Tree<'a> {
    /// Each node's child by segment; the root is node 0.
    children: HashMap<(usize, &'a str), usize>,
    /// The name that ends at each node, if one does.
    names: Vec<Option<&'a PartName>>,
}

impl<'a> Tree<'a> {
    /// The tree of `names`.
    fn of(names: impl Iterator<Item = &'a PartName>) -> Self {
        let mut tree = Self {
            children: HashMap::new(),
            names: vec![None],
        };
        for name in names {
            let mut node = 0;
            for segment in segments(name.key()) {
                let next = tree.names.len();
                node = *tree.children.entry((node, segment)).or_insert(next);
                if node == next {
                    tree.names.push(None);
                }
            }
            if let Some(slot) = tree.names.get_mut(node) {
                *slot = Some(name);
            }
        }
        tree
    }

    /// The shortest name that `key` continues with more segments.
    fn ancestor(&self, key: &str) -> Option<&'a PartName> {
        let segments: Vec<&str> = segments(key).collect();
        let (_, ancestors) = segments.split_last()?;
        let mut node = 0;
        for segment in ancestors {
            node = *self.children.get(&(node, *segment))?;
            if let Some(name) = self.names.get(node).copied().flatten() {
                return Some(name);
            }
        }
        None
    }
}

/// The segments of the part name key `key`.
fn segments(key: &str) -> std::str::Split<'_, char> {
    key.get(1..).unwrap_or_default().split('/')
}

/// Pushes `part_name_duplicate` at the entry `later`, naming `first`; or,
/// when `later` is not readable, at `first`, naming `later`, since the zip
/// diagnostic of one entry does not say why the other is not a part.
#[track_caller]
fn duplicate(entries: &[Entry], later: usize, first: usize, diagnostics: &mut Diagnostics) {
    let readable = |index| entries.get(index).is_some_and(Entry::is_readable);
    let (at, other) = if readable(later) {
        (later, first)
    } else {
        (first, later)
    };
    let name = |index| {
        entries
            .get(index)
            .map_or(&[][..], |entry: &Entry| entry.name())
    };
    let diagnostic = Diagnostic::PartNameDuplicate {
        other: Echo::from_utf8_lossy(name(other)),
    };
    let location = Location::default().with_entry_bytes(name(at));
    push(entries, at, diagnostic, location, diagnostics);
}

/// Pushes `entry_name_invalid` at `entry`, unless it is not readable.
#[track_caller]
fn refuse(entry: &Entry, reason: PartNameReason, diagnostics: &mut Diagnostics) {
    if entry.is_readable() {
        let location = Location::default().with_entry_bytes(entry.name());
        diagnostics.push(Diagnostic::EntryNameInvalid { reason }, location);
    }
}

/// Pushes `diagnostic` at `location`, unless the entry at `index` is not
/// readable.
#[track_caller]
fn push(
    entries: &[Entry],
    index: usize,
    diagnostic: Diagnostic,
    location: Location,
    diagnostics: &mut Diagnostics,
) {
    if entries.get(index).is_some_and(Entry::is_readable) {
        diagnostics.push(diagnostic, location);
    }
}

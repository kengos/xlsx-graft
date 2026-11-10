//! The limits a policy may set, each with the most it may be set to.
//!
//! A host may raise a limit as well as lower it: it is the party that
//! trusts, and a workbook larger than a default limit could not be verified
//! at all otherwise. What bounds a limit from above is the **hard max**: past it the
//! engine's memory estimate, or the type that holds the value, no longer
//! holds. A value past its hard max, or 0, is `out_of_range`.
//!
//! A byte count that is a hard max of 2^53 − 1 (9,007,199,254,740,991) is
//! "no limit that matters": a diagnostic carries a limit as a JSON number,
//! and 2^53 − 1 is the largest one a host that reads JSON numbers as doubles
//! (JavaScript) reads back exactly. It is also the hard max of each limit
//! that bounds only time or is only a threshold, as no memory depends on it.
//!
//! A `fix` that names a policy line (`maxEntries: 2000000`) writes the value
//! the workbook needs, which may be past the hard max; the policy then
//! refuses it as `out_of_range`, which says the engine cannot admit that
//! workbook.
//!
//! | Policy key | Default | Hard max | Why the hard max |
//! | --- | --- | --- | --- |
//! | `maxEntries` | 10,000 | 1,048,576 | each entry of the directory keeps a name and a position in memory |
//! | `maxDirectoryBytes` | 16 MiB | 1 GiB | the directory is read whole; 1,048,576 headers of 46 bytes with a name of up to 1,024 bytes come to about 1 GiB |
//! | `maxEntryNameBytes` | 1,024 | 65,535 | the name's length is a 16-bit field of the zip |
//! | `maxEntryBytes` | 2 GiB | 2^53 − 1 | an entry is read as a stream and never held; it bounds time |
//! | `maxTotalBytes` | 8 GiB | 2^53 − 1 | as `maxEntryBytes` |
//! | `maxCompressionRatio` | 1,000 | 2^53 − 1 | a threshold; it costs no memory |
//! | `minRatioCheckedBytes` | 16 MiB | 2^53 − 1 | a threshold; it costs no memory |
//! | `maxRelationships` | 100,000 | 4,194,304 | the most `maxTotalRelationships` can hold, as it keeps them all |
//! | `maxTotalRelationships` | 1,000,000 | 4,194,304 | the graph keeps every relationship, some 150 bytes and its strings each: 600 MiB and the strings |
//! | `maxXmlDepth` | 256 | 65,536 | each open element keeps the count of its namespace declarations |
//! | `maxXmlAttributes` | 1,024 | 65,536 | the attributes of one element are counted, and past 32 checked for duplicates through a hash set |
//! | `maxXmlNamespaces` | 256 | 4,096 | resolving a prefix walks the declarations in scope, so each element costs this much |
//! | `maxXmlNamespaceBytes` | 256 | 65,536 | each element compares the URI of its prefix |
//! | `maxXmlEventBytes` | 16 MiB | 1 GiB | an event is held whole in the reader's buffer |
//! | `maxXmlBytes` | 16 MiB | 1 GiB | a part read as a whole is held in memory before it is decoded |
//! | `maxFormulaBytes` | 64 KiB | 16 MiB | a formula is held whole to be read, which allocates some tens of times its size |
//! | `maxDefinedNames` | 65,536 | 1,048,576 | each name kept costs a few hundred bytes beside its text |
//! | `maxMergedCells` | 262,144 | 4,194,304 | each range kept costs some 100 bytes as overlaps are looked for: 400 MiB |
//! | `maxCellFormats` | 65,536 | 1,048,576 | the styles' formats are counted and a cell's reference checked against the count |
//! | `maxNumberFormats` | 65,536 | 1,048,576 | as `maxCellFormats` |
//! | `maxDiagnostics` | 1,000 | 10,000 | each diagnostic kept is held, and written into the result: about 2 KiB, and 10 KiB at the most (an echo is up to 800 bytes, and a diagnostic holds several), so the result stays under 100 MB |
//!
//! `maxDiagnostics` is the number of diagnostics the result lists; the ones
//! past it are counted in the `diagnostics_truncated` notice.

use xlsx_graft_diagnostics::{Diagnostics, Limit};
use xlsx_graft_verify::Limits;

/// The largest whole number a JSON number carries exactly as a double.
const MAX_SAFE: u64 = (1 << 53) - 1;

/// What [`Policy`](crate::Policy) sets: the limits verify holds a workbook
/// to, and how many diagnostics the result lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostLimits {
    /// The limits [`verify`](xlsx_graft_verify::verify) holds a workbook to.
    pub verify: Limits,
    /// The most diagnostics the result lists. Policy name `maxDiagnostics`.
    pub max_diagnostics: Limit,
}

impl Default for HostLimits {
    fn default() -> Self {
        Self {
            verify: Limits::default(),
            max_diagnostics: Limit::default_of(Diagnostics::DEFAULT_LIMIT),
        }
    }
}

/// One limit a policy sets: its key, its hard max and where it lives.
pub struct LimitKey {
    /// The policy key, in camelCase.
    pub name: &'static str,
    /// The most the limit may be set to.
    pub hard_max: u64,
    slot: fn(&mut HostLimits) -> &mut Limit,
}

impl LimitKey {
    /// The value the engine holds this limit to unless a policy sets it.
    #[must_use]
    pub fn default_value(&self) -> u64 {
        (self.slot)(&mut HostLimits::default()).value
    }

    /// Sets this limit in `limits` to `value`, as the policy's.
    pub(crate) fn set(&self, limits: &mut HostLimits, value: u64) {
        *(self.slot)(limits) = Limit::policy_of(value);
    }
}

/// Defines [`LIMITS`] from its rows.
macro_rules! limit_keys {
    ($( $name:literal, $hard_max:expr, |$l:ident| $slot:expr; )*) => {
        /// Every limit a policy may set, in the order of the table above.
        pub const LIMITS: &[LimitKey] = &[
            $( LimitKey { name: $name, hard_max: $hard_max, slot: |$l| $slot }, )*
        ];
    };
}

limit_keys! {
    "maxEntries", 1 << 20, |l| &mut l.verify.package.zip.max_entries;
    "maxDirectoryBytes", 1 << 30, |l| &mut l.verify.package.zip.max_directory_bytes;
    "maxEntryNameBytes", 65_535, |l| &mut l.verify.package.zip.max_entry_name_bytes;
    "maxEntryBytes", MAX_SAFE, |l| &mut l.verify.package.zip.max_entry_bytes;
    "maxTotalBytes", MAX_SAFE, |l| &mut l.verify.package.zip.max_total_bytes;
    "maxCompressionRatio", MAX_SAFE, |l| &mut l.verify.package.zip.max_compression_ratio;
    "minRatioCheckedBytes", MAX_SAFE, |l| &mut l.verify.package.zip.min_ratio_checked_bytes;
    "maxRelationships", 1 << 22, |l| &mut l.verify.package.max_relationships;
    "maxTotalRelationships", 1 << 22, |l| &mut l.verify.package.max_total_relationships;
    "maxXmlDepth", 65_536, |l| &mut l.verify.package.xml.max_depth;
    "maxXmlAttributes", 65_536, |l| &mut l.verify.package.xml.max_attributes;
    "maxXmlNamespaces", 4_096, |l| &mut l.verify.package.xml.max_namespaces;
    "maxXmlNamespaceBytes", 65_536, |l| &mut l.verify.package.xml.max_namespace_bytes;
    "maxXmlEventBytes", 1 << 30, |l| &mut l.verify.package.xml.max_event_bytes;
    "maxXmlBytes", 1 << 30, |l| &mut l.verify.package.xml.max_bytes;
    "maxFormulaBytes", 16 << 20, |l| &mut l.verify.max_formula_bytes;
    "maxDefinedNames", 1 << 20, |l| &mut l.verify.max_defined_names;
    "maxMergedCells", 1 << 22, |l| &mut l.verify.max_merged_cells;
    "maxCellFormats", 1 << 20, |l| &mut l.verify.max_cell_formats;
    "maxNumberFormats", 1 << 20, |l| &mut l.verify.max_number_formats;
    "maxDiagnostics", 10_000, |l| &mut l.max_diagnostics;
}

#[cfg(test)]
mod tests;

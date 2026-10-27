//! [`Limits`]: how much of a package the reader admits, and [`ZipLimits`],
//! its share for the zip.

use xlsx_graft_diagnostics::Limit;
use xlsx_graft_xml::XmlLimits;

/// The limits a [`Package`](crate::Package) holds a package to: the zip's,
/// the XML's for the content types stream and the Relationships parts, and
/// the package model's own.
///
/// The defaults are provisional, to be revisited against large real
/// workbooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The limits on the zip.
    pub zip: ZipLimits,
    /// The limits on each XML stream the package model reads.
    pub xml: XmlLimits,
    /// The most relationships one Relationships part may hold. Policy name
    /// `maxRelationships`.
    pub max_relationships: Limit,
    /// The most relationships all the Relationships parts may hold in all,
    /// since the graph keeps every one in memory. It bounds their count,
    /// not their size: the `Id`, `Type` and external `Target` strings kept
    /// are bounded only by the parts' sizes, so by `max_total_bytes` of the
    /// zip in all. Policy name `maxTotalRelationships`.
    pub max_total_relationships: Limit,
}

impl Limits {
    /// The default `max_relationships`. A worksheet's relationships grow with
    /// its hyperlinks, up to Excel's 65,530.
    pub const DEFAULT_MAX_RELATIONSHIPS: u64 = 100_000;
    /// The default `max_total_relationships`: ten worksheets at the
    /// `max_relationships` default.
    pub const DEFAULT_MAX_TOTAL_RELATIONSHIPS: u64 = 1_000_000;
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            zip: ZipLimits::default(),
            xml: XmlLimits::default(),
            max_relationships: Limit::default_of(Self::DEFAULT_MAX_RELATIONSHIPS),
            max_total_relationships: Limit::default_of(Self::DEFAULT_MAX_TOTAL_RELATIONSHIPS),
        }
    }
}

/// The limits a [`ZipArchive`](crate::ZipArchive) holds a zip to.
///
/// Each is a [`Limit`], so a diagnostic it decides names who set it. The
/// defaults are provisional, to be revisited against large real workbooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZipLimits {
    /// The most entries the end record may list. Checked before the central
    /// directory is read. Policy name `maxEntries`.
    pub max_entries: Limit,
    /// The most bytes the central directory may take, as the end record
    /// gives it. Checked before it is read. Policy name `maxDirectoryBytes`.
    pub max_directory_bytes: Limit,
    /// The longest an entry's name may be, in bytes. Policy name
    /// `maxEntryNameBytes`.
    pub max_entry_name_bytes: Limit,
    /// The most bytes one entry may declare it inflates to. Reading an entry
    /// stops at what it declares, so this bounds what it can produce too.
    /// Policy name `maxEntryBytes`.
    pub max_entry_bytes: Limit,
    /// The most bytes all the entries may declare they inflate to. Policy
    /// name `maxTotalBytes`.
    pub max_total_bytes: Limit,
    /// The most times an entry's declared inflated size may be its
    /// compressed size, for an entry that declares at least
    /// `min_ratio_checked_bytes`. Policy name `maxCompressionRatio`.
    pub max_compression_ratio: Limit,
    /// The smallest declared inflated size `max_compression_ratio` applies
    /// to: below it, a high ratio cannot cost much. Policy name
    /// `minRatioCheckedBytes`.
    pub min_ratio_checked_bytes: Limit,
}

impl ZipLimits {
    /// The default `max_entries`. A sheet takes six to eight entries (the
    /// sheet, its relationships, a drawing, comments, VML, tables), so 255
    /// sheets come to about 2,000.
    pub const DEFAULT_MAX_ENTRIES: u64 = 10_000;
    /// The default `max_directory_bytes`, 16 MiB: 10,000 headers of 46 bytes
    /// with a name of up to 1,024 bytes each come to about 10 MiB.
    pub const DEFAULT_MAX_DIRECTORY_BYTES: u64 = 16 * 1024 * 1024;
    /// The default `max_entry_name_bytes`. Excel's part names run to a few
    /// dozen bytes; a zip allows 65,535.
    pub const DEFAULT_MAX_ENTRY_NAME_BYTES: u64 = 1_024;
    /// The default `max_entry_bytes`, 2 GiB: a sheet of a million rows of 20
    /// columns comes to about 600 MB.
    pub const DEFAULT_MAX_ENTRY_BYTES: u64 = 2 * 1024 * 1024 * 1024;
    /// The default `max_total_bytes`, 8 GiB.
    pub const DEFAULT_MAX_TOTAL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
    /// The default `max_compression_ratio`. Deflate cannot exceed about
    /// 1,032, and a large sheet of one repeated value can come close.
    pub const DEFAULT_MAX_COMPRESSION_RATIO: u64 = 1_000;
    /// The default `min_ratio_checked_bytes`, 16 MiB.
    pub const DEFAULT_MIN_RATIO_CHECKED_BYTES: u64 = 16 * 1024 * 1024;
}

impl Default for ZipLimits {
    fn default() -> Self {
        Self {
            max_entries: Limit::default_of(Self::DEFAULT_MAX_ENTRIES),
            max_directory_bytes: Limit::default_of(Self::DEFAULT_MAX_DIRECTORY_BYTES),
            max_entry_name_bytes: Limit::default_of(Self::DEFAULT_MAX_ENTRY_NAME_BYTES),
            max_entry_bytes: Limit::default_of(Self::DEFAULT_MAX_ENTRY_BYTES),
            max_total_bytes: Limit::default_of(Self::DEFAULT_MAX_TOTAL_BYTES),
            max_compression_ratio: Limit::default_of(Self::DEFAULT_MAX_COMPRESSION_RATIO),
            min_ratio_checked_bytes: Limit::default_of(Self::DEFAULT_MIN_RATIO_CHECKED_BYTES),
        }
    }
}

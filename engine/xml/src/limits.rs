//! [`XmlLimits`]: how much of a part's XML the reader admits.

use xlsx_graft_diagnostics::Limit;

/// The limits an [`XmlReader`](crate::XmlReader) or an
/// [`XmlStreamReader`](crate::XmlStreamReader) holds a part to.
///
/// Each is a [`Limit`], so a diagnostic it decides names who set it. The
/// defaults are provisional, to be revisited against large real workbooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlLimits {
    /// The deepest an element may nest; the root element is at depth 1.
    /// Policy name `maxXmlDepth`.
    pub max_depth: Limit,
    /// The most attributes one element may have. Policy name
    /// `maxXmlAttributes`.
    pub max_attributes: Limit,
    /// The most namespace declarations (`xmlns` and `xmlns:*` attributes)
    /// in scope at once. It also bounds what resolving a prefix costs, which
    /// walks the declarations in scope. Policy name `maxXmlNamespaces`.
    pub max_namespaces: Limit,
    /// The most bytes of XML read into memory, before decoding, by an
    /// [`XmlReader`](crate::XmlReader). An
    /// [`XmlStreamReader`](crate::XmlStreamReader) does not apply it. Policy
    /// name `maxXmlBytes`.
    pub max_bytes: Limit,
}

impl XmlLimits {
    /// The default `max_depth`. A relationships part or the content types
    /// stream nests 2 deep, a worksheet about 6.
    pub const DEFAULT_MAX_DEPTH: u64 = 256;
    /// The default `max_attributes`. Excel writes at most a few dozen.
    pub const DEFAULT_MAX_ATTRIBUTES: u64 = 1_024;
    /// The default `max_namespaces`. Excel declares a dozen or two on a
    /// worksheet's root.
    pub const DEFAULT_MAX_NAMESPACES: u64 = 256;
    /// The default `max_bytes`, 16 MiB: a worksheet's relationships, one per
    /// hyperlink up to Excel's 65,530, come to about 7 MiB.
    pub const DEFAULT_MAX_BYTES: u64 = 16 * 1024 * 1024;
}

impl Default for XmlLimits {
    fn default() -> Self {
        Self {
            max_depth: Limit::default_of(Self::DEFAULT_MAX_DEPTH),
            max_attributes: Limit::default_of(Self::DEFAULT_MAX_ATTRIBUTES),
            max_namespaces: Limit::default_of(Self::DEFAULT_MAX_NAMESPACES),
            max_bytes: Limit::default_of(Self::DEFAULT_MAX_BYTES),
        }
    }
}

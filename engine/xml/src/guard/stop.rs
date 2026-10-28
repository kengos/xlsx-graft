//! [`Stop`]: why the guard stopped a reader, and the diagnostic both readers
//! return for it.

use crate::{XmlLimits, malformed};
use xlsx_graft_diagnostics::Diagnostic;

/// Why the guard stopped the reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Stop {
    /// Not well-formed, for the reason given.
    Malformed(String),
    /// A DOCTYPE declaration.
    Doctype,
    /// An element one deeper than `max_depth`.
    TooDeep,
    /// An element with this many attributes, more than `max_attributes`.
    TooManyAttributes(u64),
    /// An element with this many namespace declarations in scope, more than
    /// `max_namespaces`.
    TooManyNamespaces(u64),
    /// A namespace declaration whose value is this many bytes, more than
    /// `max_namespace_bytes`.
    NamespaceTooLong(u64),
}

impl Stop {
    /// The diagnostic for this stop at `offset`, where the event starts,
    /// under `limits`. [`Stop::TooDeep`] reports the element one past
    /// `max_depth`, the depth read so far; the in-memory reader reads on for
    /// the deepest.
    pub(crate) fn diagnostic(self, offset: u64, limits: &XmlLimits) -> Diagnostic {
        match self {
            Self::Malformed(detail) => malformed(offset, &detail),
            Self::Doctype => Diagnostic::XmlDoctype { offset },
            Self::TooDeep => Diagnostic::XmlTooDeep {
                offset,
                depth: limits.max_depth.value.saturating_add(1),
                limit: limits.max_depth,
            },
            Self::TooManyAttributes(count) => Diagnostic::XmlTooManyAttributes {
                offset,
                count,
                limit: limits.max_attributes,
            },
            Self::TooManyNamespaces(count) => Diagnostic::XmlTooManyNamespaces {
                offset,
                count,
                limit: limits.max_namespaces,
            },
            Self::NamespaceTooLong(length) => Diagnostic::XmlNamespaceTooLong {
                offset,
                length,
                limit: limits.max_namespace_bytes,
            },
        }
    }
}

//! Which encoding a part is in: the byte order mark decides between UTF-8
//! and UTF-16, and the XML declaration, when it names an encoding, must
//! agree (ECMA-376-2:2021 §6.2.5; XML 1.0 §4.3.3 and Appendix F). The
//! decoder sniffs the mark; the reader checks the declaration.

use crate::malformed;
use quick_xml::events::BytesDecl;
use xlsx_graft_diagnostics::{Diagnostic, Echo, EncodingReason};

/// The encoding the bytes are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Found {
    /// UTF-8, with or without a byte order mark.
    Utf8,
    /// UTF-16, little-endian, with a byte order mark.
    Utf16Le,
    /// UTF-16, big-endian, with a byte order mark.
    Utf16Be,
}

/// The encoding the first bytes show, and the bytes after the byte order
/// mark (XML 1.0 Appendix F). Without a mark the bytes are UTF-8, unless a
/// NUL among the first two shows UTF-16, which XML requires to carry one, or
/// UTF-32. A NUL is never XML, so this refuses no UTF-8 part that could read.
pub(crate) fn sniff(bytes: &[u8]) -> Result<(Found, &[u8]), Diagnostic> {
    match bytes {
        [0x00, 0x00, ..]
        | [0xff, 0xfe, 0x00, 0x00, ..]
        | [0xfe, 0xff, 0x00, 0x00, ..]
        | [0x00, _, 0x00, 0x00, ..]
        | [_, 0x00, 0x00, 0x00, ..] => Err(unsupported("UTF-32", EncodingReason::OtherEncoding)),
        [0xef, 0xbb, 0xbf, rest @ ..] => Ok((Found::Utf8, rest)),
        [0xff, 0xfe, rest @ ..] => Ok((Found::Utf16Le, rest)),
        [0xfe, 0xff, rest @ ..] => Ok((Found::Utf16Be, rest)),
        [0x00, _, ..] => Err(unsupported("UTF-16BE", EncodingReason::Utf16WithoutBom)),
        [_, 0x00, ..] => Err(unsupported("UTF-16LE", EncodingReason::Utf16WithoutBom)),
        _ => Ok((Found::Utf8, bytes)),
    }
}

/// Checks the encoding `decl`, the XML declaration, names, if it names one,
/// against `found`, the one the part was decoded from. The names compare
/// ignoring ASCII case; only `UTF-8` and `UTF-16` are admitted (§6.2.5
/// names no other, and `UTF-16LE` or `UTF-16BE` would label UTF-16 without
/// a byte order mark).
pub(crate) fn check_decl(decl: &BytesDecl<'_>, found: Found) -> Result<(), Diagnostic> {
    let name = match decl.encoding() {
        None => return Ok(()),
        Some(Ok(name)) => name,
        Some(Err(error)) => return Err(malformed(0, &error.to_string())),
    };
    let fits = match name.to_ascii_uppercase().as_str() {
        "UTF-8" => found == Found::Utf8,
        "UTF-16" => found != Found::Utf8,
        _ => return Err(unsupported(&name, EncodingReason::OtherEncoding)),
    };
    if fits {
        Ok(())
    } else {
        Err(unsupported(&name, EncodingReason::DeclarationMismatch))
    }
}

/// `xml_encoding_unsupported` for `encoding`, because of `reason`.
pub(crate) fn unsupported(encoding: &str, reason: EncodingReason) -> Diagnostic {
    Diagnostic::XmlEncodingUnsupported {
        encoding: Echo::new(encoding),
        reason,
    }
}

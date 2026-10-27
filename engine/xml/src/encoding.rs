//! A part's bytes as UTF-8 text: the byte order mark decides between UTF-8
//! and UTF-16, and the XML declaration, when it names an encoding, must
//! agree (ECMA-376-2:2021 §6.2.5; XML 1.0 §4.3.3 and Appendix F).

use crate::malformed;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::borrow::Cow;
use xlsx_graft_diagnostics::{Diagnostic, Echo, EncodingReason};

/// The encoding the bytes are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Found {
    Utf8,
    Utf16Le,
    Utf16Be,
}

/// `bytes` as UTF-8 text, without its byte order mark.
pub(crate) fn decode(bytes: &[u8]) -> Result<Cow<'_, str>, Diagnostic> {
    let (found, body) = sniff(bytes)?;
    let text = match found {
        Found::Utf8 => Cow::Borrowed(
            std::str::from_utf8(body)
                .map_err(|_| unsupported("UTF-8", EncodingReason::InvalidUtf8))?,
        ),
        Found::Utf16Le => Cow::Owned(utf16(body, u16::from_le_bytes, "UTF-16LE")?),
        Found::Utf16Be => Cow::Owned(utf16(body, u16::from_be_bytes, "UTF-16BE")?),
    };
    check_declaration(&text, found)?;
    Ok(text)
}

/// The encoding the first bytes show, and the bytes after the byte order
/// mark (XML 1.0 Appendix F). Without a mark the bytes are UTF-8, unless a
/// NUL among the first two shows UTF-16, which XML requires to carry one, or
/// UTF-32. A NUL is never XML, so this refuses no UTF-8 part that could read.
fn sniff(bytes: &[u8]) -> Result<(Found, &[u8]), Diagnostic> {
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

/// `body` decoded from UTF-16, each code unit read by `unit`.
fn utf16(body: &[u8], unit: fn([u8; 2]) -> u16, name: &str) -> Result<String, Diagnostic> {
    let invalid = || unsupported(name, EncodingReason::InvalidUtf16);
    let (pairs, rest) = body.as_chunks::<2>();
    if !rest.is_empty() {
        return Err(invalid());
    }
    let units = pairs.iter().copied().map(unit);
    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .map_err(|_| invalid())
}

/// Checks the encoding the XML declaration names, if it has one, against
/// the one `text` was decoded from. The names compare ignoring ASCII case;
/// only `UTF-8` and `UTF-16` are admitted (§6.2.5 names no other, and
/// `UTF-16LE` or `UTF-16BE` would label UTF-16 without a byte order mark).
fn check_declaration(text: &str, found: Found) -> Result<(), Diagnostic> {
    let Ok(Event::Decl(decl)) = Reader::from_str(text).read_event() else {
        return Ok(());
    };
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
fn unsupported(encoding: &str, reason: EncodingReason) -> Diagnostic {
    Diagnostic::XmlEncodingUnsupported {
        encoding: Echo::new(encoding),
        reason,
    }
}

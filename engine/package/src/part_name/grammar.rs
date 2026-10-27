//! The part name grammar (ECMA-376-2:2021 §6.2.2.2): `1*( "/"
//! isegment-nz )`, with `ipchar` from RFC 3987 §2.2, under the constraints
//! on percent-encoding and dots; and the decodings a name goes through
//! before it is checked.

use xlsx_graft_diagnostics::PartNameReason;

/// Checks `name`, which starts with `/`, segment by segment.
pub(super) fn check(name: &str) -> Result<(), PartNameReason> {
    for segment in name.get(1..).unwrap_or_default().split('/') {
        if segment.is_empty() {
            return Err(PartNameReason::EmptySegment);
        }
        check_segment(segment)?;
        if segment.ends_with('.') {
            return Err(PartNameReason::EndsWithDot);
        }
    }
    Ok(())
}

/// Checks each character of a non-empty segment: an `ipchar`, and a
/// percent-encoding of neither a slash nor an unreserved character.
fn check_segment(segment: &str) -> Result<(), PartNameReason> {
    let mut rest = segment;
    while let Some(c) = rest.chars().next() {
        let len = if c == '%' {
            let byte = encoded(rest).ok_or(PartNameReason::InvalidCharacter)?;
            if byte == b'/' || byte == b'\\' {
                return Err(PartNameReason::EncodedSlash);
            }
            if is_unreserved(byte) {
                return Err(PartNameReason::EncodedUnreserved);
            }
            3
        } else if is_ipchar(c) {
            c.len_utf8()
        } else {
            return Err(PartNameReason::InvalidCharacter);
        };
        rest = rest.get(len..).unwrap_or_default();
    }
    Ok(())
}

/// Decodes each run of percent-encoded bytes of 0x80 and over that is UTF-8
/// for a `ucschar`, so a name reaches its IRI form (RFC 3987 §3.2); the
/// bytes of a run that are not UTF-8, or are a character an IRI may not
/// hold unencoded, stay encoded, in uppercase.
pub(crate) fn decode_non_ascii(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(c) = rest.chars().next() {
        let (run, after) = encoded_run(rest);
        if run.is_empty() {
            out.push(c);
            rest = rest.get(c.len_utf8()..).unwrap_or_default();
            continue;
        }
        for chunk in run.utf8_chunks() {
            for c in chunk.valid().chars() {
                if is_ucschar(c) {
                    out.push(c);
                } else {
                    let mut utf8 = [0; 4];
                    encode(c.encode_utf8(&mut utf8).as_bytes(), &mut out);
                }
            }
            encode(chunk.invalid(), &mut out);
        }
        rest = after;
    }
    out
}

/// Appends `bytes` to `out`, each percent-encoded.
pub(super) fn encode(bytes: &[u8], out: &mut String) {
    for byte in bytes {
        out.push_str(&format!("%{byte:02X}"));
    }
}

/// Decodes the percent-encoded unreserved ASCII characters of `path`.
pub(crate) fn decode_unreserved(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(c) = rest.chars().next() {
        let (c, len) = match encoded(rest).filter(|&byte| is_unreserved(byte)) {
            Some(byte) => (char::from(byte), 3),
            None => (c, c.len_utf8()),
        };
        out.push(c);
        rest = rest.get(len..).unwrap_or_default();
    }
    out
}

/// The percent-encoded bytes of 0x80 and over that `text` starts with, and
/// the text after them.
fn encoded_run(mut text: &str) -> (Vec<u8>, &str) {
    let mut run = Vec::new();
    while let Some(byte) = encoded(text).filter(|&byte| byte >= 0x80) {
        run.push(byte);
        text = text.get(3..).unwrap_or_default();
    }
    (run, text)
}

/// The byte `text` percent-encodes at its start, if it starts with `%` and
/// two hexadecimal digits.
pub(super) fn encoded(text: &str) -> Option<u8> {
    let hex = text.strip_prefix('%')?.get(..2)?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u8::from_str_radix(hex, 16).ok()
}

/// RFC 3986 `unreserved`: a letter, a digit, `-`, `.`, `_` or `~`.
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// RFC 3987 `ipchar`, but for `pct-encoded`, which the caller reads.
pub(super) fn is_ipchar(c: char) -> bool {
    match u8::try_from(c) {
        Ok(byte) if byte.is_ascii() => is_unreserved(byte) || b"!$&'()*+,;=:@".contains(&byte),
        _ => is_ucschar(c),
    }
}

/// RFC 3987 `ucschar`: the non-ASCII characters an IRI may hold outside its
/// query. Private use characters and the noncharacters are not among them.
fn is_ucschar(c: char) -> bool {
    let c = u32::from(c);
    matches!(c, 0xA0..=0xD7FF | 0xF900..=0xFDCF | 0xFDF0..=0xFFEF | 0xE_1000..=0xE_FFFD)
        || ((0x1_0000..=0xD_FFFF).contains(&c) && c & 0xFFFF <= 0xFFFD)
}

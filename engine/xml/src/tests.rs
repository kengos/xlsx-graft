//! Tests for `XmlReader`, driven through `open` and `read_event_into`. Each file under
//! `tests/` covers one aspect of what the reader refuses or admits.

mod doctype;
mod encoding;
mod limits;
mod malformed;
mod reading;

use crate::events::Event;
use crate::{XmlLimits, XmlReader};
use xlsx_graft_diagnostics::Diagnostic;

/// Reads `bytes` to the end under `limits`: how many events came before
/// `Eof`, or the first diagnostic.
fn read_with(bytes: &[u8], limits: &XmlLimits) -> Result<usize, Diagnostic> {
    let mut reader = XmlReader::open(bytes, limits)?;
    let mut buf = Vec::new();
    let mut events = 0;
    while !matches!(reader.read_event_into(&mut buf)?, Event::Eof) {
        events += 1;
    }
    Ok(events)
}

/// [`read_with`] under the default limits.
fn read(bytes: &[u8]) -> Result<usize, Diagnostic> {
    read_with(bytes, &XmlLimits::default())
}

/// The offset and detail of the `xml_malformed` that reading `bytes` ends in.
fn malformed_at(bytes: &[u8]) -> (u64, String) {
    match read(bytes) {
        Err(Diagnostic::XmlMalformed { offset, detail }) => (offset, detail.as_str().to_owned()),
        other => panic!("expected xml_malformed, got {other:?}"),
    }
}

/// `text` in UTF-16 with a byte order mark, little-endian unless `big`.
fn utf16(text: &str, big: bool) -> Vec<u8> {
    let mut bytes = if big {
        vec![0xfe, 0xff]
    } else {
        vec![0xff, 0xfe]
    };
    for unit in text.encode_utf16() {
        bytes.extend(if big {
            unit.to_be_bytes()
        } else {
            unit.to_le_bytes()
        });
    }
    bytes
}

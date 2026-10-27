//! [`shadow_key`]: the key a zip entry whose name is not a part name would
//! have, had a lenient reader taken it for a part.
//!
//! The entry is not a part, and is reported. Its shadow key stands for it
//! where a later check would look a name up: a relationship that targets
//! it, or a Relationships part whose source it is, would otherwise report
//! the part missing, a second diagnostic for the one problem. A shadow key
//! need not be one any part name has; one that is not matches nothing.

use super::grammar::{decode_non_ascii, decode_unreserved, encode, encoded, is_ipchar};

/// The shadow key of the entry `name`: lossy UTF-8, each `\` read as `/`,
/// decoded as a part name and a reference are; empty and `.` segments
/// dropped, a `..` taking the segment before it, and a final interleaved
/// piece segment (`[0].piece`, `[1].last.piece`; §7.2.4) dropped, under a
/// leading `/`; every character a part name may not hold percent-encoded,
/// and lowered.
pub(crate) fn shadow_key(name: &[u8]) -> String {
    let name = String::from_utf8_lossy(name).replace('\\', "/");
    let name = decode_unreserved(&decode_non_ascii(&name));
    let mut segments: Vec<&str> = Vec::new();
    for segment in name.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            segment => segments.push(segment),
        }
    }
    if segments.last().is_some_and(|last| is_piece(last)) {
        segments.pop();
    }
    let path = format!("/{}", segments.join("/"));
    let mut key = String::with_capacity(path.len());
    let mut rest = path.as_str();
    while let Some(c) = rest.chars().next() {
        let escaped = c == '%' && encoded(rest).is_some();
        let len = if escaped { 3 } else { c.len_utf8() };
        let (head, tail) = rest.split_at_checked(len).unwrap_or((rest, ""));
        if escaped || c == '/' || is_ipchar(c) {
            key.push_str(head);
        } else {
            encode(head.as_bytes(), &mut key);
        }
        rest = tail;
    }
    key.to_ascii_lowercase()
}

/// Whether `segment` names an interleaved piece: `[` digits `]`, then
/// `.piece` or `.last.piece`, letter case and leading zeros aside.
fn is_piece(segment: &str) -> bool {
    let lower = segment.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix('[') else {
        return false;
    };
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let suffix = rest.get(digits..).unwrap_or_default();
    digits > 0 && (suffix == "].piece" || suffix == "].last.piece")
}

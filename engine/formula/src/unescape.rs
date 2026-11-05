//! Decodes a formula's `_xHHHH_` escapes once (ECMA-376-1:2016 §22.9.2.19,
//! `ST_Xstring`).

use crate::{Malformed, MalformedReason, Span};

/// An escape's length: `_x`, four hex digits, `_`.
const ESCAPE_LEN: usize = 7;

/// What an escape is.
enum Escape<'a> {
    /// It decodes to this character.
    Decoded(char),
    /// It stays as written, malformed for this reason.
    Kept(&'a str, MalformedReason),
}

/// `src` with each `_xHHHH_` decoded, reading left to right and never
/// reading what a decoded escape gave back, so `_x005F_x0041_` is
/// `_x0041_`. An escape naming a surrogate, and one written `_XHHHH_`,
/// stay as written and are malformed, at their place in the decoded text.
/// `_x` followed by anything but four hex digits and `_` is no escape and
/// stays as it is.
pub(crate) fn unescape(src: &str) -> (String, Vec<Malformed>) {
    let mut out = String::with_capacity(src.len());
    let mut malformed = Vec::new();
    let mut rest = src;
    while let Some(at) = start(rest) {
        let (head, tail) = rest.split_at_checked(at).unwrap_or((rest, ""));
        out.push_str(head);
        match escape(tail) {
            Some(Escape::Decoded(c)) => out.push(c),
            Some(Escape::Kept(raw, reason)) => {
                let start = out.len();
                out.push_str(raw);
                malformed.push(Malformed {
                    reason,
                    span: Span {
                        start,
                        end: out.len(),
                    },
                });
            }
            None => {
                let (head, after) = tail.split_at_checked(2).unwrap_or((tail, ""));
                out.push_str(head);
                rest = after;
                continue;
            }
        }
        rest = tail.get(ESCAPE_LEN..).unwrap_or("");
    }
    out.push_str(rest);
    (out, malformed)
}

/// Where the first `_x` or `_X` in `rest` is.
fn start(rest: &str) -> Option<usize> {
    rest.match_indices('_')
        .map(|(at, _)| at)
        .find(|at| matches!(rest.as_bytes().get(at.saturating_add(1)), Some(b'x' | b'X')))
}

/// The escape `tail` starts with, if it starts with one.
///
/// ECMA-376 writes the `x` in lower case. Whether Excel decodes `_X0028_`
/// is not written down, and a guess either way can hide a call (decoded,
/// `F_X005B_(1)+_X005D_` is `F[(1)+]`, a table), so it is kept as written
/// and malformed: a caller refuses what it cannot read.
fn escape(tail: &str) -> Option<Escape<'_>> {
    let raw = tail.get(..ESCAPE_LEN)?;
    let digits = raw.get(2..)?.strip_suffix('_')?;
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    if raw.starts_with("_X") {
        return Some(Escape::Kept(raw, MalformedReason::UpperCaseEscape));
    }
    let code = u32::from_str_radix(digits, 16).ok()?;
    Some(char::from_u32(code).map_or(
        Escape::Kept(raw, MalformedReason::UndecodableEscape),
        Escape::Decoded,
    ))
}

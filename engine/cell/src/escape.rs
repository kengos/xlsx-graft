//! [`unescape`] and [`escape`]: the `_xHHHH_` escapes of ST_Xstring
//! (ECMA-376-1:2016 §22.9.2.19) undone, on a value that arrives in pieces
//! ([`unescape`]), and made for writing ([`escape`]).
//!
//! An escape is `_x`, four hexadecimal digits in either case, and `_`, and
//! stands for the UTF-16 code unit the digits give: `_x005F_` is `_`
//! itself, which is how a writer keeps a literal `_x0041_` from being read
//! as `A`. A high surrogate escaped right before a low one makes one
//! character; an unpaired surrogate, like anything else that is not an
//! escape, is kept as written. The text is read once from the left, so what
//! an escape turns into is never read again as part of another.
//!
//! A value arrives as the XML reader reads it: text, then a reference, then
//! more text. An escape can run across them, so what may still turn out to
//! start one is held back until more arrives, never more than one pair of
//! escapes' length ([`Xstring`]).

/// The length of one escape, `_xHHHH_`.
const ESCAPE: usize = 7;

/// The length of the longest escape, a surrogate pair: what may still start
/// one is shorter.
const LONGEST_ESCAPE: usize = 2 * ESCAPE;

/// The value of one ST_Xstring element as it arrives, piece by piece, its
/// escapes undone.
///
/// The caller pushes the last piece of a value with `last = true`: until
/// then, the text from the first `_` that is fewer than 14 bytes (the
/// length of an escaped surrogate pair) from the end is held back, whether
/// or not it turns out to start an escape, and it is passed to the `out`
/// of a later [`Xstring::push`]. [`Xstring::begin`] starts the next value
/// and discards the text held back from the previous one.
///
/// # Examples
///
/// ```
/// use std::convert::Infallible;
/// use xlsx_graft_cell::Xstring;
///
/// let mut value = String::new();
/// let mut xstring = Xstring::default();
/// xstring.begin();
/// for (piece, last) in [("a_x00", false), ("41_b", true)] {
///     xstring
///         .push::<Infallible>(piece, last, |text| {
///             value.push_str(text);
///             Ok(())
///         })
///         .unwrap();
/// }
/// assert_eq!(value, "aAb");
/// ```
#[derive(Debug, Default)]
pub struct Xstring {
    /// The end of the value read so far that may still start an escape,
    /// held back until more of it arrives.
    held: String,
}

impl Xstring {
    /// Starts a value, discarding the text held back from the previous
    /// value.
    pub fn begin(&mut self) {
        self.held.clear();
    }

    /// Adds `text` to the value, its escapes undone and handed to `out`,
    /// holding back what may still start one unless `last` says `text` is
    /// the last piece of the value.
    ///
    /// What was held back is joined to no more of `text` than finishes an
    /// escape it may start, so `text` is never copied whole.
    pub fn push<E>(
        &mut self,
        text: &str,
        last: bool,
        mut out: impl FnMut(&str) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut text = text;
        if !self.held.is_empty() {
            let cut = (text.char_indices().map(|(at, _)| at))
                .find(|&at| at >= LONGEST_ESCAPE)
                .unwrap_or(text.len());
            let (head, tail) = text.split_at(cut);
            let mut joined = std::mem::take(&mut self.held);
            let held = joined.len();
            joined.push_str(head);
            let read = unescape(&joined, last && tail.is_empty(), &mut out)?;
            if tail.is_empty() {
                self.held.push_str(joined.get(read..).unwrap_or_default());
                return Ok(());
            }
            // With a whole escape's length after it, nothing held is held
            // again: reading stopped in `head`.
            text = text.get(read.saturating_sub(held)..).unwrap_or_default();
        }
        let read = unescape(text, last, out)?;
        self.held.push_str(text.get(read..).unwrap_or_default());
        Ok(())
    }
}

/// Undoes the escapes in `text`, handing the value to `out` piece by piece,
/// and returns how many bytes of `text` it read. Unless `last`, it stops
/// at an `_` too close to the end to tell whether it starts an escape, and
/// the caller passes `text[n..]` again, `n` the returned count, joined in
/// front of the next piece.
pub fn unescape<E>(
    text: &str,
    last: bool,
    mut out: impl FnMut(&str) -> Result<(), E>,
) -> Result<usize, E> {
    let mut rest = text;
    while let Some(at) = rest.find('_') {
        let (head, tail) = rest.split_at(at);
        out(head)?;
        if !last && tail.len() < LONGEST_ESCAPE {
            return Ok(text.len().saturating_sub(tail.len()));
        }
        let (c, len) = escaped(tail).unwrap_or(('_', 1));
        out(c.encode_utf8(&mut [0; 4]))?;
        rest = tail.get(len..).unwrap_or_default();
    }
    out(rest)?;
    Ok(text.len())
}

/// The character the escape at the start of `text` stands for, and how
/// many bytes it takes: one escape, or two for a surrogate pair.
fn escaped(text: &str) -> Option<(char, usize)> {
    let high = unit(text)?;
    if let Some(c) = char::from_u32(u32::from(high)) {
        return Some((c, ESCAPE));
    }
    let low = unit(text.get(ESCAPE..)?)?;
    let c = char::decode_utf16([high, low]).next()?.ok()?;
    Some((c, LONGEST_ESCAPE))
}

/// The code unit of the escape at the start of `text`.
fn unit(text: &str) -> Option<u16> {
    let digits = text.strip_prefix("_x")?.get(..5)?.strip_suffix('_')?;
    // `from_str_radix` would admit a sign.
    digits
        .bytes()
        .all(|b| b.is_ascii_hexdigit())
        .then_some(())?;
    u16::from_str_radix(digits, 16).ok()
}

/// Writes `text` as an ST_Xstring value (ECMA-376-1:2016 §22.9.2.19), the
/// inverse of [`unescape`], handing it to `out` piece by piece.
///
/// A character that is not an XML 1.0 `Char` (U+0000 to U+001F but tab,
/// line feed and carriage return, and U+FFFE and U+FFFF) is written as
/// `_xHHHH_`, the digits in upper case. An `_` that would otherwise start an
/// escape is written `_x005F_`: one followed by `x`, four hexadecimal
/// digits in either case, and then an `_` or a character that is itself
/// written as an escape (`_xABCD` followed by U+0001 is written
/// `_xABCD_x0001_`, whose `_xABCD_` would be read as the escape of U+ABCD).
/// Any other `_` is written as it is.
///
/// Use it on an ST_Xstring value first, then apply the XML's escape
/// ([`xml_text`](crate::xml_text) or
/// [`xml_attribute`](crate::xml_attribute)) to what it gives: the
/// ST_Xstring value is made first, and the XML is written from it.
pub fn escape<E>(text: &str, mut out: impl FnMut(&str) -> Result<(), E>) -> Result<(), E> {
    let mut start = 0;
    for (at, c) in text.char_indices() {
        let after = at.saturating_add(c.len_utf8());
        let hex;
        let escaped: &str = if is_not_xml_char(c) {
            hex = format!("_x{:04X}_", u32::from(c));
            &hex
        } else if c == '_' && starts_escape(text.get(after..).unwrap_or_default()) {
            "_x005F_"
        } else {
            continue;
        };
        out(text.get(start..at).unwrap_or_default())?;
        out(escaped)?;
        start = after;
    }
    out(text.get(start..).unwrap_or_default())
}

/// Whether `c` is not a `Char` of XML 1.0's
/// grammar (U+0009, U+000A, U+000D, U+0020 to U+D7FF, U+E000 to U+FFFD and
/// U+10000 up).
fn is_not_xml_char(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}')
}

/// Whether an `_` followed by `rest` would be read as an escape by
/// [`unescape`] once what follows is written: `x`, four hexadecimal digits,
/// then an `_` or a character [`escape`] writes as an escape.
fn starts_escape(rest: &str) -> bool {
    let mut chars = rest.chars();
    chars.next() == Some('x')
        && (&mut chars).take(4).filter(char::is_ascii_hexdigit).count() == 4
        && chars.next().is_some_and(|c| c == '_' || is_not_xml_char(c))
}

#[cfg(test)]
mod tests;

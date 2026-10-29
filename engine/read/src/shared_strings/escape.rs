//! [`unescape`]: the `_xHHHH_` escapes of ST_Xstring (ECMA-376-1:2016
//! §22.9.2.19) undone, on a value that arrives in pieces.
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
#[derive(Debug, Default)]
pub(crate) struct Xstring {
    /// The end of the value read so far that may still start an escape,
    /// held back until more of it arrives.
    held: String,
}

impl Xstring {
    /// Starts a value.
    pub(crate) fn begin(&mut self) {
        self.held.clear();
    }

    /// Adds `text` to the value, its escapes undone and handed to `out`,
    /// holding back what may still start one unless it is the `last`.
    ///
    /// What was held back is joined to no more of `text` than finishes an
    /// escape it may start, so `text` is never copied whole.
    pub(crate) fn push<E>(
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
/// the caller hands the rest in again ahead of what follows.
pub(crate) fn unescape<E>(
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

#[cfg(test)]
mod tests;

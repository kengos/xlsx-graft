//! What the checks count: an unsigned number as a part writes one
//! ([`Unsigned`]), and the length of a string in UTF-16 code units, its
//! `_xHHHH_` escapes undone ([`Utf16`]), both read as the text arrives in
//! pieces, so a long one costs no more memory than a short one.

/// An `xsd:unsignedInt` or `xsd:unsignedLong` as it arrives, in pieces:
/// digits, a `+` before them or not, with XML white space around them.
///
/// A number past `u64::MAX` saturates there, so it reads as past any count
/// it is checked against.
#[derive(Debug, Default)]
pub(crate) struct Unsigned {
    /// The value of the digits so far.
    value: u64,
    /// Whether a digit was read.
    digits: bool,
    /// Whether a `+` was read.
    signed: bool,
    /// Whether white space came after the digits or the `+`.
    ended: bool,
    /// Whether something else came, or a `+` after the first character
    /// that is not white space.
    invalid: bool,
}

impl Unsigned {
    /// The number `text` holds whole, if it holds one.
    pub(crate) fn parse(text: &str) -> Option<u64> {
        let mut number = Self::default();
        number.push(text);
        number.value()
    }

    /// Adds `text` to the number.
    pub(crate) fn push(&mut self, text: &str) {
        for c in text.chars() {
            match c.to_digit(10) {
                Some(digit) if !self.ended => {
                    self.value = (self.value.saturating_mul(10)).saturating_add(u64::from(digit));
                    self.digits = true;
                }
                None if c == '+' && !self.digits && !self.signed => self.signed = true,
                None if matches!(c, ' ' | '\t' | '\r' | '\n') => {
                    self.ended |= self.digits || self.signed;
                }
                _ => self.invalid = true,
            }
        }
    }

    /// The number read, if the text held one.
    pub(crate) fn value(&self) -> Option<u64> {
        (self.digits && !self.invalid).then_some(self.value)
    }
}

/// The length of one ST_Xstring value (ECMA-376-1:2016 §22.9.2.19) in
/// UTF-16 code units, as it arrives in pieces: each `_xHHHH_` escape, four
/// hexadecimal digits in either case, counts as the one code unit it
/// stands for, a surrogate alone included, and anything else as written.
///
/// It is read once from the left, as the escapes are undone: `_x005F_` is
/// `_`, so `_x005F_x0041_` is seven units, not two.
#[derive(Debug, Default)]
pub(crate) struct Utf16 {
    /// The code units counted.
    units: u64,
    /// How many characters of an escape have been read (`_`, `_x`, then
    /// each digit), not yet counted: up to six.
    held: u64,
}

impl Utf16 {
    /// The code units counted, the characters held back included.
    pub(crate) fn units(&self) -> u64 {
        self.units.saturating_add(self.held)
    }

    /// Adds `text` to the value.
    pub(crate) fn push(&mut self, text: &str) {
        for c in text.chars() {
            self.char(c);
        }
    }

    /// Ends the value: what was held back counts as written.
    pub(crate) fn end(&mut self) {
        self.units = self.units();
        self.held = 0;
    }

    /// Adds `c` to the value.
    fn char(&mut self, c: char) {
        let next = match (self.held, c) {
            (0 | 1, '_') => Some(1),
            (1, 'x') => Some(2),
            (2..=5, c) if c.is_ascii_hexdigit() => Some(self.held + 1),
            (6, '_') => {
                self.units = self.units.saturating_add(1);
                self.held = 0;
                return;
            }
            _ => None,
        };
        match next {
            // A `_` after a `_` counts the first and may start an escape.
            Some(1) if self.held == 1 => self.units = self.units.saturating_add(1),
            Some(held) => self.held = held,
            None => {
                // What was held is no escape; `c` may start one.
                self.end();
                if c == '_' {
                    self.held = 1;
                } else {
                    self.units = self.units.saturating_add(u64::from(c.len_utf16() == 2) + 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

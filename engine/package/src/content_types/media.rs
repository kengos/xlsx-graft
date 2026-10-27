//! [`ContentType`]: a media type as RFC 7231 §3.1.1.1 spells one, with no
//! white space but around a parameter's `;` and no comments
//! (ECMA-376-2:2021 §7.2.3.2.4; 4th edition M1.14, M1.15).

/// A media type from the content types stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentType {
    /// The value as written, parameters included.
    value: String,
    /// `type/subtype`, lowered.
    essence: String,
}

impl ContentType {
    /// `value` as a media type, or `None` when it is not one.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        // `ST_ContentType` admits Latin-1 at most, in a quoted string.
        if value.chars().any(|c| u32::from(c) > 0xFF) {
            return None;
        }
        let mut cursor = Cursor {
            bytes: value.as_bytes(),
            at: 0,
        };
        let kind = cursor.token()?;
        cursor.byte(b'/')?;
        let subtype = cursor.token()?;
        while !cursor.is_done() {
            cursor.spaces();
            cursor.byte(b';')?;
            cursor.spaces();
            cursor.token()?;
            cursor.byte(b'=')?;
            if cursor.peek() == Some(b'"') {
                cursor.quoted()?;
            } else {
                cursor.token()?;
            }
        }
        Some(Self {
            value: value.to_owned(),
            essence: format!("{kind}/{subtype}").to_ascii_lowercase(),
        })
    }

    /// The media type as written, parameters included.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// The type and subtype, lowered: what two media types are compared by.
    /// Parameters are not compared.
    #[must_use]
    pub fn essence(&self) -> &str {
        &self.essence
    }
}

/// A position in a media type's bytes.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn is_done(&self) -> bool {
        self.at >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// Takes `byte`, or fails.
    fn byte(&mut self, byte: u8) -> Option<()> {
        (self.peek()? == byte).then(|| self.at += 1)
    }

    /// Takes `OWS`.
    fn spaces(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.at += 1;
        }
    }

    /// Takes a `token` (RFC 7230 §3.2.6): one or more `tchar`.
    fn token(&mut self) -> Option<&'a str> {
        let start = self.at;
        while self.peek().is_some_and(is_tchar) {
            self.at += 1;
        }
        let token = self.bytes.get(start..self.at).filter(|t| !t.is_empty())?;
        std::str::from_utf8(token).ok()
    }

    /// Takes a `quoted-string` (RFC 7230 §3.2.6).
    fn quoted(&mut self) -> Option<()> {
        self.byte(b'"')?;
        loop {
            match self.peek()? {
                b'"' => return self.byte(b'"'),
                b'\\' => {
                    self.at += 1;
                    let escaped = self.peek()?;
                    (escaped == b'\t' || escaped >= b' ') && escaped != 0x7f
                }
                byte => byte == b'\t' || (byte >= b' ' && byte != 0x7f),
            }
            .then(|| self.at += 1)?;
        }
    }
}

/// RFC 7230 `tchar`.
fn is_tchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

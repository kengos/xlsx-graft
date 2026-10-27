//! [`Fields`]: the little-endian fields of a zip record, read off its bytes
//! one after another, each `None` once the bytes run out.

/// A cursor over a record's bytes.
pub(crate) struct Fields<'a> {
    bytes: &'a [u8],
}

impl<'a> Fields<'a> {
    /// A cursor at the start of `bytes`.
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// The next `len` bytes.
    pub(crate) fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let (head, rest) = self.bytes.split_at_checked(len)?;
        self.bytes = rest;
        Some(head)
    }

    /// The next two bytes as a number.
    pub(crate) fn u16(&mut self) -> Option<u16> {
        self.take(2)?.try_into().ok().map(u16::from_le_bytes)
    }

    /// The next four bytes as a number.
    pub(crate) fn u32(&mut self) -> Option<u32> {
        self.take(4)?.try_into().ok().map(u32::from_le_bytes)
    }

    /// The next eight bytes as a number.
    pub(crate) fn u64(&mut self) -> Option<u64> {
        self.take(8)?.try_into().ok().map(u64::from_le_bytes)
    }

    /// The bytes not read yet.
    pub(crate) fn rest(&self) -> &'a [u8] {
        self.bytes
    }
}

/// The data of the zip64 extended information extra field (APPNOTE 6.3.10
/// §4.5.3, header ID 0x0001) among a header's `extra` fields, if there is one.
pub(crate) fn zip64_extra(extra: &[u8]) -> Option<&[u8]> {
    let mut fields = Fields::new(extra);
    loop {
        let id = fields.u16()?;
        let len = fields.u16()?;
        let data = fields.take(usize::from(len))?;
        if id == 0x0001 {
            return Some(data);
        }
    }
}

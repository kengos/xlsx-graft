//! A zip builder for the tests: it lays out local headers, data, data
//! descriptors, the central directory and the end records byte by byte, and
//! every field it writes can be set to a lie.
//!
//! [`Item`] is one entry: what its local header and its central directory
//! header say ([`Header`], equal unless a test changes one), its data, and an
//! optional data descriptor. [`Zip`] is the archive: its items, data in
//! front of them, zip64 end records, and what the end record says.

/// The fields a local header and a central directory header share.
#[derive(Clone, Debug)]
pub(crate) struct Header {
    pub(crate) name: Vec<u8>,
    pub(crate) flags: u16,
    pub(crate) method: u16,
    pub(crate) crc32: u32,
    pub(crate) compressed_size: u64,
    pub(crate) size: u64,
}

/// A data descriptor after an item's data.
#[derive(Clone, Debug)]
pub(crate) struct Descriptor {
    pub(crate) signature: bool,
    /// Write 8-byte sizes, as a zip64 item's descriptor has.
    pub(crate) wide: bool,
    pub(crate) crc32: u32,
    pub(crate) compressed_size: u64,
    pub(crate) size: u64,
}

/// One entry of a [`Zip`].
#[derive(Clone, Debug)]
pub(crate) struct Item {
    pub(crate) local: Header,
    pub(crate) central: Header,
    /// The bytes after the local header, as stored.
    pub(crate) data: Vec<u8>,
    pub(crate) descriptor: Option<Descriptor>,
    /// Write sizes and the offset as 0xFFFFFFFF, with a zip64 extra field.
    pub(crate) zip64: bool,
    /// Write no local header or data: the central header points elsewhere.
    pub(crate) no_local: bool,
    /// The local header offset the central header gives, instead of where
    /// the local header was written.
    pub(crate) offset: Option<u64>,
    /// The disk number the central header gives.
    pub(crate) disk: u16,
    /// Extra fields both headers carry ahead of any zip64 extra field.
    pub(crate) extra: Vec<u8>,
    /// Bytes after the record that no header accounts for.
    pub(crate) padding: Vec<u8>,
}

impl Item {
    /// An item whose data is `stored` as `data` and inflates to `inflated`.
    fn new(name: &str, method: u16, data: Vec<u8>, inflated: &[u8]) -> Self {
        let header = Header {
            name: name.as_bytes().to_vec(),
            flags: 0,
            method,
            crc32: crc32fast::hash(inflated),
            compressed_size: data.len() as u64,
            size: inflated.len() as u64,
        };
        Self {
            local: header.clone(),
            central: header,
            data,
            descriptor: None,
            zip64: false,
            no_local: false,
            offset: None,
            disk: 0,
            extra: Vec::new(),
            padding: Vec::new(),
        }
    }

    /// `data`, stored.
    pub(crate) fn stored(name: &str, data: &[u8]) -> Self {
        Self::new(name, 0, data.to_vec(), data)
    }

    /// `data`, deflated.
    pub(crate) fn deflated(name: &str, data: &[u8]) -> Self {
        Self::new(
            name,
            8,
            miniz_oxide::deflate::compress_to_vec(data, 6),
            data,
        )
    }

    /// This item with bit 3 set and a data descriptor holding the CRC-32 and
    /// the sizes, which the local header leaves as zero.
    pub(crate) fn with_descriptor(mut self, signature: bool) -> Self {
        self.local.flags |= 0x0008;
        self.central.flags |= 0x0008;
        self.descriptor = Some(Descriptor {
            signature,
            wide: self.zip64,
            crc32: self.local.crc32,
            compressed_size: self.local.compressed_size,
            size: self.local.size,
        });
        self.local.crc32 = 0;
        self.local.compressed_size = 0;
        self.local.size = 0;
        self
    }

    /// Both headers' `flags`.
    pub(crate) fn with_flags(mut self, flags: u16) -> Self {
        self.local.flags = flags;
        self.central.flags = flags;
        self
    }

    /// Both headers' `method`.
    pub(crate) fn with_method(mut self, method: u16) -> Self {
        self.local.method = method;
        self.central.method = method;
        self
    }

    /// The local header, as written at the start of the item.
    pub(crate) fn local_header(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let local = &self.local;
        put32(&mut out, 0x0403_4b50);
        put16(&mut out, 20);
        put16(&mut out, local.flags);
        put16(&mut out, local.method);
        put32(&mut out, 0);
        put32(&mut out, local.crc32);
        let extra = if self.zip64 {
            put32(&mut out, u32::MAX);
            put32(&mut out, u32::MAX);
            zip64_extra(&[local.size, local.compressed_size])
        } else {
            put32(&mut out, local.compressed_size as u32);
            put32(&mut out, local.size as u32);
            Vec::new()
        };
        let extra = [self.extra.as_slice(), &extra].concat();
        put16(&mut out, local.name.len() as u16);
        put16(&mut out, extra.len() as u16);
        out.extend(&local.name);
        out.extend(extra);
        out
    }

    /// The local header, the data and the data descriptor.
    fn local_record(&self) -> Vec<u8> {
        let mut out = self.local_header();
        out.extend(&self.data);
        if let Some(descriptor) = &self.descriptor {
            if descriptor.signature {
                put32(&mut out, 0x0807_4b50);
            }
            put32(&mut out, descriptor.crc32);
            if self.zip64 || descriptor.wide {
                put64(&mut out, descriptor.compressed_size);
                put64(&mut out, descriptor.size);
            } else {
                put32(&mut out, descriptor.compressed_size as u32);
                put32(&mut out, descriptor.size as u32);
            }
        }
        out.extend(&self.padding);
        out
    }

    /// The central directory header, for a local header at `offset`.
    fn central_header(&self, offset: u64) -> Vec<u8> {
        let mut out = Vec::new();
        let central = &self.central;
        let offset = self.offset.unwrap_or(offset);
        put32(&mut out, 0x0201_4b50);
        put16(&mut out, 20);
        put16(&mut out, 20);
        put16(&mut out, central.flags);
        put16(&mut out, central.method);
        put32(&mut out, 0);
        put32(&mut out, central.crc32);
        let extra = if self.zip64 {
            put32(&mut out, u32::MAX);
            put32(&mut out, u32::MAX);
            zip64_extra(&[central.size, central.compressed_size, offset])
        } else {
            put32(&mut out, central.compressed_size as u32);
            put32(&mut out, central.size as u32);
            Vec::new()
        };
        let extra = [self.extra.as_slice(), &extra].concat();
        put16(&mut out, central.name.len() as u16);
        put16(&mut out, extra.len() as u16);
        put16(&mut out, 0);
        put16(&mut out, self.disk);
        put16(&mut out, 0);
        put32(&mut out, 0);
        put32(&mut out, if self.zip64 { u32::MAX } else { offset as u32 });
        out.extend(&central.name);
        out.extend(extra);
        out
    }
}

/// What the end records say, where a test makes them lie.
#[derive(Clone, Debug, Default)]
pub(crate) struct EndLies {
    pub(crate) entries: Option<u64>,
    pub(crate) entries_on_disk: Option<u64>,
    pub(crate) size: Option<u64>,
    pub(crate) offset: Option<u64>,
    pub(crate) disk: u32,
    pub(crate) directory_disk: u32,
    /// The zip64 locator's disk of the zip64 record and its total disks.
    pub(crate) locator_disk: u32,
    pub(crate) disks: u32,
    /// Where the zip64 locator says the zip64 record is.
    pub(crate) zip64_offset: Option<u64>,
    /// The entry counts a zip64 archive's end record gives instead of
    /// leaving them to the zip64 record.
    pub(crate) plain_entries: Option<u16>,
}

/// A zip, built by [`Zip::build`].
#[derive(Clone, Debug, Default)]
pub(crate) struct Zip {
    pub(crate) items: Vec<Item>,
    /// Bytes in front of the first item, which the offsets count.
    pub(crate) prefix: Vec<u8>,
    /// Bytes at the end of the central directory, which its size counts.
    pub(crate) directory_tail: Vec<u8>,
    /// Write a zip64 record and locator, and saturate the end record.
    pub(crate) zip64: bool,
    pub(crate) comment: Vec<u8>,
    pub(crate) lies: EndLies,
}

impl Zip {
    /// A zip of `items`.
    pub(crate) fn of(items: Vec<Item>) -> Self {
        Self {
            items,
            ..Self::default()
        }
    }

    /// The zip's bytes.
    pub(crate) fn build(&self) -> Vec<u8> {
        let mut out = self.prefix.clone();
        let mut offsets = Vec::new();
        for item in &self.items {
            offsets.push(out.len() as u64);
            if !item.no_local {
                out.extend(item.local_record());
            }
        }
        let directory_offset = out.len() as u64;
        for (item, offset) in self.items.iter().zip(offsets) {
            out.extend(item.central_header(offset));
        }
        out.extend(&self.directory_tail);
        let lies = &self.lies;
        let entries = lies.entries.unwrap_or(self.items.len() as u64);
        let entries_on_disk = lies.entries_on_disk.unwrap_or(entries);
        let size = lies.size.unwrap_or(out.len() as u64 - directory_offset);
        let offset = lies.offset.unwrap_or(directory_offset);
        if self.zip64 {
            let record = out.len() as u64;
            put32(&mut out, 0x0606_4b50);
            put64(&mut out, 44);
            put16(&mut out, 45);
            put16(&mut out, 45);
            put32(&mut out, lies.disk);
            put32(&mut out, lies.directory_disk);
            put64(&mut out, entries_on_disk);
            put64(&mut out, entries);
            put64(&mut out, size);
            put64(&mut out, offset);
            put32(&mut out, 0x0706_4b50);
            put32(&mut out, lies.locator_disk);
            put64(&mut out, lies.zip64_offset.unwrap_or(record));
            put32(&mut out, lies.disks.max(1));
        }
        put32(&mut out, 0x0605_4b50);
        if self.zip64 {
            put16(&mut out, 0);
            put16(&mut out, 0);
            put16(&mut out, lies.plain_entries.unwrap_or(u16::MAX));
            put16(&mut out, lies.plain_entries.unwrap_or(u16::MAX));
            put32(&mut out, u32::MAX);
            put32(&mut out, u32::MAX);
        } else {
            put16(&mut out, lies.disk as u16);
            put16(&mut out, lies.directory_disk as u16);
            put16(&mut out, entries_on_disk as u16);
            put16(&mut out, entries as u16);
            put32(&mut out, size as u32);
            put32(&mut out, offset as u32);
        }
        put16(&mut out, self.comment.len() as u16);
        out.extend(&self.comment);
        out
    }
}

/// A zip64 extended information extra field holding `values`.
fn zip64_extra(values: &[u64]) -> Vec<u8> {
    let mut out = Vec::new();
    put16(&mut out, 0x0001);
    put16(&mut out, (values.len() * 8) as u16);
    for &value in values {
        put64(&mut out, value);
    }
    out
}

pub(crate) fn put16(out: &mut Vec<u8>, value: u16) {
    out.extend(value.to_le_bytes());
}

pub(crate) fn put32(out: &mut Vec<u8>, value: u32) {
    out.extend(value.to_le_bytes());
}

pub(crate) fn put64(out: &mut Vec<u8>, value: u64) {
    out.extend(value.to_le_bytes());
}

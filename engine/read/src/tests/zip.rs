//! A zip builder for the tests: stored entries laid out one after another,
//! the central directory and the end record. It writes only zips that
//! read; a header may lie only about the CRC-32 and the method, which the
//! zip reader checks as it reads an entry.
//!
//! The package crate's own builder writes every field as a test asks; it
//! is that crate's test code, so the few lines a well-formed zip takes are
//! written again here.

/// One entry: its name, its bytes, stored, and what its headers claim.
#[derive(Clone, Debug)]
pub(super) struct Item {
    name: String,
    data: Vec<u8>,
    crc32: u32,
    method: u16,
}

impl Item {
    /// `data`, stored as `name`.
    pub(super) fn stored(name: &str, data: &[u8]) -> Self {
        Self {
            name: name.to_owned(),
            data: data.to_vec(),
            crc32: crc32fast::hash(data),
            method: 0,
        }
    }

    /// This item, its bytes changed by `damage` after the CRC-32 was taken.
    pub(super) fn damaged(mut self, damage: impl FnOnce(&mut Vec<u8>)) -> Self {
        damage(&mut self.data);
        self
    }

    /// This item, its headers naming compression `method`.
    pub(super) fn with_method(mut self, method: u16) -> Self {
        self.method = method;
        self
    }
}

/// The zip of `items`, in their order.
pub(super) fn build(items: Vec<Item>) -> Vec<u8> {
    let mut zip = Vec::new();
    let mut directory = Vec::new();
    for item in &items {
        let offset = zip.len() as u32;
        let size = item.data.len() as u32;
        let name = item.name.as_bytes();
        zip.extend(0x0403_4b50_u32.to_le_bytes());
        // Version needed, flags, method, time and date.
        zip.extend([20, 0, 0, 0]);
        zip.extend(item.method.to_le_bytes());
        zip.extend([0; 4]);
        zip.extend(item.crc32.to_le_bytes());
        zip.extend(size.to_le_bytes());
        zip.extend(size.to_le_bytes());
        zip.extend((name.len() as u16).to_le_bytes());
        zip.extend([0; 2]);
        zip.extend(name);
        zip.extend(&item.data);

        directory.extend(0x0201_4b50_u32.to_le_bytes());
        // Version made by, version needed, flags, method, time and date.
        directory.extend([20, 0, 20, 0, 0, 0]);
        directory.extend(item.method.to_le_bytes());
        directory.extend([0; 4]);
        directory.extend(item.crc32.to_le_bytes());
        directory.extend(size.to_le_bytes());
        directory.extend(size.to_le_bytes());
        directory.extend((name.len() as u16).to_le_bytes());
        // Extra and comment lengths, disk, internal and external attributes.
        directory.extend([0; 12]);
        directory.extend(offset.to_le_bytes());
        directory.extend(name);
    }
    let start = zip.len() as u32;
    let count = (items.len() as u16).to_le_bytes();
    zip.extend(&directory);
    zip.extend(0x0605_4b50_u32.to_le_bytes());
    zip.extend([0; 4]);
    zip.extend(count);
    zip.extend(count);
    zip.extend((directory.len() as u32).to_le_bytes());
    zip.extend(start.to_le_bytes());
    zip.extend([0; 2]);
    zip
}

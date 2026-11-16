//! Helpers the unit tests of the modules share.

use serde_json::Value;
use xlsx_graft_diagnostics::{Diagnostic, Record};

/// The folder of the workbooks Excel saved, at the repository's root.
pub(crate) const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

/// The JSON `text` holds.
pub(crate) fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

/// What each record says, one string each: `<code> <key>|<reason>|<hint>`
/// for a fault of a setting, the code alone for any other diagnostic.
pub(crate) fn faults(records: &[Record]) -> Vec<String> {
    let said = |code: &str, key, reason, hint| format!("{code} {key}|{reason}|{hint}");
    (records.iter())
        .map(|record| match &record.diagnostic {
            Diagnostic::PolicyInvalid { key, reason, hint }
            | Diagnostic::RulesInvalid { key, reason, hint } => {
                said(record.diagnostic.code().as_str(), key, reason, hint)
            }
            other => other.code().as_str().to_owned(),
        })
        .collect()
}

/// A zip of the entries `(name, data, method)`, each laid out as the zip
/// format has it: a local record, then the central directory, then the end
/// record. `method` is the compression method both headers declare, and
/// `data` is written stored whatever `method` is: with 0 the entry is read,
/// and with 99 (a method the package reader does not support) it is refused.
pub(crate) fn zip_with_methods(entries: &[(&str, &str, u16)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for (name, data, method) in entries {
        let offset = u32::try_from(out.len()).unwrap();
        let crc = crc32(data.as_bytes());
        let size = u32::try_from(data.len()).unwrap();
        let name_len = u16::try_from(name.len()).unwrap();
        // The fields both headers share, from the flags to the name's length.
        let mut shared = Vec::new();
        shared.extend(0u16.to_le_bytes());
        shared.extend(method.to_le_bytes());
        shared.extend([0; 4]);
        shared.extend(crc.to_le_bytes());
        shared.extend(size.to_le_bytes());
        shared.extend(size.to_le_bytes());
        shared.extend(name_len.to_le_bytes());
        shared.extend(0u16.to_le_bytes());
        out.extend(0x0403_4b50u32.to_le_bytes());
        out.extend(20u16.to_le_bytes());
        out.extend(&shared);
        out.extend(name.as_bytes());
        out.extend(data.as_bytes());
        directory.extend(0x0201_4b50u32.to_le_bytes());
        directory.extend(20u16.to_le_bytes());
        directory.extend(20u16.to_le_bytes());
        directory.extend(&shared);
        // The comment's length, the disk, the internal and external
        // attributes, then where the local record starts.
        directory.extend([0; 10]);
        directory.extend(offset.to_le_bytes());
        directory.extend(name.as_bytes());
    }
    let count = u16::try_from(entries.len()).unwrap();
    let directory_offset = u32::try_from(out.len()).unwrap();
    out.extend(&directory);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0; 4]);
    out.extend(count.to_le_bytes());
    out.extend(count.to_le_bytes());
    out.extend(u32::try_from(directory.len()).unwrap().to_le_bytes());
    out.extend(directory_offset.to_le_bytes());
    out.extend([0; 2]);
    out
}

/// A zip of the stored entries `(name, data)`.
pub(crate) fn zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let with_methods: Vec<_> = (entries.iter())
        .map(|(name, data)| (*name, *data, 0))
        .collect();
    zip_with_methods(&with_methods)
}

/// The CRC-32 of `bytes`, as the zip format has it.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// The content types stream of a package: its `Default` elements give the
/// extensions `rels` and `xml` their media types, so a part named `*.bin`
/// has none.
pub(crate) const CONTENT_TYPES: &str = concat!(
    r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
    r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
    r#"<Default Extension="xml" ContentType="application/xml"/></Types>"#,
);

/// The zip of a package: [`CONTENT_TYPES`], then `entries`.
pub(crate) fn package(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut all = vec![("[Content_Types].xml", CONTENT_TYPES)];
    all.extend(entries);
    zip(&all)
}

/// The XML of a Relationships part holding `body`.
pub(crate) fn rels(body: &str) -> String {
    format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{body}</Relationships>"#
    )
}

/// An internal relationship `id` of the type `kind` (under a made-up
/// namespace) to `target`.
pub(crate) fn rel(id: &str, kind: &str, target: &str) -> String {
    rel_of(id, &format!("urn:x/{kind}"), target)
}

/// An internal relationship `id` of the type `type_uri` to `target`.
pub(crate) fn rel_of(id: &str, type_uri: &str, target: &str) -> String {
    format!(r#"<Relationship Id="{id}" Type="{type_uri}" Target="{target}"/>"#)
}

/// The package's own Relationships part, naming `xl/workbook.xml` as the main
/// document (the `officeDocument` relationship of the namespace Excel
/// writes).
pub(crate) fn main_rels() -> String {
    let office_document =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
    rels(&rel_of("rId1", office_document, "xl/workbook.xml"))
}

//! Refusing one entry at a time: its name, encryption and method, its local
//! header and data descriptor against the central directory, and its data
//! against the file's end, the directory and the other entries. Each refused
//! entry stays listed, not readable, with one diagnostic.

use super::builder::{Descriptor, Item, Zip};
use super::{Reader, limit, readable};
use crate::ZipArchive;
use crate::ZipLimits;
use std::cell::Cell;
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, ZipHeaderField};

/// Which entries of `zip` are readable, and what was found, at the default
/// limits.
fn check(zip: &Zip) -> (Vec<bool>, Vec<(Diagnostic, String)>) {
    readable(zip.build(), &ZipLimits::default())
}

/// Asserts that the entry `a` alone in a zip, changed by `change`, is
/// refused with `diagnostic`.
fn refuses(change: impl FnOnce(&mut Item), diagnostic: Diagnostic) {
    let mut item = Item::stored("a", b"hello");
    change(&mut item);
    let (readable, found) = check(&Zip::of(vec![item]));
    assert_eq!(readable, [false]);
    assert_eq!(found, [(diagnostic, "a".to_owned())]);
}

/// The diagnostic for a header disagreeing on `field`.
fn mismatch(field: ZipHeaderField) -> Diagnostic {
    Diagnostic::ZipHeaderMismatch { field }
}

#[test]
fn a_name_at_the_limit_is_read_and_one_past_it_is_refused() {
    let at = "a".repeat(1_024);
    let past = "b".repeat(1_025);
    let zip = Zip::of(vec![Item::stored(&at, b"x"), Item::stored(&past, b"y")]);
    let (readable, found) = check(&zip);
    assert_eq!(readable, [true, false]);
    let refused = Diagnostic::EntryNameTooLong {
        length: 1_025,
        limit: limit(1_024),
    };
    assert_eq!(found, [(refused, Echo::new(&past).as_str().to_owned())]);
}

#[test]
fn an_encrypted_entry_is_refused() {
    for flags in [0x0001, 0x0040, 0x0041, 0x2000] {
        refuses(
            |item| *item = item.clone().with_flags(flags),
            Diagnostic::ZipEncryptedEntry {},
        );
    }
}

#[test]
fn a_method_other_than_stored_or_deflate_is_refused() {
    refuses(
        |item| *item = item.clone().with_method(12),
        Diagnostic::ZipUnsupportedCompression { method: 12 },
    );
}

#[test]
fn an_entry_gets_only_the_first_diagnostic_found() {
    let long = "a".repeat(1_025);
    let item = Item::stored(&long, b"x").with_flags(0x0001).with_method(12);
    let (readable, found) = check(&Zip::of(vec![item]));
    assert_eq!(readable, [false]);
    assert!(matches!(
        found.as_slice(),
        [(Diagnostic::EntryNameTooLong { .. }, _)]
    ));
}

#[test]
fn a_local_header_disagreeing_with_the_directory_is_refused() {
    refuses(
        |item| item.local.name = b"b".to_vec(),
        mismatch(ZipHeaderField::Name),
    );
    refuses(
        |item| item.local.flags = 0x0002,
        mismatch(ZipHeaderField::Flags),
    );
    refuses(
        |item| item.local.method = 8,
        mismatch(ZipHeaderField::Method),
    );
    refuses(
        |item| item.local.crc32 ^= 1,
        mismatch(ZipHeaderField::Crc32),
    );
    refuses(
        |item| item.local.compressed_size += 1,
        mismatch(ZipHeaderField::CompressedSize),
    );
    refuses(
        |item| item.local.size += 1,
        mismatch(ZipHeaderField::UncompressedSize),
    );
}

#[test]
fn no_local_header_where_the_directory_points_is_a_signature_mismatch() {
    let mut bytes = Zip::of(vec![Item::stored("a", b"hello")]).build();
    bytes[0] = b'p';
    let (readable, found) = readable(bytes, &ZipLimits::default());
    assert_eq!(readable, [false]);
    assert_eq!(
        found,
        [(mismatch(ZipHeaderField::Signature), "a".to_owned())]
    );
}

#[test]
fn a_data_descriptor_holds_what_the_local_header_leaves_as_zero() {
    for signature in [true, false] {
        let zip = Zip::of(vec![
            Item::deflated("a", b"hello").with_descriptor(signature),
            Item::stored("b", b"world"),
        ]);
        assert_eq!(check(&zip), (vec![true, true], vec![]));
    }
    // A local header may hold the values even with bit 3 set.
    let mut item = Item::stored("a", b"hello").with_descriptor(true);
    item.local = item.central.clone();
    assert_eq!(check(&Zip::of(vec![item])), (vec![true], vec![]));
}

#[test]
fn a_zip64_data_descriptor_has_wide_sizes() {
    let mut item = Item::deflated("a", b"hello").with_descriptor(false);
    item.zip64 = true;
    let zip = Zip::of(vec![item, Item::stored("b", b"world")]);
    assert_eq!(check(&zip), (vec![true, true], vec![]));
}

#[test]
fn a_data_descriptor_disagreeing_with_the_directory_is_refused() {
    fn descriptor(item: &mut Item) -> &mut Descriptor {
        *item = item.clone().with_descriptor(true);
        item.descriptor.as_mut().unwrap()
    }
    refuses(
        |item| descriptor(item).crc32 ^= 1,
        mismatch(ZipHeaderField::Crc32),
    );
    refuses(
        |item| descriptor(item).compressed_size += 1,
        mismatch(ZipHeaderField::CompressedSize),
    );
    refuses(
        |item| descriptor(item).size += 1,
        mismatch(ZipHeaderField::UncompressedSize),
    );
}

#[test]
fn a_local_record_running_past_the_end_of_the_file_is_truncated() {
    let truncated = || (Diagnostic::ZipTruncated {}, "b".to_owned());
    // The name and extra field after the local header: patch b's extra
    // field length to 0xffff.
    let mut bytes = Zip::of(vec![
        Item::stored("a", b"hello"),
        Item::stored("b", b"world"),
    ])
    .build();
    bytes[36 + 28..36 + 30].copy_from_slice(&[0xff, 0xff]);
    assert_eq!(
        readable(bytes, &ZipLimits::default()),
        (vec![true, false], vec![truncated()])
    );
    // The data.
    let mut long = Item::stored("b", b"world");
    long.local.compressed_size = 1_000_000;
    long.central.compressed_size = 1_000_000;
    let zip = Zip::of(vec![Item::stored("a", b"hello"), long]);
    assert_eq!(check(&zip), (vec![true, false], vec![truncated()]));
}

#[test]
fn a_local_name_of_another_length_is_refused_before_it_is_read() {
    let mut bytes = Zip::of(vec![Item::stored("a", b"hello")]).build();
    // The name and extra field lengths: 0 and 1 instead of 1 and 0, so the
    // record keeps its length.
    bytes[26..30].copy_from_slice(&[0, 0, 1, 0]);
    assert_eq!(
        readable(bytes, &ZipLimits::default()),
        (
            vec![false],
            vec![(mismatch(ZipHeaderField::Name), "a".to_owned())]
        )
    );
}

#[test]
fn a_data_descriptor_is_told_by_its_length() {
    // 8-byte sizes without a zip64 extra field, as Java's ZipOutputStream
    // writes them.
    for signature in [true, false] {
        let mut item = Item::deflated("a", b"hello").with_descriptor(signature);
        item.descriptor.as_mut().unwrap().wide = true;
        let zip = Zip::of(vec![item, Item::stored("b", b"world")]);
        assert_eq!(check(&zip), (vec![true, true], vec![]));
    }
}

#[test]
fn a_data_descriptor_of_no_descriptor_length_is_refused() {
    // None at all, 8 bytes, and 12 bytes followed by 4 more: 16 bytes that
    // do not start with the signature.
    let mut none = Item::stored("a", b"hello").with_descriptor(false);
    none.descriptor = None;
    let mut short = none.clone();
    short.padding = vec![0; 8];
    let mut long = Item::stored("a", b"hello").with_descriptor(false);
    long.padding = vec![0; 4];
    let mut padded = none.clone();
    padded.padding = vec![0; 4 << 20];
    for item in [none, short, long, padded.clone()] {
        let zip = Zip::of(vec![item, Item::stored("b", b"world")]);
        assert_eq!(
            check(&zip),
            (
                vec![false, true],
                vec![(mismatch(ZipHeaderField::Descriptor), "a".to_owned())]
            )
        );
    }
    // Megabytes up to the next record are refused before they are read: the
    // end records' search reads the last 64 KiB, the rest is headers.
    let read = Rc::new(Cell::new(0));
    let zip = Zip::of(vec![padded, Item::stored("b", b"world")]).build();
    let reader = Reader::counted(zip, read.clone());
    ZipArchive::open(reader, &ZipLimits::default(), &mut Diagnostics::new()).unwrap();
    assert!(read.get() < 100 * 1024, "read {} bytes", read.get());
}

#[test]
fn bytes_no_record_accounts_for_are_reported_and_the_entries_still_read() {
    let mut a = Item::stored("a", b"hello");
    a.padding = vec![0; 7];
    let mut b = Item::stored("b", b"world");
    b.padding = vec![0; 3];
    let unlisted = |size, entry: &str| (Diagnostic::ZipUnlistedData { size }, entry.to_owned());
    assert_eq!(
        check(&Zip::of(vec![a, b])),
        (vec![true, true], vec![unlisted(7, "b"), unlisted(3, "")])
    );
}

#[test]
fn a_refused_entry_still_takes_its_bytes() {
    let zip = Zip::of(vec![
        Item::stored("a", b"hello").with_flags(0x0001),
        Item::stored("b", b"world"),
    ]);
    assert_eq!(
        check(&zip),
        (
            vec![false, true],
            vec![(Diagnostic::ZipEncryptedEntry {}, "a".to_owned())]
        )
    );
}

#[test]
fn no_gap_is_reported_after_a_record_whose_end_is_not_known() {
    let mut bytes = Zip::of(vec![
        Item::stored("a", b"hello"),
        Item::stored("b", b"world"),
    ])
    .build();
    bytes[0] = b'p';
    assert_eq!(
        readable(bytes, &ZipLimits::default()),
        (
            vec![false, true],
            vec![(mismatch(ZipHeaderField::Signature), "a".to_owned())]
        )
    );
}

#[test]
fn an_entry_hidden_in_another_entrys_data_overlaps_it() {
    let mut hidden = Item::stored("b", b"world");
    hidden.no_local = true;
    // a's data starts after its local header (30 + 1).
    hidden.offset = Some(31);
    let mut data = hidden.local_header();
    data.extend(&hidden.data);
    let outer = Item::stored("a", &data);
    let overlap = (
        Diagnostic::ZipEntriesOverlap {
            other: Echo::new("a"),
        },
        "b".to_owned(),
    );
    let zip = Zip::of(vec![outer.clone(), hidden.clone()]);
    assert_eq!(check(&zip), (vec![true, false], vec![overlap.clone()]));
    // The one that starts first is kept, whichever the directory lists first.
    let zip = Zip::of(vec![hidden, outer]);
    assert_eq!(check(&zip), (vec![false, true], vec![overlap]));
}

#[test]
fn an_entry_after_a_shorter_one_is_checked_against_the_longest_so_far() {
    // b sits inside a's data; c starts inside a's data too, after b ends.
    let mut b = Item::stored("b", b"x");
    b.no_local = true;
    b.offset = Some(31);
    let mut c = Item::stored("c", b"y");
    c.no_local = true;
    c.offset = Some(31 + 32);
    let mut data = b.local_header();
    data.extend(&b.data);
    data.extend(c.local_header());
    data.extend(&c.data);
    let zip = Zip::of(vec![Item::stored("a", &data), b, c]);
    let (readable, found) = check(&zip);
    assert_eq!(readable, [true, false, false]);
    let others: Vec<String> = found
        .iter()
        .map(|(diagnostic, _)| match diagnostic {
            Diagnostic::ZipEntriesOverlap { other } => other.as_str().to_owned(),
            other => panic!("expected an overlap, got {other:?}"),
        })
        .collect();
    assert_eq!(others, ["a", "a"]);
}

#[test]
fn an_entry_running_into_the_central_directory_is_refused() {
    let overlaps = || {
        (
            vec![true, false],
            vec![(Diagnostic::ZipEntryOverlapsDirectory {}, "b".to_owned())],
        )
    };
    let mut item = Item::stored("b", b"world");
    item.local.compressed_size += 10;
    item.central.compressed_size += 10;
    let zip = Zip::of(vec![Item::stored("a", b"hello"), item]);
    assert_eq!(check(&zip), overlaps());
    // One that starts past it is refused before its local header is read.
    let mut far = Item::stored("b", b"world");
    far.offset = Some(1 << 32);
    far.zip64 = true;
    far.no_local = true;
    let zip = Zip::of(vec![Item::stored("a", b"hello"), far]);
    assert_eq!(check(&zip), overlaps());
}

#[test]
fn entries_with_the_same_name_are_both_listed() {
    let zip = Zip::of(vec![
        Item::stored("a", b"hello"),
        Item::stored("a", b"world"),
    ]);
    let (archive, found) = super::open(zip.build());
    let archive = archive.unwrap();
    assert!(found.is_empty());
    assert_eq!(archive.entries().len(), 2);
    assert_eq!(archive.position(b"a"), Some(0));
    assert_eq!(archive.position(b"b"), None);
}

#[test]
fn a_data_descriptor_whose_crc32_equals_its_signature_is_read_either_way() {
    for signature in [true, false] {
        let mut item = Item::stored("a", b"hello").with_descriptor(signature);
        item.central.crc32 = 0x0807_4b50;
        item.descriptor.as_mut().unwrap().crc32 = 0x0807_4b50;
        let zip = Zip::of(vec![item, Item::stored("b", b"world")]);
        assert_eq!(check(&zip), (vec![true, true], vec![]));
    }
}

#[test]
fn an_entry_starting_inside_an_earlier_one_is_refused_before_it_is_read() {
    // b points at a's local header; read, it would disagree on the name.
    let mut b = Item::stored("b", b"world");
    b.no_local = true;
    b.offset = Some(0);
    let zip = Zip::of(vec![Item::stored("a", b"hello"), b.clone(), b]);
    let overlap = || {
        (
            Diagnostic::ZipEntriesOverlap {
                other: Echo::new("a"),
            },
            "b".to_owned(),
        )
    };
    assert_eq!(
        check(&zip),
        (vec![true, false, false], vec![overlap(), overlap()])
    );
}

#[test]
fn a_refused_entry_keeps_the_bytes_read_for_it() {
    // a is refused on its flags, but its record still takes its bytes; b,
    // hidden in a's name, overlaps them.
    let mut a = Item::stored("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", b"hello");
    a.local.flags = 0x0002;
    let mut b = Item::stored("b", b"world");
    b.no_local = true;
    b.offset = Some(30 + 4);
    let zip = Zip::of(vec![a, b]);
    let (readable, found) = check(&zip);
    assert_eq!(readable, [false, false]);
    assert_eq!(found[0].0, mismatch(ZipHeaderField::Flags));
    assert!(
        matches!(&found[1].0, Diagnostic::ZipEntriesOverlap { other } if other.as_str().starts_with("aaa"))
    );
}

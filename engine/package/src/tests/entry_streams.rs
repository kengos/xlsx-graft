//! Reading an entry through `stream_entry`: streams kept beside the archive
//! and read in turns, between reads of other entries and of the same entry;
//! an entry another reader refused meanwhile, before or partway through the
//! stream; and a stream handed an archive without its entry, before and
//! after it ends.

use super::builder::{Item, Zip};
use super::{Archive, Found, found, noise, open};
use std::io::{self, Read};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics};

/// The archive of `items`, which must open without a diagnostic.
fn archive(items: Vec<Item>) -> Archive {
    let (archive, found) = open(Zip::of(items).build());
    assert!(found.is_empty(), "{found:?}");
    archive.unwrap()
}

#[test]
fn streams_read_in_turns_each_read_their_own_entry() {
    let large = noise(100_000);
    let mut archive = archive(vec![
        Item::deflated("a", &large),
        Item::stored("b", &noise(70_000)),
    ]);
    let mut a = archive.stream_entry(0).unwrap();
    let mut b = archive.stream_entry(1).unwrap();
    let mut diagnostics = Diagnostics::new();
    let (mut read_a, mut read_b) = (Vec::new(), Vec::new());
    let mut out = [0; 1000];
    loop {
        let len_a = a.read(&mut archive, &mut out, &mut diagnostics).unwrap();
        read_a.extend(&out[..len_a]);
        // Another read of the archive in between moves its reader.
        let mut entry = archive.open_entry(1, &mut diagnostics).unwrap().unwrap();
        entry.read_exact(&mut [0; 10]).unwrap();
        drop(entry);
        let len_b = b.read(&mut archive, &mut out, &mut diagnostics).unwrap();
        read_b.extend(&out[..len_b]);
        if len_a == 0 && len_b == 0 {
            break;
        }
    }
    assert_eq!((read_a, read_b), (large, noise(70_000)));
    assert_eq!(a.read(&mut archive, &mut out, &mut diagnostics).unwrap(), 0);
    assert!(!a.is_stopped() && !b.is_stopped());
    assert!(diagnostics.records().is_empty());
}

#[test]
fn two_streams_of_one_entry_each_keep_their_own_place() {
    let large = noise(100_000);
    for item in [Item::stored("a", &large), Item::deflated("a", &large)] {
        let mut archive = archive(vec![item]);
        let mut first = archive.stream_entry(0).unwrap();
        let mut second = archive.stream_entry(0).unwrap();
        let mut diagnostics = Diagnostics::new();
        let mut out = [0; 3000];
        let len = first
            .read(&mut archive, &mut out, &mut diagnostics)
            .unwrap();
        let mut read_first = out[..len].to_vec();
        let mut read_second = Vec::new();
        loop {
            let len = second
                .read(&mut archive, &mut out, &mut diagnostics)
                .unwrap();
            read_second.extend(&out[..len]);
            let len_first = first
                .read(&mut archive, &mut out, &mut diagnostics)
                .unwrap();
            read_first.extend(&out[..len_first]);
            if len == 0 && len_first == 0 {
                break;
            }
        }
        assert_eq!((&read_first, &read_second), (&large, &large));
        assert!(diagnostics.records().is_empty());
    }
}

#[test]
fn an_entry_refused_while_a_stream_is_partway_stops_it_without_a_second_diagnostic() {
    let mut item = Item::deflated("a", &noise(100_000));
    item.central.crc32 ^= 1;
    item.local.crc32 ^= 1;
    let mut archive = archive(vec![item]);
    let mut partway = archive.stream_entry(0).unwrap();
    let mut diagnostics = Diagnostics::new();
    let mut out = [0; 1000];
    assert!(
        partway
            .read(&mut archive, &mut out, &mut diagnostics)
            .unwrap()
            > 0
    );
    let mut entry = archive.open_entry(0, &mut diagnostics).unwrap().unwrap();
    assert!(entry.read_to_end(&mut Vec::new()).is_err());
    drop(entry);
    assert_eq!(diagnostics.records().len(), 1);
    assert!(
        partway
            .read(&mut archive, &mut out, &mut diagnostics)
            .is_err()
    );
    assert!(partway.is_stopped());
    assert_eq!(diagnostics.records().len(), 1);
}

#[test]
fn an_entry_another_reader_refused_stops_the_stream_without_a_second_diagnostic() {
    let mut item = Item::stored("a", b"hello");
    item.central.crc32 ^= 1;
    item.local.crc32 ^= 1;
    let mut archive = archive(vec![item]);
    let mut early = archive.stream_entry(0).unwrap();
    let mut diagnostics = Diagnostics::new();
    let mut entry = archive.open_entry(0, &mut diagnostics).unwrap().unwrap();
    assert!(entry.read_to_end(&mut Vec::new()).is_err());
    assert!(entry.is_stopped());
    drop(entry);
    let found: Found = found(diagnostics);
    assert!(matches!(
        found[..],
        [(Diagnostic::ZipCrcMismatch { .. }, _)]
    ));

    let mut diagnostics = Diagnostics::new();
    let error = early.read(&mut archive, &mut [0; 8], &mut diagnostics);
    assert_eq!(error.unwrap_err().kind(), io::ErrorKind::InvalidData);
    assert!(early.is_stopped());
    assert!(
        early
            .read(&mut archive, &mut [0; 8], &mut diagnostics)
            .is_err()
    );
    assert!(diagnostics.records().is_empty());
    assert!(archive.stream_entry(0).is_none());
    assert!(archive.stream_entry(1).is_none());
}

#[test]
fn a_stream_handed_an_archive_without_its_entry_fails() {
    let mut two = archive(vec![Item::stored("a", b"x"), Item::stored("b", b"y")]);
    let mut one = archive(vec![Item::stored("a", b"x")]);
    let mut stream = two.stream_entry(1).unwrap();
    let mut diagnostics = Diagnostics::new();
    let error = stream.read(&mut one, &mut [0; 8], &mut diagnostics);
    assert_eq!(
        error.unwrap_err().to_string(),
        "the archive has no such zip entry"
    );
    // An entry at the same index with other data is not the stream's
    // either, and is left alone.
    let mut other = archive(vec![Item::stored("a", b"x"), Item::stored("b", b"z")]);
    let mut stream = two.stream_entry(1).unwrap();
    let error = stream.read(&mut other, &mut [0; 8], &mut diagnostics);
    assert_eq!(
        error.unwrap_err().to_string(),
        "the archive has no such zip entry"
    );
    assert!(other.entries()[1].is_readable());
    let error = stream.read(&mut two, &mut [0; 8], &mut diagnostics);
    assert_eq!(
        error.unwrap_err().to_string(),
        "an earlier read of the zip entry failed"
    );
    assert!(!stream.is_stopped());
    assert!(diagnostics.records().is_empty());
}

#[test]
fn a_stream_that_ended_or_stopped_stays_so_whatever_archive_it_is_handed() {
    let mut item = Item::stored("b", b"y");
    item.central.crc32 ^= 1;
    item.local.crc32 ^= 1;
    let mut two = archive(vec![Item::stored("a", b"x"), item]);
    let mut one = archive(vec![Item::stored("a", b"x")]);
    let mut done = two.stream_entry(0).unwrap();
    let mut stopped = two.stream_entry(1).unwrap();
    let mut diagnostics = Diagnostics::new();
    let mut out = [0; 8];
    assert_eq!(done.read(&mut two, &mut out, &mut diagnostics).unwrap(), 1);
    assert_eq!(done.read(&mut two, &mut out, &mut diagnostics).unwrap(), 0);
    assert_eq!(
        stopped.read(&mut two, &mut out, &mut diagnostics).unwrap(),
        1
    );
    assert!(stopped.read(&mut two, &mut out, &mut diagnostics).is_err());
    assert_eq!(diagnostics.records().len(), 1);
    assert_eq!(done.read(&mut one, &mut out, &mut diagnostics).unwrap(), 0);
    assert!(stopped.read(&mut one, &mut out, &mut diagnostics).is_err());
    assert!(stopped.is_stopped());
    assert_eq!(diagnostics.records().len(), 1);
}

//! What every [`Spool`] does, run on a [`MemorySpool`] and on a
//! [`FileSpool`](crate::FileSpool) in a temporary file alike: append, read
//! any range back, refuse a range past the end.

use crate::{MemorySpool, MemorySpools, Spool, SpoolFactory, TempFiles};
use std::io;

/// Each spool the crate makes, empty, under a name for the failures.
fn spools() -> Vec<(&'static str, Box<dyn Spool>)> {
    vec![
        ("memory", Box::new(MemorySpool::new())),
        ("memory factory", MemorySpools.create().unwrap()),
        ("temp file", TempFiles::new().create().unwrap()),
    ]
}

/// What reading `len` bytes from `offset` gives.
fn read(spool: &mut dyn Spool, offset: u64, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; len];
    spool.read_exact_at(offset, &mut buf)?;
    Ok(buf)
}

/// Every range of `expected`, and every range one past its end, read
/// back from `spool`.
fn check_every_range(name: &str, spool: &mut dyn Spool, expected: &[u8]) {
    for offset in 0..=expected.len() {
        for len in 0..=expected.len() - offset {
            let got = read(spool, offset as u64, len).unwrap();
            assert_eq!(
                got,
                &expected[offset..offset + len],
                "{name} {offset}+{len}"
            );
        }
        let past = expected.len() - offset + 1;
        let error = read(spool, offset as u64, past).unwrap_err();
        assert_eq!(
            error.kind(),
            io::ErrorKind::UnexpectedEof,
            "{name} {offset}+{past}"
        );
    }
}

#[test]
fn an_empty_spool_reads_nothing_but_an_empty_range_at_zero() {
    for (name, mut spool) in spools() {
        assert_eq!(read(&mut *spool, 0, 0).unwrap(), b"", "{name}");
        for (offset, len) in [(0, 1), (1, 0), (u64::MAX, 0), (u64::MAX, 1)] {
            let error = read(&mut *spool, offset, len).unwrap_err();
            assert_eq!(
                error.kind(),
                io::ErrorKind::UnexpectedEof,
                "{name} {offset}+{len}"
            );
        }
    }
}

#[test]
fn every_range_of_what_was_appended_reads_back_between_appends() {
    for (name, mut spool) in spools() {
        let mut expected = Vec::new();
        for bytes in [&b"abc"[..], b"", b"d", b"efgh"] {
            spool.append(bytes).unwrap();
            expected.extend_from_slice(bytes);
            check_every_range(name, &mut *spool, &expected);
        }
        let error = read(&mut *spool, u64::MAX, 2).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof, "{name}");
    }
}

#[test]
fn a_read_does_not_move_where_the_next_append_goes() {
    for (name, mut spool) in spools() {
        spool.append(b"abcd").unwrap();
        assert_eq!(read(&mut *spool, 1, 1).unwrap(), b"b", "{name}");
        spool.append(b"ef").unwrap();
        assert_eq!(read(&mut *spool, 0, 6).unwrap(), b"abcdef", "{name}");
    }
}

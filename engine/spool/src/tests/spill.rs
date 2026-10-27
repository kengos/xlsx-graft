//! [`SpillBuffer`]: every range reads back before and after the spill and
//! between appends, the write buffer holds what it was given and no more,
//! and a spool that fails leaves the buffer as it was.

use super::FlakySpools;
use crate::{Fill, MemorySpools, SpillBuffer, SpoolFactory, TempFiles};
use std::io;

/// Every range of `expected` read back from `buffer`, and every range one
/// past its end refused, with the buffer left as it was.
fn check_every_range(name: &str, buffer: &mut SpillBuffer, expected: &[u8]) {
    assert_eq!(buffer.len(), expected.len() as u64, "{name}");
    assert_eq!(buffer.is_empty(), expected.is_empty(), "{name}");
    for offset in 0..=expected.len() {
        for len in 0..=expected.len() - offset {
            let mut buf = vec![0; len];
            let fill = buffer.read_exact_at(offset as u64, &mut buf).unwrap();
            assert_eq!(fill, Fill::Filled, "{name} {offset}+{len}");
            assert_eq!(
                buf,
                &expected[offset..offset + len],
                "{name} {offset}+{len}"
            );
        }
        let mut buf = vec![b'?'; expected.len() - offset + 1];
        let fill = buffer.read_exact_at(offset as u64, &mut buf).unwrap();
        assert_eq!(fill, Fill::OutOfRange, "{name} {offset}");
        assert!(buf.iter().all(|&b| b == b'?'), "{name} {offset}");
    }
    for len in [0, 1] {
        let mut buf = vec![0; len];
        let fill = buffer.read_exact_at(u64::MAX, &mut buf).unwrap();
        assert_eq!(fill, Fill::OutOfRange, "{name} {len}");
    }
}

/// Appends `parts` one by one, checking every range after each.
fn append_all(name: &str, buffer: &mut SpillBuffer, expected: &mut Vec<u8>, parts: &[&[u8]]) {
    for part in parts {
        buffer.append(part).unwrap();
        expected.extend_from_slice(part);
        check_every_range(name, buffer, expected);
    }
}

/// The factories the crate has, under a name for the failures.
fn factories() -> Vec<(&'static str, Box<dyn SpoolFactory>)> {
    vec![
        ("memory", Box::new(MemorySpools)),
        ("temp file", Box::new(TempFiles::new())),
    ]
}

#[test]
fn before_the_spill_everything_is_in_memory() {
    let mut buffer = SpillBuffer::default();
    let mut expected = Vec::new();
    check_every_range("empty", &mut buffer, &expected);
    append_all("memory", &mut buffer, &mut expected, &[b"abc", b"", b"de"]);
    assert!(!buffer.is_spilled());
    assert!(buffer.memory_bytes() >= 5);
}

#[test]
fn after_the_spill_every_range_reads_back_across_spool_and_buffer() {
    for (name, mut spools) in factories() {
        let mut buffer = SpillBuffer::new();
        let mut expected = Vec::new();
        append_all(name, &mut buffer, &mut expected, &[b"abcdef"]);
        buffer.spill(&mut *spools, 4).unwrap();
        assert!(buffer.is_spilled(), "{name}");
        assert_eq!(buffer.memory_bytes(), 0, "{name}");
        check_every_range(name, &mut buffer, &expected);
        // "gh" and "ij" fill the write buffer exactly, "k" sends them to the
        // spool, "lmnop" is larger than the buffer and goes straight after
        // "k", "qr" stays in the buffer.
        append_all(
            name,
            &mut buffer,
            &mut expected,
            &[b"gh", b"ij", b"k", b"lmnop", b"", b"qr"],
        );
        assert_eq!(buffer.memory_bytes(), 4, "{name}");
    }
}

#[test]
fn an_empty_buffer_spills_too() {
    let mut buffer = SpillBuffer::new();
    buffer.spill(&mut MemorySpools, 2).unwrap();
    let mut expected = Vec::new();
    check_every_range("empty", &mut buffer, &expected);
    append_all(
        "spilled",
        &mut buffer,
        &mut expected,
        &[b"ab", b"c", b"defg"],
    );
}

#[test]
fn with_no_write_buffer_every_append_goes_to_the_spool() {
    let mut buffer = SpillBuffer::new();
    buffer.spill(&mut MemorySpools, 0).unwrap();
    let mut expected = Vec::new();
    append_all(
        "unbuffered",
        &mut buffer,
        &mut expected,
        &[b"a", b"", b"bc"],
    );
    assert_eq!(buffer.memory_bytes(), 0);
}

#[test]
fn a_second_spill_changes_nothing() {
    let mut spools = FlakySpools::default();
    let mut buffer = SpillBuffer::new();
    buffer.append(b"abc").unwrap();
    buffer.spill(&mut spools, 2).unwrap();
    buffer.spill(&mut spools, 8).unwrap();
    assert_eq!(spools.created, 1);
    assert_eq!(buffer.memory_bytes(), 0);
    check_every_range("twice", &mut buffer, b"abc");
}

#[test]
fn a_spill_that_fails_leaves_the_bytes_in_memory() {
    let mut buffer = SpillBuffer::new();
    buffer.append(b"abc").unwrap();

    let mut refusing = FlakySpools {
        refusing: true,
        ..FlakySpools::default()
    };
    assert!(buffer.spill(&mut refusing, 2).is_err());
    let mut failing = FlakySpools::default();
    failing.budget.set(Some(0));
    assert!(buffer.spill(&mut failing, 2).is_err());
    assert!(!buffer.is_spilled());
    check_every_range("unspilled", &mut buffer, b"abc");

    buffer.spill(&mut MemorySpools, 2).unwrap();
    assert!(buffer.is_spilled());
    check_every_range("spilled at last", &mut buffer, b"abc");
}

#[test]
fn the_write_buffer_is_allocated_at_the_first_append_after_the_spill() {
    let mut buffer = SpillBuffer::new();
    buffer.append(b"abc").unwrap();
    buffer.spill(&mut MemorySpools, usize::MAX).unwrap();
    assert_eq!(buffer.memory_bytes(), 0);
    let crate::SpoolError::Io(error) = buffer.append(b"d").unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::OutOfMemory);
    check_every_range("unallocated", &mut buffer, b"abc");
}

#[test]
fn a_spool_that_fails_after_the_spill_fails_the_call() {
    let mut spools = FlakySpools::default();
    let budget = std::rc::Rc::clone(&spools.budget);
    let mut buffer = SpillBuffer::new();
    buffer.append(b"ab").unwrap();
    buffer.spill(&mut spools, 2).unwrap();
    buffer.append(b"cd").unwrap();

    budget.set(Some(0));
    // Sending the full write buffer to the spool fails, before "e" or "efg"
    // is written, and so does a read that reaches the spool.
    assert!(buffer.append(b"e").is_err());
    assert!(buffer.append(b"efg").is_err());
    assert!(buffer.read_exact_at(0, &mut [0; 1]).is_err());
    assert_eq!(buffer.len(), 4);
    // What is in the write buffer still reads back.
    let mut buf = [0; 2];
    assert_eq!(buffer.read_exact_at(2, &mut buf).unwrap(), Fill::Filled);
    assert_eq!(&buf, b"cd");

    // The write buffer reaches the spool, then "efg", larger than it, fails.
    budget.set(Some(1));
    assert!(buffer.append(b"efg").is_err());
    budget.set(None);
    assert_eq!(buffer.memory_bytes(), 2);
    let mut expected = b"abcd".to_vec();
    check_every_range("half failed", &mut buffer, &expected);
    append_all("recovered", &mut buffer, &mut expected, &[b"e", b"fgh"]);
}

#[test]
fn debug_says_what_the_buffer_holds() {
    let mut buffer = SpillBuffer::new();
    buffer.append(b"abc").unwrap();
    buffer.spill(&mut MemorySpools, 2).unwrap();
    buffer.append(b"d").unwrap();
    assert_eq!(
        format!("{buffer:?}"),
        "SpillBuffer { len: 4, spooled: 3, memory_bytes: 2, buffer: Some(2), .. }"
    );
}

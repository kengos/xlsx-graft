//! [`SpillBuffer::read_at`]: as many bytes as there are from any offset,
//! before and after the spill, across the spool and the write buffer, 0 at
//! and past the end, and an error when the spool fails.

use super::FlakySpools;
use crate::{MemorySpools, SpillBuffer, SpoolFactory, TempFiles};

/// `len` bytes that differ from place to place.
fn bytes(len: usize) -> Vec<u8> {
    (0..len).map(|n| (n * 7 + n / 251) as u8).collect()
}

/// Every offset up to and past the end of `expected` read with buffers of
/// every length in `lens`: the bytes that exist, and their count.
fn check(name: &str, buffer: &mut SpillBuffer, expected: &[u8], lens: &[usize]) {
    for offset in (0..=expected.len() + 2).chain([usize::MAX / 2]) {
        for &len in lens {
            let mut buf = vec![b'?'; len];
            let read = buffer.read_at(offset as u64, &mut buf).unwrap();
            let left = expected.len().saturating_sub(offset);
            assert_eq!(read, left.min(len), "{name} {offset}+{len}");
            if read > 0 {
                assert_eq!(buf[..read], expected[offset..offset + read], "{name}");
            }
            // The rest of the buffer is as it was.
            assert!(buf[read..].iter().all(|&b| b == b'?'), "{name}");
        }
    }
    let mut buf = [b'?'; 4];
    assert_eq!(buffer.read_at(u64::MAX, &mut buf).unwrap(), 0, "{name}");
}

#[test]
fn it_reads_what_there_is_before_and_after_the_spill() {
    let factories: [(&str, Box<dyn SpoolFactory>); 2] = [
        ("memory", Box::new(MemorySpools)),
        ("temp file", Box::new(TempFiles::new())),
    ];
    let lens = [0, 1, 2, 7, 10, 11, 40];
    for (name, mut factory) in factories {
        // Around the write buffer's size, 10: under it, at it, over it, and
        // over a few buffers' worth.
        for total in [0, 1, 9, 10, 11, 25] {
            let data = bytes(total);
            let mut buffer = SpillBuffer::new();
            buffer.append(&data[..total / 2]).unwrap();
            check(name, &mut buffer, &data[..total / 2], &lens);
            buffer.spill(&mut *factory, 10).unwrap();
            buffer.append(&data[total / 2..]).unwrap();
            check(name, &mut buffer, &data, &lens);
        }
    }
}

#[test]
fn a_buffer_larger_than_a_chunk_is_read_in_chunks_to_its_end() {
    let data = bytes(200_000);
    let mut spools = MemorySpools;
    let mut buffer = SpillBuffer::new();
    buffer.append(&data[..1_000]).unwrap();
    buffer.spill(&mut spools, 100).unwrap();
    buffer.append(&data[1_000..]).unwrap();
    let (mut out, mut chunk, mut at) = (Vec::<u8>::new(), vec![0; 65_536], 0);
    loop {
        let read = buffer.read_at(at, &mut chunk).unwrap();
        if read == 0 {
            break;
        }
        out.extend(&chunk[..read]);
        at += read as u64;
    }
    assert_eq!(out, data);
}

#[test]
fn a_spool_that_fails_to_read_is_an_error() {
    let mut spools = FlakySpools::default();
    let mut buffer = SpillBuffer::new();
    buffer.spill(&mut spools, 100).unwrap();
    buffer.append(&bytes(1_000)).unwrap();
    spools.budget.set(Some(0));
    let error = buffer.read_at(0, &mut [0; 10]).unwrap_err();
    assert!(error.to_string().contains("the disk went away"));
    // Past the end, nothing is read, so nothing fails.
    assert_eq!(buffer.read_at(1_000, &mut [0; 10]).unwrap(), 0);
}

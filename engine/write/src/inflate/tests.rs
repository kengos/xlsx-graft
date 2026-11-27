//! `Inflater`: the bytes it gives back for what `DeflateWriter` wrote, in
//! memory or spilled, at the end of the stream, for a stream that is not
//! whole, and for a spool that fails, with `fill` and as a `Read`.

use super::Inflater;
use crate::tests::{Unreadables, deflated, noise};
use crate::{DeflateWriter, WriteError};
use std::io::{self, Read, Write};
use xlsx_graft_spool::SpillBuffer;

/// Reads `buffer` to its end through `Inflater::fill` in pieces of `piece`
/// bytes. Each call before the end returns at least 1 and at most `piece`,
/// which bounds the loop: a `fill` that never returns 0 fails the test.
/// After the end, `fill` returns 0 again, every time.
fn fill_all(buffer: &mut SpillBuffer, piece: usize, total: usize) -> Vec<u8> {
    let mut inflater = Inflater::new(buffer);
    let mut out = Vec::new();
    let mut chunk = vec![0; piece];
    for _ in 0..=total {
        let written = inflater.fill(&mut chunk).unwrap();
        assert!(written <= piece);
        if written == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..written]);
    }
    assert!(out.len() <= total, "the stream did not end");
    for _ in 0..2 {
        assert_eq!(inflater.fill(&mut chunk).unwrap(), 0);
    }
    out
}

#[test]
fn what_deflate_writer_wrote_is_inflated_back_whether_it_spilled_or_not() {
    let data = noise(200_000);
    for spill_at in [u64::MAX, 1_000] {
        let mut content = deflated(&data, spill_at);
        assert_eq!(content.buffer.is_spilled(), spill_at != u64::MAX);
        for piece in [1, 7, 16_384, 70_000] {
            assert_eq!(
                fill_all(&mut content.buffer, piece, data.len()),
                data,
                "{piece}"
            );
        }
        // Bounded, so a reader that never ends fails the comparison.
        let mut read = Vec::new();
        let bound = data.len() as u64 + 1;
        Inflater::new(&mut content.buffer)
            .take(bound)
            .read_to_end(&mut read)
            .unwrap();
        assert_eq!(read, data);
    }
}

#[test]
fn a_stream_of_no_bytes_ends_at_once_and_a_read_with_no_room_reads_nothing() {
    let mut content = deflated(b"", 1_000);
    assert!(fill_all(&mut content.buffer, 16, 0).is_empty());
    let mut inflater = Inflater::new(&mut content.buffer);
    assert_eq!(inflater.read(&mut []).unwrap(), 0);
    let mut byte = [0; 1];
    assert_eq!(inflater.read(&mut byte).unwrap(), 0);
}

#[test]
fn bytes_that_are_not_a_whole_deflate_stream_are_invalid_data() {
    let data = noise(5_000);
    let mut whole = deflated(&data, u64::MAX).buffer;
    let mut bytes = vec![0; whole.len() as usize];
    assert_eq!(whole.read_at(0, &mut bytes).unwrap(), bytes.len());
    let reserved = [0xff; 16].to_vec();
    let half = bytes[..bytes.len() / 2].to_vec();
    // One byte: the first call of the decompressor writes nothing.
    let one = bytes[..1].to_vec();
    for stored in [reserved, half, one, Vec::new()] {
        let mut buffer = SpillBuffer::new();
        buffer.append(&stored).unwrap();
        let mut inflater = Inflater::new(&mut buffer);
        let mut out = [0; 64];
        // For the half, the bytes before the cut inflate and the failure comes
        // at the cut; for the others it comes at once.
        let error = (0..=5_000)
            .find_map(|_| inflater.fill(&mut out).err())
            .expect("fill returned no error in 5,001 calls");
        assert!(
            matches!(&error, WriteError::Io(e) if e.kind() == io::ErrorKind::InvalidData),
            "{error:?}"
        );
        // Bounded, so a reader that never ends does not hang the test.
        let error = Inflater::new(&mut buffer)
            .take(1 << 22)
            .read_to_end(&mut Vec::new())
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(
            WriteError::from(error)
                .to_string()
                .contains("writing the zip failed")
        );
    }
}

/// A buffer spilled to a spool that cannot be read.
fn unreadable() -> SpillBuffer {
    let mut spools = Unreadables;
    let mut writer = DeflateWriter::new(&mut spools, 0);
    writer.write_all(&noise(300_000)).unwrap();
    writer.finish().unwrap().buffer
}

#[test]
fn a_spool_that_cannot_be_read_is_a_spool_error_and_a_read_error_that_carries_it() {
    let mut buffer = unreadable();
    let error = Inflater::new(&mut buffer).fill(&mut [0; 64]).unwrap_err();
    assert!(matches!(error, WriteError::Spool(_)), "{error:?}");
    let error = Inflater::new(&mut buffer).read(&mut [0; 64]).unwrap_err();
    assert!(
        error.to_string().contains("the spool cannot be read"),
        "{error}"
    );
    let error = WriteError::from(error);
    assert!(matches!(error, WriteError::Spool(_)), "{error:?}");
}

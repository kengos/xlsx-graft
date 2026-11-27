//! The deflated rows a `SpooledRows` holds: the same output spilled to a
//! spool or not and past the chunk sizes, a spool that fails, and a stored
//! stream that is not valid deflate.

use super::{num, render};
use crate::tests::{Sink, Unreadables};
use crate::{Row, RowCell, RowsWriter, SpooledRows, WriteError};
use std::io;
use xlsx_graft_cell::{CellValue, DateSystem};
use xlsx_graft_spool::{MemorySpools, SpillBuffer, Spool, SpoolFactory};

/// Spools that cannot be made.
struct Failing;

impl SpoolFactory for Failing {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        Err(io::Error::other("no space for a spool"))
    }
}

/// `len` lowercase letters that deflate poorly, from `seed`.
fn letters(seed: u32, len: usize) -> String {
    let mut state = seed.wrapping_mul(2_654_435_761) | 1;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            char::from(b'a' + state.to_le_bytes()[0] % 26)
        })
        .collect()
}

/// 400 rows of a number and a 200-letter string, with the output they make
/// when written.
fn rows_and_text() -> (Vec<String>, String) {
    let strings: Vec<String> = (0..400).map(|i| letters(i + 1, 200)).collect();
    let mut expected = String::new();
    for (i, text) in strings.iter().enumerate() {
        let r = i + 1;
        expected.push_str(&format!(
            r#"<row r="{r}"><c r="A{r}"><v>{r}</v></c><c r="B{r}" t="inlineStr"><is><t>{text}</t></is></c></row>"#
        ));
    }
    (strings, expected)
}

fn write_all(spools: &mut dyn SpoolFactory, spill_at: u64, strings: &[String]) -> SpooledRows {
    let mut writer = RowsWriter::new(spools, spill_at, DateSystem::Base1900);
    for (i, text) in strings.iter().enumerate() {
        let cells = [
            num(1, (i + 1) as f64),
            RowCell::new(2, None, CellValue::String(text)),
        ];
        writer.push(&Row::new(i as u32 + 1, &cells)).unwrap();
    }
    writer.finish().unwrap()
}

#[test]
fn rows_past_the_chunk_sizes_are_the_same_in_memory_and_spilled() {
    let (strings, expected) = rows_and_text();
    let mut memory = write_all(&mut MemorySpools, u64::MAX, &strings);
    assert!(!memory.buffer.is_spilled());
    assert!(memory.buffer.len() > 32_768 && expected.len() > 65_536);
    let mut spilled = write_all(&mut MemorySpools, 0, &strings);
    assert!(spilled.buffer.is_spilled());
    assert_eq!(render(&mut memory), expected);
    assert_eq!(render(&mut spilled), expected);
}

#[test]
fn a_spool_that_cannot_be_made_fails_the_push_and_then_the_finish() {
    let mut failing = Failing;
    let mut writer = RowsWriter::new(&mut failing, 100, DateSystem::Base1900);
    // The compressor holds the deflated output until a block is full: this
    // much text passes `spill_at` in one of the pushes.
    let mut failed = None;
    for number in 1..=1_000 {
        let text = letters(number, 1_000);
        let cells = [RowCell::new(1, None, CellValue::String(&text))];
        if let Err(error) = writer.push(&Row::new(number, &cells)) {
            failed = Some(error);
            break;
        }
    }
    let error = failed.unwrap();
    assert!(matches!(error, WriteError::Spool(_)), "{error:?}");
    assert!(error.to_string().contains("no space for a spool"));
    // A push after the failed one returns `Unusable`, not another `Spool`
    // error or `Ok`.
    let error = writer.push(&Row::new(5_000, &[num(1, 1.0)])).unwrap_err();
    assert!(matches!(error, WriteError::Unusable), "{error:?}");
    assert!(matches!(writer.finish(), Err(WriteError::Unusable)));

    let mut failing = Failing;
    let mut writer = RowsWriter::new(&mut failing, 0, DateSystem::Base1900);
    writer.push(&Row::new(1, &[num(1, 1.0)])).unwrap();
    let error = writer.finish().err().unwrap();
    assert!(matches!(error, WriteError::Spool(_)), "{error:?}");
}

/// A `SpooledRows` whose stored bytes are `bytes`, which no writer made.
fn stored(bytes: &[u8]) -> SpooledRows {
    let mut buffer = SpillBuffer::new();
    buffer.append(bytes).unwrap();
    SpooledRows {
        buffer,
        extent: None,
        truncated: 0,
    }
}

#[test]
fn stored_bytes_that_are_not_a_deflate_stream_are_an_error_not_output() {
    // A block of type 3, which RFC 1951 reserves.
    let mut out = Sink::failing_after(1 << 22);
    let error = stored(&[0xff; 16]).write_sheet_data(&mut out).unwrap_err();
    assert!(
        matches!(&error, WriteError::Io(e) if e.kind() == io::ErrorKind::InvalidData),
        "{error:?}"
    );
}

#[test]
fn stored_bytes_that_end_before_the_stream_does_are_an_error() {
    let (strings, _) = rows_and_text();
    let whole = write_all(&mut MemorySpools, u64::MAX, &strings);
    let mut bytes = vec![0; whole.buffer.len() as usize];
    let mut buffer = whole.buffer;
    assert_eq!(buffer.read_at(0, &mut bytes).unwrap(), bytes.len());
    for keep in [0, 1, bytes.len() / 2, bytes.len() - 1] {
        let mut out = Sink::failing_after(1 << 22);
        let error = stored(&bytes[..keep])
            .write_sheet_data(&mut out)
            .unwrap_err();
        assert!(
            matches!(&error, WriteError::Io(e) if e.kind() == io::ErrorKind::InvalidData),
            "keep {keep}: {error:?}"
        );
    }
}

#[test]
fn a_spool_that_fails_to_read_fails_write_sheet_data() {
    // Past the 65,536 bytes the buffer keeps in memory once spilled, so
    // bytes are written to the spool.
    let strings: Vec<String> = (0..1_000).map(|i| letters(i + 1, 200)).collect();
    let mut rows = write_all(&mut Unreadables, 0, &strings);
    let error = rows
        .write_sheet_data(&mut Sink::failing_after(1 << 22))
        .unwrap_err();
    assert!(matches!(error, WriteError::Spool(_)), "{error:?}");
}

#[test]
fn rows_that_inflate_to_many_chunks_from_one_read_of_stored_bytes_are_written_whole() {
    // Repeated text deflates to a few hundred bytes, read at once, which
    // inflate expands to more than two output chunks of 16,384 bytes.
    let text = "a".repeat(30_000);
    let mut spools = MemorySpools;
    let mut writer = RowsWriter::new(&mut spools, u64::MAX, DateSystem::Base1900);
    let mut expected = String::new();
    for number in 1..=10_u32 {
        let cells = [RowCell::new(1, None, CellValue::String(&text))];
        writer.push(&Row::new(number, &cells)).unwrap();
        expected.push_str(&format!(
            r#"<row r="{number}"><c r="A{number}" t="inlineStr"><is><t>{text}</t></is></c></row>"#
        ));
    }
    let mut rows = writer.finish().unwrap();
    assert!(rows.buffer.len() < 16_384 && expected.len() > 2 * 16_384);
    assert_eq!(render(&mut rows), expected);
}

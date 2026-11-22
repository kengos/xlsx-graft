//! `DeflateWriter`: what it writes inflates back, whether it spilled to a
//! spool or stayed in memory, and what a spool that fails does.

use super::{deflated, noise, open, read};
use crate::{DeflateWriter, WriteError, ZipWriter};
use std::cell::Cell;
use std::io::{self, Write};
use std::rc::Rc;
use xlsx_graft_spool::{Spool, SpoolFactory};

use super::Sink;

/// Spools that fail to be made while `failing` is set.
struct Failing(Rc<Cell<bool>>);

impl SpoolFactory for Failing {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        if self.0.get() {
            return Err(io::Error::other("no space for a spool"));
        }
        Ok(Box::new(xlsx_graft_spool::MemorySpool::new()))
    }
}

/// The zip of one entry `data`, deflated with `spill_at`.
fn zip(data: &[u8], spill_at: u64) -> Vec<u8> {
    let mut zip = ZipWriter::new(Sink::default());
    zip.write_deflated(b"a", deflated(data, spill_at)).unwrap();
    zip.finish().unwrap().bytes
}

#[test]
fn what_it_writes_is_the_same_spilled_or_not_and_inflates_back() {
    // Over a few output chunks, in many small writes and one large one.
    let data = noise(300_000);
    let memory = zip(&data, u64::MAX);
    for spill_at in [0, 1, 1_000, 100_000, 299_999] {
        assert_eq!(zip(&data, spill_at), memory, "spill_at {spill_at}");
    }
    let mut archive = open(memory);
    assert_eq!(read(&mut archive, 0), data);

    let mut spools = xlsx_graft_spool::MemorySpools;
    let mut writer = DeflateWriter::new(&mut spools, 1_000);
    for piece in data.chunks(777) {
        writer.write_all(piece).unwrap();
        writer.flush().unwrap();
    }
    let mut again = ZipWriter::new(Sink::default());
    again
        .write_deflated(b"a", writer.finish().unwrap())
        .unwrap();
    assert_eq!(again.finish().unwrap().bytes, zip(&data, 0));
}

#[test]
fn nothing_written_is_an_empty_entry() {
    let mut archive = open(zip(b"", 0));
    assert_eq!(read(&mut archive, 0), b"");
    assert_eq!(archive.entries()[0].size(), 0);
}

#[test]
fn a_spool_that_cannot_be_made_fails_the_write_and_the_finish() {
    let failing = Rc::new(Cell::new(true));
    let mut spools = Failing(failing.clone());
    let mut writer = DeflateWriter::new(&mut spools, 100);
    // Deflated output stays in the compressor until it is finished or its
    // window fills: this much noise passes `spill_at` in a write.
    let error = writer.write_all(&noise(100_000)).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(error.to_string().contains("no space for a spool"));
    // A write that failed leaves a CRC-32 and a size that count its bytes.
    assert!(matches!(writer.finish(), Err(WriteError::Unusable)));

    let mut spools = Failing(failing.clone());
    let mut writer = DeflateWriter::new(&mut spools, 0);
    writer.write_all(b"small").unwrap();
    let error = writer.finish().err().unwrap();
    assert!(matches!(error, WriteError::Spool(_)));
    assert!(error.to_string().contains("no space for a spool"));

    // The spool is only asked for once the output passes `spill_at`.
    let mut spools = Failing(failing);
    let mut writer = DeflateWriter::new(&mut spools, u64::MAX);
    writer.write_all(&noise(100_000)).unwrap();
    assert!(writer.finish().is_ok());
}

/// Writes `data` through a writer whose spools cannot be made, as a caller
/// that returns `WriteError` does, with `?`.
fn write_with_failing_spools(data: &[u8]) -> Result<(), WriteError> {
    let mut spools = Failing(Rc::new(Cell::new(true)));
    let mut writer = DeflateWriter::new(&mut spools, 100);
    writer.write_all(data)?;
    Ok(())
}

#[test]
fn a_spool_failure_inside_write_keeps_its_variant_through_a_question_mark() {
    let error = write_with_failing_spools(&noise(100_000)).unwrap_err();
    assert!(matches!(error, WriteError::Spool(_)), "{error:?}");
    assert!(error.to_string().contains("no space for a spool"));
}

#[test]
fn a_plain_io_error_converts_to_the_io_variant_with_its_kind() {
    let error = WriteError::from(io::Error::new(io::ErrorKind::NotFound, "gone"));
    assert!(matches!(&error, WriteError::Io(e) if e.kind() == io::ErrorKind::NotFound));
    assert_eq!(error.to_string(), "writing the zip failed: gone");
}

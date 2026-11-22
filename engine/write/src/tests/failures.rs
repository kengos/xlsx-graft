//! What the writer refuses and how the failures of the host come out: an
//! entry that is not as declared, one that is not readable, a name no zip
//! holds, and a writer that fails at each byte of the zip.

use super::{Sink, deflated, noise, open, open_with, read, stored, written};
use crate::{DeflateWriter, WriteError, ZipWriter};
use std::io::{self, Cursor, Write};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics};
use xlsx_graft_package::{ZipArchive, ZipLimits};
use xlsx_graft_spool::{Spool, SpoolFactory};

/// `zip` with its first entry's CRC-32 changed in both headers.
fn with_wrong_crc(mut zip: Vec<u8>) -> Vec<u8> {
    zip[14] ^= 1;
    let central = zip.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
    zip[central + 16] ^= 1;
    zip
}

/// `zip` with its first entry's method changed to 12 in both headers.
fn with_method_12(mut zip: Vec<u8>) -> Vec<u8> {
    zip[8] = 12;
    let central = zip.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
    zip[central + 10] = 12;
    zip
}

#[test]
fn an_entry_with_another_crc32_is_refused_with_a_diagnostic() {
    for zip in [stored(&[(b"a", b"hello")]), written(&[(b"a", b"hello")])] {
        let mut archive = open(with_wrong_crc(zip));
        let mut zip = ZipWriter::new(Sink::default());
        let mut diagnostics = Diagnostics::new();
        assert!(!zip.copy_entry(&mut archive, 0, &mut diagnostics).unwrap());
        let records = diagnostics.records();
        assert!(matches!(
            records,
            [record] if matches!(record.diagnostic, Diagnostic::ZipCrcMismatch { .. })
        ));
    }
}

#[test]
fn an_entry_that_is_not_readable_or_not_there_is_not_copied() {
    let bytes = with_method_12(stored(&[(b"a", b"hello")]));
    let mut archive = ZipArchive::open(
        Cursor::new(bytes),
        &ZipLimits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    assert!(!archive.entries()[0].is_readable());
    let mut zip = ZipWriter::new(Sink::default());
    let mut diagnostics = Diagnostics::new();
    for index in [0, 1] {
        assert!(
            !zip.copy_entry(&mut archive, index, &mut diagnostics)
                .unwrap()
        );
    }
    assert!(diagnostics.records().is_empty());
    // Nothing of them was written.
    assert!(zip.finish().unwrap().bytes.len() <= 22);
}

#[test]
fn a_name_longer_than_a_zip_holds_is_refused() {
    let mut zip = ZipWriter::new(Sink::default());
    let name = vec![b'a'; 65_536];
    let error = zip
        .write_deflated(&name, deflated(b"x", 1 << 20))
        .unwrap_err();
    assert!(matches!(
        error,
        WriteError::EntryNameTooLong { length: 65_536 }
    ));
    assert_eq!(
        error.to_string(),
        "an entry name of 65536 bytes is longer than a zip holds"
    );
    // Nothing was written: the writer takes the longest name a zip holds,
    // and finishes a zip that reads back.
    let name = vec![b'a'; 65_535];
    zip.write_deflated(&name, deflated(b"x", 1 << 20)).unwrap();
    let limits = ZipLimits {
        max_entry_name_bytes: xlsx_graft_diagnostics::Limit::policy_of(65_535),
        ..ZipLimits::default()
    };
    let archive = open_with(zip.finish().unwrap().bytes, &limits);
    assert_eq!(archive.entries().len(), 1);
    assert_eq!(archive.entries()[0].name().len(), 65_535);
}

/// Writes a copied stored entry, a copied deflated entry and a new entry
/// to `sink`, finishing the zip; on a failure, the stage it failed in: 1
/// and 2 for the copies, 3 for the new entry, 4 for the finish.
fn write_all(sink: Sink) -> Result<Vec<u8>, (u8, WriteError)> {
    let mut sources = [
        open(stored(&[(b"s", b"stored bytes")])),
        open(written(&[(b"d", &[b'd'; 3_000])])),
    ];
    let mut zip = ZipWriter::new(sink);
    let mut diagnostics = Diagnostics::new();
    for (stage, source) in (1..).zip(&mut sources) {
        let copied = zip.copy_entry(source, 0, &mut diagnostics);
        assert!(copied.map_err(|error| (stage, error))?);
    }
    zip.write_deflated(b"n", deflated(&[b'n'; 200_000], 100))
        .map_err(|error| (3, error))?;
    Ok(zip.finish().map_err(|error| (4, error))?.bytes)
}

#[test]
fn a_writer_that_fails_at_any_byte_fails_the_call_writing_it_and_the_later_ones_are_refused() {
    let whole = write_all(Sink::default()).unwrap();
    let mut stages = Vec::new();
    for limit in 0..whole.len() {
        let (stage, error) = write_all(Sink::failing_after(limit)).unwrap_err();
        assert!(matches!(error, WriteError::Io(_)), "{error:?}");
        stages.push(stage);
    }
    // The stages come in order, and each one is reached.
    assert!(stages.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!((stages[0], stages[stages.len() - 1]), (1, 4));
    for stage in 1..=4 {
        assert!(stages.contains(&stage), "stage {stage}");
    }
    assert_eq!(write_all(Sink::failing_after(whole.len())).unwrap(), whole);
    let (_, error) = write_all(Sink::failing_after(0)).unwrap_err();
    assert_eq!(
        error.to_string(),
        "writing the zip failed: the disk is full"
    );
}

#[test]
fn after_a_failure_the_writer_refuses_every_call() {
    let mut source = open(stored(&[(b"s", b"stored bytes")]));
    let mut diagnostics = Diagnostics::new();
    let mut zip = ZipWriter::new(Sink::failing_after(10));
    assert!(matches!(
        zip.copy_entry(&mut source, 0, &mut diagnostics),
        Err(WriteError::Io(_))
    ));
    assert!(matches!(
        zip.copy_entry(&mut source, 0, &mut diagnostics),
        Err(WriteError::Unusable)
    ));
    assert!(matches!(
        zip.write_deflated(b"a", deflated(b"x", 1 << 20)),
        Err(WriteError::Unusable)
    ));
    let error = zip.finish().unwrap_err();
    assert_eq!(
        error.to_string(),
        "an earlier call failed or left a partial entry: the output is not a zip"
    );
}

#[test]
fn an_entry_refused_after_its_header_was_written_leaves_the_writer_unusable() {
    let mut refused = open(with_wrong_crc(stored(&[(b"a", b"hello")])));
    let mut diagnostics = Diagnostics::new();
    let mut zip = ZipWriter::new(Sink::default());
    assert!(!zip.copy_entry(&mut refused, 0, &mut diagnostics).unwrap());
    assert_eq!(diagnostics.records().len(), 1);
    let mut good = open(stored(&[(b"b", b"fine")]));
    assert!(matches!(
        zip.copy_entry(&mut good, 0, &mut diagnostics),
        Err(WriteError::Unusable)
    ));
    assert!(matches!(
        zip.write_deflated(b"a", deflated(b"x", 1 << 20)),
        Err(WriteError::Unusable)
    ));
    assert!(matches!(zip.finish(), Err(WriteError::Unusable)));
}

#[test]
fn an_entry_that_is_not_copied_because_it_is_not_there_leaves_the_writer_usable() {
    let mut source = open(stored(&[(b"a", b"hello")]));
    let mut diagnostics = Diagnostics::new();
    let mut zip = ZipWriter::new(Sink::default());
    assert!(!zip.copy_entry(&mut source, 5, &mut diagnostics).unwrap());
    assert!(zip.copy_entry(&mut source, 0, &mut diagnostics).unwrap());
    zip.write_deflated(b"b", deflated(b"x", 1 << 20)).unwrap();
    let bytes = zip.finish().unwrap().bytes;
    let archive = open(bytes);
    assert_eq!(archive.entries().len(), 2);
}

/// A spool that appends but cannot read back.
struct WriteOnly;

impl Spool for WriteOnly {
    fn append(&mut self, _: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn read_exact_at(&mut self, _: u64, _: &mut [u8]) -> io::Result<()> {
        Err(io::Error::other("the disk went bad"))
    }
}

struct WriteOnlySpools;

impl SpoolFactory for WriteOnlySpools {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        Ok(Box::new(WriteOnly))
    }
}

#[test]
fn a_spool_that_cannot_be_read_back_is_a_scratch_storage_error_and_leaves_the_writer_unusable() {
    let mut spools = WriteOnlySpools;
    let mut writer = DeflateWriter::new(&mut spools, 0);
    let data = noise(200_000);
    writer.write_all(&data).unwrap();
    let content = writer.finish().unwrap();
    let mut zip = ZipWriter::new(Sink::default());
    let error = zip.write_deflated(b"a", content).unwrap_err();
    assert!(matches!(error, WriteError::Spool(_)), "{error:?}");
    assert!(error.to_string().contains("the disk went bad"));
    // The local header was written before the read failed.
    assert!(matches!(
        zip.write_deflated(b"b", deflated(b"x", 1 << 20)),
        Err(WriteError::Unusable)
    ));
    assert!(matches!(zip.finish(), Err(WriteError::Unusable)));
}

/// A writer whose second write is interrupted once.
struct Interrupted {
    bytes: Vec<u8>,
    writes: usize,
}

impl Write for Interrupted {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        if self.writes == 2 {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        self.bytes.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn an_interrupted_write_of_the_content_is_retried() {
    // The first write is the local header, the second the content.
    let mut zip = ZipWriter::new(Interrupted {
        bytes: Vec::new(),
        writes: 0,
    });
    zip.write_deflated(b"a", deflated(b"hello", 1 << 20))
        .unwrap();
    let mut archive = open(zip.finish().unwrap().bytes);
    assert_eq!(read(&mut archive, 0), b"hello");
}

/// A writer that takes `limit` bytes, then accepts none (`Ok(0)`), as a
/// full device can.
struct Full {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for Full {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len().min(self.limit - self.bytes.len());
        self.bytes.extend(&buf[..len]);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn an_output_that_takes_no_more_bytes_is_an_output_error() {
    // The local header of the entry "a" is 31 bytes: the content does not
    // fit after it.
    let mut zip = ZipWriter::new(Full {
        bytes: Vec::new(),
        limit: 31,
    });
    let error = zip
        .write_deflated(b"a", deflated(b"hello", 1 << 20))
        .unwrap_err();
    assert!(matches!(error, WriteError::Io(_)), "{error:?}");
    assert!(matches!(
        zip.write_deflated(b"b", deflated(b"x", 1 << 20)),
        Err(WriteError::Unusable)
    ));
}

/// Spools that cannot be made.
struct NoSpools;

impl SpoolFactory for NoSpools {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        Err(io::Error::other("no space for a spool"))
    }
}

#[test]
fn a_zip_written_to_a_deflate_writer_reports_its_spool_failures_as_spool_errors() {
    // Content of 200,000 bytes that deflate poorly makes the outer writer
    // produce output, which it cannot spill.
    let data = noise(200_000);

    let mut spools = NoSpools;
    let mut zip = ZipWriter::new(DeflateWriter::new(&mut spools, 0));
    let mut source = open(stored(&[(b"a", &data)]));
    let error = zip
        .copy_entry(&mut source, 0, &mut Diagnostics::new())
        .unwrap_err();
    assert!(matches!(error, WriteError::Spool(_)), "{error:?}");

    let mut spools = NoSpools;
    let mut zip = ZipWriter::new(DeflateWriter::new(&mut spools, 0));
    let error = zip
        .write_deflated(b"a", deflated(&data, 1 << 20))
        .unwrap_err();
    assert!(matches!(error, WriteError::Spool(_)), "{error:?}");
}

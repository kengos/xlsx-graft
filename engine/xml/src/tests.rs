//! Tests for `XmlReader` and `XmlStreamReader`, driven through their public
//! entry points. Each file under `tests/` covers one aspect of what the
//! readers refuse or admit; [`read_with`] runs every input through both, at
//! several chunk sizes, so that what a part gets depends neither on the
//! reader (the in-memory one is the streaming one over a slice) nor on how
//! its bytes arrive. `streaming` covers what only the streaming reader does.

mod doctype;
mod encoding;
mod limits;
mod malformed;
mod reading;
mod streaming;

use crate::events::Event;
use crate::{XmlLimits, XmlReader, XmlStreamError, XmlStreamReader};
use std::io::{self, BufReader, Read};
use xlsx_graft_diagnostics::Diagnostic;

/// A reader over a part's bytes that can fail like a host: it returns
/// `Interrupted` once when it reaches `interrupt_at`, and an error of its
/// own once it reaches `fail_at`.
#[derive(Debug)]
pub(crate) struct Flaky {
    bytes: Vec<u8>,
    pos: usize,
    pub(crate) interrupt_at: Option<usize>,
    pub(crate) fail_at: Option<usize>,
    /// How many reads it answered, errors included.
    pub(crate) reads: usize,
}

impl Read for Flaky {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reads += 1;
        if self.interrupt_at == Some(self.pos) {
            self.interrupt_at = None;
            return Err(io::ErrorKind::Interrupted.into());
        }
        let stop = self.fail_at.unwrap_or(usize::MAX).min(self.bytes.len());
        if self.fail_at == Some(self.pos) {
            return Err(io::Error::other("the host failed"));
        }
        let end = stop.min(self.pos + buf.len());
        buf[..end - self.pos].copy_from_slice(&self.bytes[self.pos..end]);
        let read = end - self.pos;
        self.pos = end;
        Ok(read)
    }
}

/// The one reader type every streamed test reads through, so each generic
/// function is built, and measured for coverage, once.
pub(crate) type Chunked = BufReader<Flaky>;

/// `bytes` handed out at most `step` bytes at a time.
pub(crate) fn chunked(bytes: &[u8], step: usize) -> Chunked {
    let flaky = Flaky {
        bytes: bytes.to_vec(),
        pos: 0,
        interrupt_at: None,
        fail_at: None,
        reads: 0,
    };
    BufReader::with_capacity(step, flaky)
}

/// Reads `bytes` to the end under `limits` with the in-memory reader, and
/// with the streaming reader at every chunk size (the whole part at once, a
/// byte at a time, and three bytes, which splits multi-byte characters and
/// UTF-16 code units both ways), asserting they all agree on the offset of
/// every event before `Eof` and on the first diagnostic: how many events
/// came before `Eof`, or that diagnostic.
fn read_with(bytes: &[u8], limits: &XmlLimits) -> Result<usize, Diagnostic> {
    let memory = memory_offsets(bytes, limits);
    for step in [bytes.len().max(1), 1, 3] {
        let streamed = stream_offsets(chunked(bytes, step), limits);
        assert_eq!(
            streamed, memory,
            "the streaming reader, {step} bytes at a time"
        );
    }
    memory.map(|offsets| offsets.len())
}

/// [`read_with`] under the default limits.
fn read(bytes: &[u8]) -> Result<usize, Diagnostic> {
    read_with(bytes, &XmlLimits::default())
}

/// [`read_with`] with the in-memory reader alone.
fn read_memory_with(bytes: &[u8], limits: &XmlLimits) -> Result<usize, Diagnostic> {
    memory_offsets(bytes, limits).map(|offsets| offsets.len())
}

/// [`read_with`] with the streaming reader alone, over `source`.
fn read_stream_with(source: Chunked, limits: &XmlLimits) -> Result<usize, Diagnostic> {
    stream_offsets(source, limits).map(|offsets| offsets.len())
}

/// Where each event before `Eof` starts as the in-memory reader reads
/// `bytes`, or the first diagnostic.
fn memory_offsets(bytes: &[u8], limits: &XmlLimits) -> Result<Vec<u64>, Diagnostic> {
    let mut reader = XmlReader::open(bytes, limits)?;
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    while !matches!(reader.read_event_into(&mut buf)?, Event::Eof) {
        offsets.push(reader.offset());
    }
    Ok(offsets)
}

/// [`memory_offsets`] for the streaming reader over `source`; an I/O error
/// fails the test.
fn stream_offsets(source: Chunked, limits: &XmlLimits) -> Result<Vec<u64>, Diagnostic> {
    let mut reader = XmlStreamReader::new(source, limits);
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => return Ok(offsets),
            Ok(_) => offsets.push(reader.offset()),
            Err(XmlStreamError::Invalid(diagnostic)) => return Err(diagnostic),
            Err(XmlStreamError::Io(error)) => panic!("unexpected I/O error: {error}"),
        }
    }
}

/// The offset and detail of the `xml_malformed` that reading `bytes` ends in.
fn malformed_at(bytes: &[u8]) -> (u64, String) {
    match read(bytes) {
        Err(Diagnostic::XmlMalformed { offset, detail }) => (offset, detail.as_str().to_owned()),
        other => panic!("expected xml_malformed, got {other:?}"),
    }
}

/// `text` in UTF-16 with a byte order mark, little-endian unless `big`.
fn utf16(text: &str, big: bool) -> Vec<u8> {
    let mut bytes = if big {
        vec![0xfe, 0xff]
    } else {
        vec![0xff, 0xfe]
    };
    for unit in text.encode_utf16() {
        bytes.extend(if big {
            unit.to_be_bytes()
        } else {
            unit.to_le_bytes()
        });
    }
    bytes
}

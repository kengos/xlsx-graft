//! JSON-RPC 2.0 plumbing: newline-delimited framing with a line cap, the
//! response builders, and the bounded echo of a client's strings.

use serde_json::{Value, json};
use std::io::{self, BufRead, Read, Write};

// The JSON-RPC error codes this server answers with (the standard set).
pub(crate) const PARSE_ERROR: i64 = -32700;
pub(crate) const INVALID_REQUEST: i64 = -32600;
pub(crate) const METHOD_NOT_FOUND: i64 = -32601;
pub(crate) const INVALID_PARAMS: i64 = -32602;

/// Longest accepted request line, in bytes, without its `\n` (a `\r` before
/// it counts). A request names its inputs rather than carrying them, so real
/// ones stay far below; the cap bounds what a hostile client can make the
/// server hold. `serve`'s oversized-line error says it in words.
pub(crate) const MAX_LINE: usize = 1024 * 1024;

/// Longest echo of a client's string (a method or tool name) in an error.
const MAX_ECHO: usize = 200;

/// A request's error, answered as the response's `error` member.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RpcError {
    pub(crate) code: i64,
    pub(crate) message: String,
}

impl RpcError {
    pub(crate) fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// One read from the transport.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Frame {
    /// A complete line, its trailing newline (and `\r`) stripped.
    Line(String),
    /// The line was longer than [`MAX_LINE`]; the rest of it was discarded.
    Oversized,
    /// End of stream.
    Eof,
}

/// Reads one newline-delimited frame, holding at most [`MAX_LINE`] + 1 bytes.
pub(crate) fn read_frame<R: BufRead>(reader: &mut R) -> io::Result<Frame> {
    let mut buf = Vec::new();
    // usize to u64 widens on every target Rust supports.
    let n = (&mut *reader)
        .take(MAX_LINE as u64 + 1)
        .read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(Frame::Eof);
    }
    if buf.last() != Some(&b'\n') && buf.len() > MAX_LINE {
        drain_line(reader)?;
        return Ok(Frame::Oversized);
    }
    while matches!(buf.last(), Some(b'\n' | b'\r')) {
        buf.pop();
    }
    Ok(Frame::Line(String::from_utf8_lossy(&buf).into_owned()))
}

/// Discards the rest of an oversized line in bounded chunks, so the loop
/// picks up again at the next newline.
fn drain_line<R: BufRead>(reader: &mut R) -> io::Result<()> {
    loop {
        let mut sink = Vec::new();
        let n = (&mut *reader)
            .take(MAX_LINE as u64)
            .read_until(b'\n', &mut sink)?;
        if n == 0 || sink.last() == Some(&b'\n') {
            return Ok(());
        }
    }
}

/// Writes one message as a single line and flushes it.
pub(crate) fn write_frame<W: Write>(writer: &mut W, message: &Value) -> io::Result<()> {
    // Serializing a Value escapes every newline inside it, so this is one line.
    writer.write_all(message.to_string().as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

/// A success response.
pub(crate) fn result_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// An error response.
pub(crate) fn error_response(id: Value, error: &RpcError) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": error.code, "message": error.message },
    })
}

/// Bounds the echo of a client's string: control characters dropped, then
/// cut to [`MAX_ECHO`] characters.
pub(crate) fn clip(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(MAX_ECHO)
        .collect()
}

#[cfg(test)]
mod tests;

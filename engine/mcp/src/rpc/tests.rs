//! Unit tests for the framing bounds, the response builders and the echo
//! clip.

use super::{
    Frame, MAX_LINE, PARSE_ERROR, RpcError, clip, error_response, read_frame, result_response,
    write_frame,
};
use serde_json::{Value, json};
use std::io::{self, Cursor, Write};

fn frames(input: Vec<u8>) -> Vec<Frame> {
    let mut reader = Cursor::new(input);
    let mut frames = Vec::new();
    loop {
        let frame = read_frame(&mut reader).unwrap();
        if frame == Frame::Eof {
            return frames;
        }
        frames.push(frame);
    }
}

fn line(s: &str) -> Frame {
    Frame::Line(s.to_owned())
}

#[test]
fn lines_lose_their_newline_and_a_last_line_needs_none() {
    assert_eq!(
        frames(b"one\ntwo\r\ntail".to_vec()),
        [line("one"), line("two"), line("tail")]
    );
}

#[test]
fn a_line_of_exactly_the_cap_is_read_whole() {
    let mut data = vec![b'x'; MAX_LINE];
    data.extend_from_slice(b"\nnext\n");
    let read = frames(data);
    assert_eq!(read.len(), 2);
    assert!(matches!(&read[0], Frame::Line(l) if l.len() == MAX_LINE));
    assert_eq!(read[1], line("next"));
}

#[test]
fn one_byte_past_the_cap_is_oversized_and_the_next_line_survives() {
    let mut data = vec![b'x'; MAX_LINE + 1];
    data.extend_from_slice(b"\nnext\n");
    assert_eq!(frames(data), [Frame::Oversized, line("next")]);
}

#[test]
fn an_oversized_line_is_drained_over_several_chunks() {
    let mut data = vec![b'x'; MAX_LINE * 3];
    data.extend_from_slice(b"\nnext\n");
    assert_eq!(frames(data), [Frame::Oversized, line("next")]);
}

#[test]
fn an_oversized_line_at_the_end_of_stream_drains_cleanly() {
    assert_eq!(frames(vec![b'x'; MAX_LINE + 5]), [Frame::Oversized]);
}

#[test]
fn invalid_utf8_is_replaced_rather_than_refused() {
    assert_eq!(frames(b"a\xffb\n".to_vec()), [line("a\u{fffd}b")]);
}

#[test]
fn a_read_failure_is_an_error() {
    /// Fails every read.
    struct Broken;

    impl io::Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::ConnectionReset.into())
        }
    }

    let mut reader = io::BufReader::new(Broken);
    let err = read_frame(&mut reader).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::ConnectionReset);
}

#[test]
fn the_builders_shape_a_result_and_an_error() {
    assert_eq!(
        result_response(json!(1), json!({ "a": 1 })),
        json!({ "jsonrpc": "2.0", "id": 1, "result": { "a": 1 } })
    );
    assert_eq!(
        error_response(Value::Null, &RpcError::new(PARSE_ERROR, "bad")),
        json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "bad" } })
    );
}

#[test]
fn a_frame_is_one_line_and_a_refused_write_is_an_error() {
    let mut out = Vec::new();
    write_frame(&mut out, &json!({ "x": "a\nb" })).unwrap();
    assert_eq!(out, b"{\"x\":\"a\\nb\"}\n");

    // A zero-length buffer takes no bytes, like a full disk.
    let mut full: &mut [u8] = &mut [];
    let err = write_frame(&mut full, &json!({})).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::WriteZero);
}

#[test]
fn a_frame_that_fails_only_on_flush_is_an_error_too() {
    /// Takes every byte, then fails to flush them.
    struct FlushFails;

    impl Write for FlushFails {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::StorageFull.into())
        }
    }

    let err = write_frame(&mut FlushFails, &json!({})).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::StorageFull);
}

#[test]
fn clip_drops_control_characters_and_bounds_the_length() {
    assert_eq!(clip("a\x1b[31mb\n"), "a[31mb");
    assert_eq!(clip(&"x".repeat(500)), "x".repeat(200));
    assert_eq!(clip(&"あ".repeat(201)), "あ".repeat(200));
}

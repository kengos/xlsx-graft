//! Unit tests for the crate root: how the command line is read, and what
//! `run` writes for it.

use super::{Cli, CliError, Format, run};
use clap::Parser;
use clap::error::ErrorKind;
use std::io::{self, Write};
use xlsx_graft_core::version;

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("xlsx-graft").chain(args.iter().copied())).unwrap()
}

fn run_to_string(args: &[&str]) -> String {
    let mut out = Vec::new();
    run(&parse(args), &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn no_arguments_asks_for_nothing_in_text() {
    let cli = parse(&[]);
    assert!(!cli.version);
    assert_eq!(cli.format, Format::Text);
}

#[test]
fn version_is_read_from_the_long_and_the_short_flag() {
    assert!(parse(&["--version"]).version);
    assert!(parse(&["-V"]).version);
}

#[test]
fn format_is_read_on_either_side_of_another_flag() {
    assert_eq!(
        parse(&["--format", "json", "--version"]).format,
        Format::Json
    );
    assert_eq!(parse(&["--version", "--format=json"]).format, Format::Json);
    assert_eq!(parse(&["--format", "text"]).format, Format::Text);
}

#[test]
fn an_unknown_format_is_a_usage_error() {
    let err = Cli::try_parse_from(["xlsx-graft", "--format", "xml"]).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidValue);
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn version_prints_the_core_version_as_text() {
    assert_eq!(
        run_to_string(&["--version"]),
        format!("xlsx-graft {}\n", version())
    );
}

#[test]
fn version_prints_the_core_version_as_json() {
    assert_eq!(
        run_to_string(&["--version", "--format", "json"]),
        format!("{{\"version\":\"{}\"}}\n", version())
    );
}

#[test]
fn nothing_to_do_prints_the_help() {
    let help = run_to_string(&[]);
    assert!(help.starts_with("Read, verify, create and edit xlsx workbooks"));
    assert!(help.contains("Usage: xlsx-graft"));
    assert!(help.contains("--version"));
    assert!(help.contains("--format <FORMAT>"));
    // The help is not a command's output, so --format leaves it text.
    assert_eq!(run_to_string(&["--format", "json"]), help);
}

#[test]
fn a_full_stdout_is_an_error_rather_than_a_panic() {
    for args in [&["--version"][..], &[]] {
        // A zero-length buffer takes no bytes, like a full disk.
        let mut full: &mut [u8] = &mut [];
        let err = run(&parse(args), &mut full).unwrap_err();
        assert!(matches!(err, CliError::Write(e) if e.kind() == io::ErrorKind::WriteZero));
    }
}

#[test]
fn a_stdout_that_fails_only_on_flush_is_an_error_too() {
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

    let err = run(&parse(&["--version"]), &mut FlushFails).unwrap_err();
    assert!(matches!(err, CliError::Write(e) if e.kind() == io::ErrorKind::StorageFull));
}

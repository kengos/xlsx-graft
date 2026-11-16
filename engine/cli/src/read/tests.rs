//! Unit tests for what `read --format text` prints at the end of a run,
//! without a workbook. What it prints of each value is the host's `value`
//! module; what it prints of a run is `tests/bin/read`.

use super::text::Text;
use super::{End, Sink};
use crate::CliError;
use crate::workbook::Refused;
use std::io::{self, Write};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_read::Refusal;

#[test]
fn text_flushes_the_rows_before_the_problems_and_passes_a_closed_stderr_over() {
    let mut diagnostics = Diagnostics::new();
    let missing = Diagnostic::SharedStringMissing { index: 7 };
    diagnostics.push(missing, Location::default());
    let end = End {
        error: Some(Refused(Refusal::WorkbookUnreadable)),
        diagnostics: diagnostics.finish(),
        ..End::default()
    };
    let mut out = io::BufWriter::new(Vec::new());
    out.write_all(b"1\n").unwrap();
    // A stderr that takes nothing, as a closed one or a full disk.
    let mut refusing: &mut [u8] = &mut [];
    Text::new(&mut out, &mut refusing).end(&end).unwrap();
    assert_eq!(out.buffer(), b"");
    assert_eq!(out.get_ref(), b"1\n");
    // A stdout that refuses the rows is still an error.
    let mut full = io::BufWriter::new(&mut [][..]);
    full.write_all(b"1\n").unwrap();
    let err = Text::new(&mut full, &mut io::sink()).end(&end).unwrap_err();
    assert!(matches!(err, CliError::Write(e) if e.kind() == io::ErrorKind::WriteZero));
}

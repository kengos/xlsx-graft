//! A stdout that cannot take the output: exit 1 and a reason on stderr,
//! never a panic.

// /dev/full refuses every write with ENOSPC. The gates run in the Linux
// toolchain image; macOS has no such device.
#[cfg(target_os = "linux")]
#[test]
fn a_full_stdout_exits_1_with_the_reason_on_stderr() {
    use crate::{stderr, xlsx_graft};
    use std::fs::OpenOptions;

    let full = OpenOptions::new().write(true).open("/dev/full").unwrap();
    let output = xlsx_graft()
        .args(["--version"])
        .stdout(full)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).starts_with("xlsx-graft: failed to write the output: "));
}

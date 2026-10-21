//! A stdout that cannot take a response: exit 1 and a reason on stderr,
//! never a panic.

// /dev/full refuses every write with ENOSPC. The gates run in the Linux
// toolchain image; macOS has no such device.
#[cfg(target_os = "linux")]
#[test]
fn a_full_stdout_exits_1_with_the_reason_on_stderr() {
    use crate::{stderr, talk_to, xlsx_graft_mcp};
    use std::fs::OpenOptions;

    let full = OpenOptions::new().write(true).open("/dev/full").unwrap();
    let mut command = xlsx_graft_mcp();
    command.stdout(full);
    let output = talk_to(command, &[r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).starts_with("xlsx-graft-mcp: stdio transport failed: "));
}

//! The command line: a policy that is not valid, a file that cannot be read
//! and an argument that is not accepted each stop the server before it reads
//! a request, with the exit code the CLI uses (2 for a usage or policy
//! fault, 1 for a file that cannot be read) and the reason on stderr. A valid
//! policy file is the policy every `verify` call runs under.

use crate::{responses, stderr, talk_to, xlsx_graft_mcp};
use serde_json::json;

/// The `table.xlsx` fixture, which a default policy accepts.
const TABLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/table.xlsx");

/// A policy file with `text` in a directory of its own.
fn policy_file(text: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("policy.json");
    std::fs::write(&path, text).unwrap();
    let path = path.to_str().unwrap().to_owned();
    (dir, path)
}

#[test]
fn a_policy_that_is_not_valid_exits_2_and_lists_every_fault_on_stderr() {
    let (_dir, path) = policy_file(r#"{"maxEntries": 0, "nope": 1}"#);
    let mut command = xlsx_graft_mcp();
    command.args(["--policy", &path]);
    let output = talk_to(command, &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    let said = stderr(&output);
    assert!(said.starts_with(&format!(
        "xlsx-graft-mcp: the policy {path} is not valid:\nerror\n"
    )));
    assert_eq!(said.matches("policy_invalid").count(), 2);
}

#[test]
fn an_argument_that_is_not_accepted_exits_2_with_the_usage() {
    let mut command = xlsx_graft_mcp();
    command.arg("--policy=a.json");
    let output = talk_to(command, &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output),
        "xlsx-graft-mcp: unexpected argument: --policy=a.json\n\
         usage: xlsx-graft-mcp [--policy FILE]\n"
    );
}

#[test]
fn a_policy_file_that_cannot_be_read_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("none.json");
    let mut command = xlsx_graft_mcp();
    command.arg("--policy").arg(&path);
    let output = talk_to(command, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).starts_with(&format!(
        "xlsx-graft-mcp: the policy {} could not be read: ",
        path.display()
    )));
}

#[test]
fn a_valid_policy_is_the_policy_every_verify_call_runs_under() {
    let (_dir, path) = policy_file(r#"{"maxEntries": 1}"#);
    let call = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "verify", "arguments": { "path": TABLE } },
    })
    .to_string();
    let mut command = xlsx_graft_mcp();
    command.args(["--policy", &path]);
    let output = talk_to(command, &[&call]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let result = &responses(&output)[0]["result"];
    assert_eq!(result["isError"], false);
    assert_eq!(result["structuredContent"]["ok"], false);
    let limit = &result["structuredContent"]["diagnostics"][0]["decidedBy"];
    assert_eq!(limit["source"], "policy");
}

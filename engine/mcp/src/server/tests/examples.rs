//! The example calls in the reference pages, sent through `serve`: each
//! ```` ```json ```` block with a `method` is a request, and a block
//! without one is the expected `result` of the request block just before
//! it. A request followed by a result block fails this test when the
//! `result` of the response differs from the result block. A request with
//! no result block fails it when the response is a JSON-RPC error, or, for `tools/call`, a tool
//! result with `isError: true`. Each row of the tool table also needs a
//! `tools/call` example for its tool in the page the row names.
//!
//! The requests run with the `engine/mcp` directory as the working
//! directory. Before a request is sent, the `fixtures/` at the start of its
//! `params.arguments.path` is replaced with the absolute path of the
//! repository's `fixtures/` directory ([`FIXTURES_DIR`]). Before a
//! response is compared with its result block, the `origin` of each
//! diagnostic in its `structuredContent` is removed: it is the engine's
//! source line, outside the contract, so a result block is written without
//! it.

use super::talk;
use serde_json::Value;
use std::collections::BTreeSet;
use xlsx_graft_reference::pages;

/// The engine version the result blocks of `pages/version.md` are written
/// with. The test replaces every occurrence of it in those result blocks,
/// and only in those, with `xlsx_graft_core::version()` before comparing,
/// so a release does not need the page rewritten. A result block in any
/// other page is compared as written.
const VERSION_IN_PAGES: &str = "0.0.0";

/// What the pages write for the repository's `fixtures/` directory, such as
/// `fixtures/table.xlsx`. The requests run with `engine/mcp` as the working
/// directory, so the test replaces it at the start of a request's
/// `params.arguments.path` with [`FIXTURES_DIR`] before sending the request.
const FIXTURES_IN_PAGES: &str = "fixtures/";

/// The absolute path of the repository's `fixtures/` directory, with the
/// trailing `/`, that replaces [`FIXTURES_IN_PAGES`] in a request.
const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/");

/// Replaces `fixtures/` at the start of `request`'s `params.arguments.path`,
/// when the request has such a path, with [`FIXTURES_DIR`].
fn with_fixtures_dir(request: &mut Value) {
    let Some(path) = request.pointer_mut("/params/arguments/path") else {
        return;
    };
    if let Some(rest) = path
        .as_str()
        .and_then(|p| p.strip_prefix(FIXTURES_IN_PAGES))
    {
        *path = Value::String(format!("{FIXTURES_DIR}{rest}"));
    }
}

/// Removes the `origin` of each diagnostic in the response's
/// `result.structuredContent`, when it has any.
fn without_origins(response: &mut Value) {
    let diagnostics = response.pointer_mut("/result/structuredContent/diagnostics");
    for diagnostic in diagnostics
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        if let Some(diagnostic) = diagnostic.as_object_mut() {
            diagnostic.remove("origin");
        }
    }
}

/// The text of every ```` ```json ```` block in `markdown`, in order. A
/// fence indented inside a list item counts too. An opening fence whose
/// language name starts with `json` in any case and is not exactly
/// ```` ```json ```` (```` ```JSON ````, ```` ``` json ````, ```` ```jsonc ````,
/// `~~~json`) fails the test, so that no example is left unchecked by a
/// misspelled fence.
fn json_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut open: Option<Vec<&str>> = None;
    for line in markdown.lines() {
        match open.as_mut() {
            None if line.trim() == "```json" => open = Some(Vec::new()),
            None => assert!(
                !is_misspelled_json_fence(line.trim()),
                "write a JSON example's opening fence as ```json: {line}"
            ),
            Some(block) if line.trim() == "```" => {
                blocks.push(block.join("\n"));
                open = None;
            }
            Some(block) => block.push(line),
        }
    }
    assert!(open.is_none(), "a ```json block is not closed");
    blocks
}

/// Whether `fence`, a trimmed line, opens a fenced block with ```` ``` ````
/// or `~~~` and a language name that starts with `json` in any case.
/// `json_blocks` calls it only for a line other than ```` ```json ````.
fn is_misspelled_json_fence(fence: &str) -> bool {
    let info = fence.trim_start_matches('`').trim_start_matches('~');
    info.len() < fence.len() && info.trim().to_ascii_lowercase().starts_with("json")
}

#[test]
fn a_misspelled_json_fence_is_told_apart_from_other_lines() {
    for fence in ["```JSON", "``` json", "```jsonc", "~~~json"] {
        assert!(is_misspelled_json_fence(fence), "{fence}");
    }
    for line in ["```", "```text", "json", "{\"json\":1}"] {
        assert!(!is_misspelled_json_fence(line), "{line}");
    }
}

/// Whether `block` is a result block: JSON with no `method` key. A block
/// that is not JSON is not a result block, so the test reads it as the
/// next request, and the `expect` that parses each request panics with
/// "every ```json block in a page is JSON".
fn is_result_block(block: &str) -> bool {
    serde_json::from_str::<Value>(block).is_ok_and(|value| value.get("method").is_none())
}

#[test]
fn every_example_call_answers_as_its_result_block_or_without_an_error_and_every_tool_has_one() {
    let mut examples = BTreeSet::new();
    for page in pages() {
        let mut blocks = json_blocks(page.body()).into_iter().peekable();
        while let Some(block) = blocks.next() {
            let example: Value =
                serde_json::from_str(&block).expect("every ```json block in a page is JSON");
            assert!(
                example.get("method").is_some(),
                "{}: a result block with no example call before it: {block}",
                page.uri()
            );
            if example["method"] == "tools/call" {
                let tool = example["params"]["name"].as_str().unwrap_or_default();
                examples.insert((page.name().to_string(), tool.to_string()));
            }
            // One line: the transport reads one request per line.
            let mut sent = example.clone();
            with_fixtures_dir(&mut sent);
            let mut responses = talk(&sent.to_string());
            assert_eq!(responses.len(), 1, "{}: {block}", page.uri());
            let response = &mut responses[0];
            without_origins(response);
            let result_block = blocks.next_if(|next| is_result_block(next));
            if let Some(result_block) = result_block {
                let result_block = if page.name() == "version" {
                    result_block.replace(VERSION_IN_PAGES, xlsx_graft_core::version())
                } else {
                    result_block
                };
                let expected: Value =
                    serde_json::from_str(&result_block).expect("a result block is JSON");
                assert_eq!(response["result"], expected, "{}: {block}", page.uri());
            } else {
                assert!(
                    response.get("error").is_none(),
                    "{}: {response}",
                    page.uri()
                );
                if example["method"] == "tools/call" {
                    assert_eq!(
                        response["result"]["isError"],
                        false,
                        "{}: {response}",
                        page.uri()
                    );
                }
            }
        }
    }
    for tool in crate::tools::TOOLS {
        assert!(
            examples.contains(&(tool.page.to_string(), tool.name.to_string())),
            "pages/{}.md has no tools/call example for {}",
            tool.page,
            tool.name
        );
    }
}

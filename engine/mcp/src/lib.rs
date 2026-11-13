//! xlsx-graft MCP server: the engine as MCP tools over stdio.
//!
//! A thin host over [`xlsx_graft_core`] and [`xlsx_graft_host`], like the C
//! ABI and the CLI. `main.rs` makes the [`Context`] (the default policy) and
//! hands it, with stdin and stdout, to [`serve`]; everything testable lives
//! here. Each tool is one row of the tool table in `tools.rs`, and each
//! tool's description in `tools/list` ends with the URI of the reference
//! page (from `xlsx-graft-reference`) that documents it.
//!
//! The transport is newline-delimited JSON-RPC 2.0, written by hand rather
//! than with an SDK: a blocking loop that reads one request line and answers
//! it needs no async runtime. A malformed or unknown request is answered with
//! a JSON-RPC error and the loop goes on; only a failure of stdin or stdout
//! itself ends it, as an [`McpError`].
//!
//! A tool's `structuredContent` is a contract: its keys and their types do
//! not change once released, and the `outputSchema` of each tool that
//! answers with `structuredContent` states them. A tool whose result has no
//! `structuredContent` declares no `outputSchema`.

mod context;
mod error;
mod rpc;
mod server;
mod tools;

pub use context::Context;
pub use error::McpError;
pub use server::serve;

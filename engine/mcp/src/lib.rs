//! xlsx-graft MCP server: the engine as MCP tools over stdio.
//!
//! A thin host over [`xlsx_graft_core`], like the C ABI and the CLI.
//! `main.rs` hands stdin and stdout to [`serve`]; everything testable lives
//! here. For now the one tool is `version`.
//!
//! The transport is newline-delimited JSON-RPC 2.0, written by hand rather
//! than with an SDK: a blocking loop that reads one request line and answers
//! it needs no async runtime. A malformed or unknown request is answered with
//! a JSON-RPC error and the loop goes on; only a failure of stdin or stdout
//! itself ends it, as an [`McpError`].
//!
//! A tool's `structuredContent` is a contract: its keys and their types do
//! not change once released, and each tool's `outputSchema` states them.

mod error;
mod rpc;
mod server;
mod tools;

pub use error::McpError;
pub use server::serve;

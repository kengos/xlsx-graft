//! `McpError`: why the server stopped. `main.rs` prints it to stderr and
//! exits 1.

use std::io;
use thiserror::Error;

/// Why the server stopped before the end of stdin. A bad request never
/// reaches here: it is answered with a JSON-RPC error and the loop goes on.
#[derive(Debug, Error)]
pub enum McpError {
    /// stdin could not be read, or stdout refused a response (a closed pipe,
    /// a full disk).
    #[error("stdio transport failed: {0}")]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests;

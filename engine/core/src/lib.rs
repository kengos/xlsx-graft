//! xlsx-graft core: what every other engine crate builds on.
//!
//! For now it only reports the engine version, which is what the hosts
//! (C ABI, CLI, MCP) expose first.

/// The engine version, taken from the workspace manifest.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

//! Unit tests for the crate root: the reported version.

use super::version;

#[test]
fn version_matches_the_workspace_manifest() {
    assert_eq!(version(), "0.0.0");
}

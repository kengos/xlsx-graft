//! Unit tests for the panic guard.

use super::guard;
use crate::status::XLSX_GRAFT_ERR_PANIC;

#[test]
fn a_body_that_returns_passes_its_status_through() {
    assert_eq!(guard(|| 7), 7);
}

#[test]
fn a_body_that_panics_becomes_the_panic_status() {
    assert_eq!(guard(|| panic!("boom")), XLSX_GRAFT_ERR_PANIC);
}

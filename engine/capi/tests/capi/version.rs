//! `xlsx_graft_version`: what a binding reads back.

use std::ptr;
use xlsx_graft_capi::{XLSX_GRAFT_OK, xlsx_graft_version};

#[test]
fn the_version_comes_back_as_a_pointer_and_a_length() {
    let mut out: *const u8 = ptr::null();
    let mut len = 0;
    // SAFETY: both arguments are writable slots of the declared types.
    let status = unsafe { xlsx_graft_version(&raw mut out, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the library points `out` at `len` static bytes.
    let bytes = unsafe { std::slice::from_raw_parts(out, len) };
    assert_eq!(std::str::from_utf8(bytes).unwrap(), "0.0.0");
}

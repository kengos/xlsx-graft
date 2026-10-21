//! Unit tests for the exported entry point: what it writes, and the
//! null-argument refusals.

use super::{XLSX_GRAFT_ERR_NULL_ARG, XLSX_GRAFT_OK, xlsx_graft_version};
use std::ptr;

#[test]
fn the_core_version_is_written_as_a_pointer_and_a_length() {
    let mut out: *const u8 = ptr::null();
    let mut len = 0;
    // SAFETY: both arguments are writable slots of the declared types.
    let status = unsafe { xlsx_graft_version(&raw mut out, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    let version = xlsx_graft_core::version();
    assert_eq!((out, len), (version.as_ptr(), version.len()));
}

#[test]
fn a_null_out_ptr_is_refused_and_the_length_is_left_alone() {
    let mut len = usize::MAX;
    // SAFETY: `len` is a writable slot; the null `out_ptr` is what is tested.
    let status = unsafe { xlsx_graft_version(ptr::null_mut(), &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_ERR_NULL_ARG);
    assert_eq!(len, usize::MAX);
}

#[test]
fn a_null_out_len_is_refused_and_the_pointer_is_left_alone() {
    let mut out: *const u8 = ptr::dangling();
    // SAFETY: `out` is a writable slot; the null `out_len` is what is tested.
    let status = unsafe { xlsx_graft_version(&raw mut out, ptr::null_mut()) };
    assert_eq!(status, XLSX_GRAFT_ERR_NULL_ARG);
    assert_eq!(out, ptr::dangling());
}

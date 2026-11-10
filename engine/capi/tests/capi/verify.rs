//! The verify handle, driven end to end as a binding drives it: a workbook
//! from a path and from bytes, a policy alone, and many verifications at
//! once, through the library as its dependents link it.

use std::ptr;
use xlsx_graft_capi::*;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

/// Copies out the result of `verify` and closes it, as a binding does.
fn finish(status: i32, verify: *mut XlsxGraftVerify) -> String {
    assert_eq!(status, XLSX_GRAFT_OK);
    let (mut ptr, mut len) = (ptr::null(), 0);
    // SAFETY: `verify` is a live handle; the out pointers are writable slots.
    let status = unsafe { xlsx_graft_verify_result(verify, &raw mut ptr, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the library lent `len` UTF-8 bytes, alive until the close.
    let text = String::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec());
    // SAFETY: `verify` is a live handle, closed once.
    assert_eq!(unsafe { xlsx_graft_verify_close(verify) }, XLSX_GRAFT_OK);
    text.unwrap()
}

/// The result of verifying the workbook at `path` under `options`.
fn verify_path(path: &str, options: &str) -> String {
    let mut verify = ptr::null_mut();
    // SAFETY: both strings are alive for the call, `verify` a writable slot.
    let status = unsafe {
        xlsx_graft_verify_path(
            path.as_ptr(),
            path.len(),
            options.as_ptr(),
            options.len(),
            &raw mut verify,
        )
    };
    finish(status, verify)
}

/// The result of verifying the workbook in `data` under `options`.
fn verify_bytes(data: &[u8], options: &str) -> String {
    let mut verify = ptr::null_mut();
    // SAFETY: the bytes are alive for the call, `verify` a writable slot.
    let status = unsafe {
        xlsx_graft_verify_bytes(
            data.as_ptr(),
            data.len(),
            options.as_ptr(),
            options.len(),
            &raw mut verify,
        )
    };
    finish(status, verify)
}

/// The result of checking `policy` alone.
fn policy_check(policy: &str) -> String {
    let mut verify = ptr::null_mut();
    // SAFETY: the policy is alive for the call, `verify` a writable slot.
    let status = unsafe { xlsx_graft_policy_check(policy.as_ptr(), policy.len(), &raw mut verify) };
    finish(status, verify)
}

#[test]
fn a_workbook_is_verified_from_its_path_and_from_its_bytes() {
    let path = format!("{FIXTURES}/macro.xlsm");
    let by_path = verify_path(&path, "");
    assert!(by_path.starts_with(r#"{"ok":false,"failOn":"error","counts":{"error":"#));
    assert!(by_path.contains(r#""code":"part_denied""#), "{by_path}");
    let data = std::fs::read(&path).unwrap();
    assert_eq!(verify_bytes(&data, "{}"), by_path);
    let table = std::fs::read(format!("{FIXTURES}/table.xlsx")).unwrap();
    let ok = verify_bytes(&table, r#"{"rules":{"failOn":"warn"}}"#);
    assert!(ok.starts_with(r#"{"ok":true,"failOn":"warn""#), "{ok}");
}

#[test]
fn a_policy_is_checked_alone() {
    assert!(policy_check("{}").starts_with(r#"{"ok":true"#));
    let bad = policy_check(r#"{"max_entries":1}"#);
    assert!(bad.contains(r#""code":"policy_invalid""#), "{bad}");
}

#[test]
fn verifications_run_at_once_and_a_result_is_read_from_many_threads() {
    let table = std::fs::read(format!("{FIXTURES}/table.xlsx")).unwrap();
    let expected = verify_bytes(&table, "{}");
    let mut verify = ptr::null_mut();
    let options = "{}";
    // SAFETY: the bytes are alive for the call, `verify` a writable slot.
    let status = unsafe {
        xlsx_graft_verify_bytes(
            table.as_ptr(),
            table.len(),
            options.as_ptr(),
            options.len(),
            &raw mut verify,
        )
    };
    assert_eq!(status, XLSX_GRAFT_OK);
    let shared = verify as usize;
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                assert_eq!(verify_bytes(&table, "{}"), expected);
                let (mut ptr, mut len) = (ptr::null(), 0);
                // SAFETY: the handle is live until the close below, and does
                // not change; the out pointers are writable slots.
                let status = unsafe {
                    xlsx_graft_verify_result(
                        shared as *const XlsxGraftVerify,
                        &raw mut ptr,
                        &raw mut len,
                    )
                };
                assert_eq!(status, XLSX_GRAFT_OK);
                // SAFETY: on OK the library lent `len` bytes, alive until the close.
                let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
                assert_eq!(bytes, expected.as_bytes());
            });
        }
    });
    // SAFETY: every thread has finished; the handle is closed once.
    assert_eq!(unsafe { xlsx_graft_verify_close(verify) }, XLSX_GRAFT_OK);
}

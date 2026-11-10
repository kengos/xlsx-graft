//! Verifying a workbook through the C ABI: each call makes a verify handle,
//! copies its result out, and closes it before it returns, so no verify
//! handle outlives the call (docs/sdk.md § Verifying through the C ABI).

use super::{Outcome, read};
use xlsx_graft_capi::{
    XLSX_GRAFT_OK, XlsxGraftVerify, xlsx_graft_policy_check, xlsx_graft_verify_bytes,
    xlsx_graft_verify_close, xlsx_graft_verify_path, xlsx_graft_verify_result,
};

/// Verifies the workbook at `path` under the request `options`
/// (`{"policy": ..., "rules": ...}` as JSON); the result's JSON is the
/// value.
#[must_use]
pub fn verify_path(path: &str, options: &str) -> Outcome {
    let mut handle = std::ptr::null_mut();
    // SAFETY: both strings are alive for the call, `handle` a writable slot.
    let status = unsafe {
        xlsx_graft_verify_path(
            path.as_ptr(),
            path.len(),
            options.as_ptr(),
            options.len(),
            &raw mut handle,
        )
    };
    // SAFETY: `handle` is what the call wrote, when its status is OK.
    unsafe { finish(status, handle) }
}

/// Verifies the workbook in `data`, which the C ABI reads in place during
/// the call.
#[must_use]
pub fn verify_bytes(data: &[u8], options: &str) -> Outcome {
    let mut handle = std::ptr::null_mut();
    // SAFETY: `data` and `options` are alive and unchanged for the call,
    // `handle` a writable slot.
    let status = unsafe {
        xlsx_graft_verify_bytes(
            data.as_ptr(),
            data.len(),
            options.as_ptr(),
            options.len(),
            &raw mut handle,
        )
    };
    // SAFETY: `handle` is what the call wrote, when its status is OK.
    unsafe { finish(status, handle) }
}

/// Checks the policy `policy` (its JSON object) alone; the result's JSON is
/// the value.
#[must_use]
pub fn check_policy(policy: &str) -> Outcome {
    let mut handle = std::ptr::null_mut();
    // SAFETY: `policy` is alive for the call, `handle` a writable slot.
    let status = unsafe { xlsx_graft_policy_check(policy.as_ptr(), policy.len(), &raw mut handle) };
    // SAFETY: `handle` is what the call wrote, when its status is OK.
    unsafe { finish(status, handle) }
}

/// The outcome of a call that made `handle`: its result copied out, then
/// the handle closed. On any status but `XLSX_GRAFT_OK` no handle was
/// written, so none is read or closed.
///
/// # Safety
///
/// When `status` is `XLSX_GRAFT_OK`, `handle` must be the live verify
/// handle the call wrote.
unsafe fn finish(status: i32, handle: *mut XlsxGraftVerify) -> Outcome {
    if status != XLSX_GRAFT_OK {
        // SAFETY: on a status other than OK neither pointer is read.
        return unsafe { read(status, std::ptr::null(), 0) };
    }
    let (mut ptr, mut len) = (std::ptr::null(), 0);
    // SAFETY: the handle is live; the out pointers are writable slots.
    let lent = unsafe { xlsx_graft_verify_result(handle, &raw mut ptr, &raw mut len) };
    // SAFETY: on OK the library lent `len` UTF-8 bytes, alive until the
    // close, which comes after the copy.
    let outcome = unsafe { read(lent, ptr, len) };
    // SAFETY: the handle is live and closed once. A close cannot fail but
    // by a panic, after which the handle is closed all the same.
    unsafe { xlsx_graft_verify_close(handle) };
    outcome
}

#[cfg(test)]
mod tests;

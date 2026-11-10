//! The verify handle's entry points: verify a workbook from a path or
//! bytes, check a policy alone, lend the result, close it.
//!
//! A verify handle does not change once made, and a verification shares
//! nothing with another. So these entry points do not mark a handle as
//! unusable after a panic, as the book's do, and they do not require the
//! caller to use a handle from one thread at a time.

use super::{bytes, missing, refuse_null, run, text};
use crate::verify::XlsxGraftVerify;

/// Writes the handle `verify` into `*out`.
///
/// # Safety
///
/// `out` must be non-null and a writable, aligned slot.
unsafe fn lend(out: *mut *mut XlsxGraftVerify, verify: XlsxGraftVerify) {
    // SAFETY: `out` is non-null, aligned and writable (this function's
    // `# Safety`).
    unsafe { out.write(Box::into_raw(Box::new(verify))) };
}

/// Verifies the workbook at the UTF-8 path `path` (`path_len` bytes, not
/// NUL-terminated) under `options` (`options_len` bytes of UTF-8 JSON,
/// `{"policy": {...}, "rules": {...}}`, each key optional) and writes a
/// verify handle holding the result's JSON into `*out_verify`.
///
/// Returns [`XLSX_GRAFT_OK`](crate::XLSX_GRAFT_OK) whenever the engine
/// tried: a file that cannot be opened or read is the result's `error`,
/// and a fault of the policy or the rules is its diagnostics.
/// [`XLSX_GRAFT_ERR_INVALID_ARG`](crate::XLSX_GRAFT_ERR_INVALID_ARG) when
/// the path is not UTF-8 or `options` is not such an object,
/// [`XLSX_GRAFT_ERR_NULL_ARG`](crate::XLSX_GRAFT_ERR_NULL_ARG) when a
/// pointer that must not be null is; neither writes `*out_verify`.
///
/// # Safety
///
/// `path` and `options` must each be null or point at that many readable
/// bytes (a null one is no bytes when its length is 0, and refused
/// otherwise), and `out_verify` be null or point at one writable, aligned
/// slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_verify_path(
    path: *const u8,
    path_len: usize,
    options: *const u8,
    options_len: usize,
    out_verify: *mut *mut XlsxGraftVerify,
) -> i32 {
    run(|| {
        refuse_null(
            missing(path, path_len) || missing(options, options_len) || out_verify.is_null(),
        )?;
        // SAFETY: neither is missing, and the caller's contract gives each
        // its bytes for the call.
        let (path, options) = unsafe { (text(path, path_len)?, bytes(options, options_len)) };
        let verify = XlsxGraftVerify::path(path, options)?;
        // SAFETY: `out_verify` is non-null, and a writable slot by contract.
        unsafe { lend(out_verify, verify) };
        Ok(())
    })
}

/// Verifies the workbook in the `len` bytes at `data` under `options`, as
/// [`xlsx_graft_verify_path`] does a file.
///
/// The bytes are not copied: they are read during the call only, and may be
/// freed once it returns.
///
/// # Safety
///
/// `data` and `options` must each be null or point at that many readable
/// bytes (a null one is no bytes when its length is 0, and refused
/// otherwise) that stay alive and unchanged until the call returns, and
/// `out_verify` be null or point at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_verify_bytes(
    data: *const u8,
    len: usize,
    options: *const u8,
    options_len: usize,
    out_verify: *mut *mut XlsxGraftVerify,
) -> i32 {
    run(|| {
        refuse_null(missing(data, len) || missing(options, options_len) || out_verify.is_null())?;
        // SAFETY: neither is missing, and the caller's contract gives each
        // its bytes for the call.
        let (data, options) = unsafe { (bytes(data, len), bytes(options, options_len)) };
        let verify = XlsxGraftVerify::bytes(data, options)?;
        // SAFETY: `out_verify` is non-null, and a writable slot by contract.
        unsafe { lend(out_verify, verify) };
        Ok(())
    })
}

/// Checks the policy in the `len` bytes at `policy` (the policy's JSON
/// object itself, not wrapped in a request) and writes a verify handle
/// holding the result's JSON into `*out_verify`: `ok`, or the policy's
/// `policy_invalid` diagnostics.
///
/// Returns [`XLSX_GRAFT_OK`](crate::XLSX_GRAFT_OK) for any bytes, UTF-8 and
/// JSON or not: a fault of the policy is a diagnostic.
/// [`XLSX_GRAFT_ERR_NULL_ARG`](crate::XLSX_GRAFT_ERR_NULL_ARG) when a
/// pointer that must not be null is, writing nothing.
///
/// # Safety
///
/// `policy` must be null or point at `len` readable bytes (a null `policy`
/// is no bytes when `len` is 0, and refused otherwise), and `out_verify` be
/// null or point at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_policy_check(
    policy: *const u8,
    len: usize,
    out_verify: *mut *mut XlsxGraftVerify,
) -> i32 {
    run(|| {
        refuse_null(missing(policy, len) || out_verify.is_null())?;
        // SAFETY: `policy` is not missing, and the caller's contract gives
        // it `len` bytes for the call.
        let verify = XlsxGraftVerify::policy(unsafe { bytes(policy, len) })?;
        // SAFETY: `out_verify` is non-null, and a writable slot by contract.
        unsafe { lend(out_verify, verify) };
        Ok(())
    })
}

/// Writes the result as UTF-8 JSON: a pointer into `*out_ptr` and the
/// length into `*out_len`. The bytes are not NUL-terminated, belong to the
/// handle, and live until its close; every call returns the same bytes.
///
/// # Safety
///
/// `verify` must be null or a verify handle not yet closed, and each out
/// pointer null or at one writable, aligned slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_verify_result(
    verify: *const XlsxGraftVerify,
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    run(|| {
        refuse_null(verify.is_null() || out_ptr.is_null() || out_len.is_null())?;
        // SAFETY: `verify` is non-null, and a live handle by contract.
        let json = unsafe { &*verify }.json();
        // SAFETY: the out pointers are non-null, and writable by contract.
        unsafe {
            out_ptr.write(json.as_ptr());
            out_len.write(json.len());
        }
        Ok(())
    })
}

/// Closes the verify handle; what it lent is gone. A null `verify` does
/// nothing; either way the call returns [`XLSX_GRAFT_OK`](crate::XLSX_GRAFT_OK)
/// (or [`XLSX_GRAFT_ERR_PANIC`](crate::XLSX_GRAFT_ERR_PANIC)).
///
/// # Safety
///
/// `verify` must be null or a verify handle not yet closed. It is closed
/// after the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xlsx_graft_verify_close(verify: *mut XlsxGraftVerify) -> i32 {
    run(|| {
        if !verify.is_null() {
            // SAFETY: a live handle is a `Box` the library leaked, and the
            // caller gives it back only once.
            drop(unsafe { Box::from_raw(verify) });
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests;

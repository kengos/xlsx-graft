//! Unit tests for what the entry points share: the guard around a body,
//! the poisoning of a book after a panic, the UTF-8 check, and
//! `xlsx_graft_error_message`.

use super::{run, run_on, text, xlsx_graft_error_message};
use crate::message::failed;
use crate::shared::Shared;
use crate::status::*;
use std::cell::Cell;
use std::ptr;
use std::rc::Rc;
use xlsx_graft_diagnostics::Diagnostics;

#[test]
fn a_body_is_its_status_or_ok() {
    assert_eq!(run(|| Ok(())), XLSX_GRAFT_OK);
    assert_eq!(
        run(|| Err(XLSX_GRAFT_ERR_SHEET_NOT_FOUND)),
        XLSX_GRAFT_ERR_SHEET_NOT_FOUND
    );
    assert_eq!(run(|| panic!("boom")), XLSX_GRAFT_ERR_PANIC);
}

#[test]
fn a_panic_on_a_handle_refuses_every_later_call_on_its_book() {
    let shared = Rc::new(Shared::new(Diagnostics::new()));
    let ran = Cell::new(0);
    let call = |body: fn() -> Result<(), i32>| {
        run_on(Some(Rc::clone(&shared)), || {
            ran.set(ran.get() + 1);
            body()
        })
    };
    assert_eq!(call(|| Ok(())), XLSX_GRAFT_OK);
    assert_eq!(call(|| panic!("boom")), XLSX_GRAFT_ERR_PANIC);
    assert_eq!(call(|| Ok(())), XLSX_GRAFT_ERR_PANIC);
    assert_eq!(ran.get(), 2);
    // A null handle has nothing to poison, and its body refuses it.
    let null = run_on(None, || Err(XLSX_GRAFT_ERR_NULL_ARG));
    assert_eq!(null, XLSX_GRAFT_ERR_NULL_ARG);
    assert_eq!(run_on(None, || panic!("boom")), XLSX_GRAFT_ERR_PANIC);
}

#[test]
fn text_that_is_not_utf8_is_an_invalid_argument() {
    let bytes = [b'a', 0xff];
    // SAFETY: `bytes` holds two bytes for the call.
    let both = unsafe { text(bytes.as_ptr(), 2) };
    assert_eq!(both, Err(XLSX_GRAFT_ERR_INVALID_ARG));
    // SAFETY: as above, one of them.
    assert_eq!(unsafe { text(bytes.as_ptr(), 1) }, Ok("a"));
}

#[test]
fn the_message_of_the_last_failure_is_lent() {
    failed("no such file");
    let (mut ptr, mut len) = (ptr::null(), 0);
    // SAFETY: both arguments are writable slots of the declared types.
    let status = unsafe { xlsx_graft_error_message(&raw mut ptr, &raw mut len) };
    assert_eq!(status, XLSX_GRAFT_OK);
    // SAFETY: on OK the message is `len` bytes, alive until the next failure.
    let message = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert_eq!(message, b"no such file");
}

#[test]
fn a_null_out_pointer_is_refused_for_the_message() {
    let mut len = usize::MAX;
    // SAFETY: `len` is a writable slot; the null `out_ptr` is what is tested.
    let status = unsafe { xlsx_graft_error_message(ptr::null_mut(), &raw mut len) };
    assert_eq!((status, len), (XLSX_GRAFT_ERR_NULL_ARG, usize::MAX));
    let mut out = ptr::dangling();
    // SAFETY: `out` is a writable slot; the null `out_len` is what is tested.
    let status = unsafe { xlsx_graft_error_message(&raw mut out, ptr::null_mut()) };
    assert_eq!((status, out), (XLSX_GRAFT_ERR_NULL_ARG, ptr::dangling()));
}

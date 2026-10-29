//! Unit tests for the C ABI calls: the value on success, and that no other
//! status reads the out-parameters.

use super::{Outcome, read, version};
use xlsx_graft_capi::{
    XLSX_GRAFT_ERR_IO, XLSX_GRAFT_ERR_NULL_ARG, XLSX_GRAFT_ERR_PANIC, XLSX_GRAFT_OK,
};

#[test]
fn the_version_is_the_engine_version() {
    // The addon and the engine share the workspace version.
    let expected = Outcome {
        status: XLSX_GRAFT_OK,
        value: Some(env!("CARGO_PKG_VERSION").to_owned()),
        message: None,
    };
    assert_eq!(version(), expected);
}

#[test]
fn ok_copies_the_lent_bytes() {
    let lent = b"1.2.3";
    // SAFETY: `lent` is alive for the call and holds `lent.len()` bytes.
    let outcome = unsafe { read(XLSX_GRAFT_OK, lent.as_ptr(), lent.len()) };
    assert_eq!(outcome.value.as_deref(), Some("1.2.3"));
}

#[test]
fn every_other_status_is_passed_through_without_a_value() {
    for status in [XLSX_GRAFT_ERR_NULL_ARG, XLSX_GRAFT_ERR_PANIC, 99] {
        // SAFETY: on a status other than XLSX_GRAFT_OK neither the null
        // pointer nor the length is read.
        let outcome = unsafe { read(status, std::ptr::null(), usize::MAX) };
        assert_eq!(
            outcome,
            Outcome {
                status,
                value: None,
                message: None,
            }
        );
    }
}

#[test]
fn a_lent_value_on_io_carries_the_c_abis_message_instead() {
    let book = crate::call::Book::open_path("/no/such/book.xlsx");
    drop(book);
    // SAFETY: on a status other than XLSX_GRAFT_OK neither the null
    // pointer nor the length is read.
    let outcome = unsafe { read(XLSX_GRAFT_ERR_IO, std::ptr::null(), 0) };
    let message = outcome.message.unwrap();
    assert!(message.starts_with("/no/such/book.xlsx: "), "{message}");
    assert_eq!((outcome.status, outcome.value), (XLSX_GRAFT_ERR_IO, None));
}

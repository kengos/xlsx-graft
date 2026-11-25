//! The bound on the memory of a [`StructureReport`]: the byte limit of what
//! it holds, and that it can be sent to and shared between threads.

use super::{KeptReference, MAX_AT_BYTES, MAX_KEPT, StructureReport};
use std::mem::size_of;

/// The byte limit of the references of a report: 20 MiB.
const MAX_REPORT_BYTES: usize = 20 * 1024 * 1024;

#[test]
fn the_references_a_report_holds_take_at_most_20_mib() {
    // One reference is its fields (an `Arc<str>` and a `&'static str` are a
    // pointer and a length), `MAX_AT_BYTES`, the byte limit of its `at`, and
    // the allocator's header for that. The `Vec` of references grows by
    // doubling, so up to one more reference's fields are reserved for each
    // reference held.
    let one = size_of::<KeptReference>() * 2 + MAX_AT_BYTES + 16;
    assert!(MAX_KEPT * one <= MAX_REPORT_BYTES);
    assert_eq!((MAX_KEPT, MAX_AT_BYTES), (16_384, 1_024));
}

#[test]
fn a_report_and_a_reference_can_be_sent_and_shared_between_threads() {
    fn is_send_and_sync<T: Send + Sync>() {}
    is_send_and_sync::<StructureReport>();
    is_send_and_sync::<KeptReference>();
}

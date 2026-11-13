//! Unit tests for the context a server holds.

use super::Context;
use xlsx_graft_host::{Input, Policy};

#[test]
fn a_default_context_holds_the_default_policy() {
    assert_eq!(Context::default().policy(), &Policy::default());
}

#[test]
fn a_context_holds_the_policy_it_is_given() {
    let policy = Policy::read(Input::Bytes(br#"{"maxEntries": 5}"#)).unwrap();
    assert_ne!(policy, Policy::default());
    assert_eq!(Context::new(policy.clone()).policy(), &policy);
}

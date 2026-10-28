//! The limits on the table, each at its value and one past it, and the
//! threshold past which it spills.

use super::{at, book, read_with, values};
use crate::{ReadLimits, SharedStrings};
use xlsx_graft_diagnostics::{Diagnostic, Limit};
use xlsx_graft_spool::MemorySpools;

/// `n` strings of one character each.
fn strings(n: usize) -> String {
    "<si><t>x</t></si>".repeat(n)
}

#[test]
fn the_defaults_are_the_documented_values() {
    let limits = ReadLimits::default();
    assert_eq!(limits.max_shared_strings, Limit::default_of(16_777_216));
    assert_eq!(
        limits.max_shared_strings_bytes,
        Limit::default_of(4_294_967_295)
    );
    assert_eq!(limits.shared_strings_spill_bytes, 32 * 1024 * 1024);
}

#[test]
fn as_many_strings_as_the_limit_read_and_one_more_is_refused() {
    let limits = ReadLimits {
        max_shared_strings: Limit::default_of(3),
        ..ReadLimits::default()
    };
    let (mut table, seen) = read_with(book(&strings(3)), &limits, &mut MemorySpools);
    assert_eq!(seen, vec![]);
    assert_eq!(values(&mut table), ["x", "x", "x"]);

    let (table, seen) = read_with(book(&strings(5)), &limits, &mut MemorySpools);
    assert!(matches!(table, SharedStrings::Unknown));
    let limit = limits.max_shared_strings;
    let diagnostic = Diagnostic::SharedStringsTooMany { count: 4, limit };
    assert_eq!(seen, vec![at(diagnostic, "/xl/sharedStrings.xml")]);
}

#[test]
fn as_many_bytes_of_text_as_the_limit_read_and_one_more_is_refused() {
    let limits = ReadLimits {
        max_shared_strings_bytes: Limit::default_of(5),
        ..ReadLimits::default()
    };
    // `é` is two bytes of UTF-8.
    let body = "<si><t>ab</t></si><si><r><t>c</t></r><r><t>é</t></r></si>";
    let (mut table, seen) = read_with(book(body), &limits, &mut MemorySpools);
    assert_eq!(seen, vec![]);
    assert_eq!(values(&mut table), ["ab", "cé"]);

    let body = "<si><t>ab</t></si><si><t>cdé</t></si>";
    let (table, seen) = read_with(book(body), &limits, &mut MemorySpools);
    assert!(matches!(table, SharedStrings::Unknown));
    let limit = limits.max_shared_strings_bytes;
    let diagnostic = Diagnostic::SharedStringsTooLarge { size: 6, limit };
    assert_eq!(seen, vec![at(diagnostic, "/xl/sharedStrings.xml")]);
}

#[test]
fn a_long_text_is_refused_as_it_passes_the_limit_not_at_its_end() {
    let limits = ReadLimits {
        max_shared_strings_bytes: Limit::default_of(5),
        ..ReadLimits::default()
    };
    // Each reference is an event of its own.
    let body = format!("<si><t>ab&amp;cd{}</t></si>", "&amp;efgh".repeat(1_000));
    let (table, seen) = read_with(book(&body), &limits, &mut MemorySpools);
    assert!(matches!(table, SharedStrings::Unknown));
    let limit = limits.max_shared_strings_bytes;
    let diagnostic = Diagnostic::SharedStringsTooLarge { size: 6, limit };
    assert_eq!(seen, vec![at(diagnostic, "/xl/sharedStrings.xml")]);
}

#[test]
fn the_limit_counts_the_text_with_its_escapes_undone() {
    let limits = ReadLimits {
        max_shared_strings_bytes: Limit::default_of(2),
        ..ReadLimits::default()
    };
    let body = "<si><t>_x0041__x0042_</t></si>";
    let (mut table, seen) = read_with(book(body), &limits, &mut MemorySpools);
    assert_eq!(seen, vec![]);
    assert_eq!(values(&mut table), ["AB"]);
}

#[test]
fn a_byte_limit_past_32_bits_is_held_to_them() {
    let limits = ReadLimits {
        max_shared_strings_bytes: Limit::default_of(u64::MAX),
        ..ReadLimits::default()
    };
    let (mut table, seen) = read_with(book(&strings(2)), &limits, &mut MemorySpools);
    assert_eq!(seen, vec![]);
    assert_eq!(values(&mut table), ["x", "x"]);
}

/// Reads `body` with the spill threshold at `threshold`, and returns the
/// table read.
fn read_at_threshold(body: &str, threshold: u64) -> SharedStrings {
    let limits = ReadLimits {
        shared_strings_spill_bytes: threshold,
        ..ReadLimits::default()
    };
    let (table, seen) = read_with(book(body), &limits, &mut MemorySpools);
    assert_eq!(seen, vec![]);
    table
}

#[test]
fn the_table_stays_in_memory_up_to_the_threshold_and_spills_past_it() {
    // Three strings of 3 bytes and their ends of 4 bytes: 21 bytes.
    let body = "<si><t>abc</t></si><si><t>def</t></si><si><t>ghi</t></si>";
    let mut table = read_at_threshold(body, 21);
    let SharedStrings::Table(inner) = &table else {
        panic!("{table:?}");
    };
    assert!(!inner.is_spilled());
    assert_eq!(values(&mut table), ["abc", "def", "ghi"]);

    let mut table = read_at_threshold(body, 20);
    let SharedStrings::Table(inner) = &table else {
        panic!("{table:?}");
    };
    assert!(inner.is_spilled());
    assert!(inner.memory_bytes() <= 20, "{inner:?}");
    assert_eq!(values(&mut table), ["abc", "def", "ghi"]);
}

#[test]
fn a_spilled_table_holds_no_more_memory_than_the_threshold() {
    let body: String = (0..500)
        .map(|i| format!("<si><t>string number {i}</t></si>"))
        .collect();
    let mut table = read_at_threshold(&body, 64);
    let SharedStrings::Table(inner) = &mut table else {
        panic!("{table:?}");
    };
    assert!(inner.is_spilled());
    assert!(inner.memory_bytes() <= 64, "{inner:?}");
    let mut out = String::new();
    for i in [0, 1, 250, 499] {
        assert_eq!(inner.get(i, &mut out).unwrap(), crate::Lookup::Found);
        assert_eq!(out, format!("string number {i}"));
    }
    assert_eq!(inner.len(), 500);
}

#[test]
fn a_threshold_of_nothing_spills_at_the_first_byte() {
    let mut table = read_at_threshold("<si><t>a</t></si><si/>", 0);
    let SharedStrings::Table(inner) = &table else {
        panic!("{table:?}");
    };
    assert!(inner.is_spilled());
    assert_eq!(inner.memory_bytes(), 0);
    assert_eq!(values(&mut table), ["a", ""]);
}

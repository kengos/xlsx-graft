//! `first_overlap`: the ranges that overlap found by sweeping the rows, and
//! its agreement with comparing every pair.

use super::range;
use crate::sheet::CellRange;
use crate::sheet::merges::first_overlap;
use proptest::prelude::*;
use xlsx_graft_cell::CellRef;

#[test]
fn ranges_that_only_touch_have_no_overlap() {
    let added = [range("A1:B2"), range("C1:D2"), range("A3:B4"), range("E5")];
    assert_eq!(first_overlap(&[], &added), None);
    assert_eq!(first_overlap(&[], &[]), None);
    assert_eq!(first_overlap(&[range("A1:B2")], &[range("C3:D4")]), None);
}

#[test]
fn an_added_range_that_overlaps_an_existing_one_is_the_later_of_the_pair() {
    let existing = [range("A1:C3"), range("E1:F2")];
    let got = first_overlap(&existing, &[range("H1:H2"), range("B3:B6")]);
    assert_eq!(got, Some((range("B3:B6"), range("A1:C3"))));
    let got = first_overlap(&existing, &[range("F2:G3")]);
    assert_eq!(got, Some((range("F2:G3"), range("E1:F2"))));
}

#[test]
fn the_same_range_twice_and_a_range_inside_another_overlap() {
    let got = first_overlap(&[], &[range("A1:B2"), range("A1:B2")]);
    assert_eq!(got, Some((range("A1:B2"), range("A1:B2"))));
    let got = first_overlap(&[], &[range("C3:D4"), range("A1:H9")]);
    assert_eq!(got, Some((range("A1:H9"), range("C3:D4"))));
}

#[test]
fn an_overlap_with_the_range_whose_left_column_is_right_of_the_new_ranges_left_column_is_found() {
    let got = first_overlap(&[], &[range("B1:C3"), range("A2:B2")]);
    assert_eq!(got, Some((range("A2:B2"), range("B1:C3"))));
}

#[test]
fn a_range_whose_bottom_row_is_above_a_later_range_does_not_overlap_it() {
    let added = [
        range("A1:B2"),
        range("A3:B4"),
        range("A5:B6"),
        range("B7:C8"),
        range("A9:A9"),
    ];
    assert_eq!(first_overlap(&[], &added), None);
}

fn any_range() -> impl Strategy<Value = CellRange> {
    ((1_u16..=7, 1_u32..=7), (1_u16..=7, 1_u32..=7)).prop_map(|((c1, r1), (c2, r2))| {
        CellRange::new(CellRef::new(c1, r1).unwrap(), CellRef::new(c2, r2).unwrap())
    })
}

proptest! {
    #[test]
    fn the_sweep_agrees_with_comparing_every_pair(
        existing in prop::collection::vec(any_range(), 0..6),
        added in prop::collection::vec(any_range(), 0..10),
    ) {
        let all: Vec<_> = existing.iter().chain(&added).copied().collect();
        let brute = (0..all.len()).any(|j| (0..j).any(|i| all[i].overlaps(all[j])));
        match first_overlap(&existing, &added) {
            None => prop_assert!(!brute),
            Some((later, earlier)) => {
                prop_assert!(brute);
                prop_assert!(later.overlaps(earlier));
                let in_order = (0..all.len()).any(|j| {
                    (0..j).any(|i| all[i] == earlier && all[j] == later)
                });
                prop_assert!(in_order);
            }
        }
    }
}

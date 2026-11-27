//! `CellRange`: the corners in either order, parsing, the A1 text and when
//! two ranges share a cell.

use super::{cell, range};
use crate::sheet::CellRange;
use proptest::prelude::*;
use xlsx_graft_cell::{CellRef, ValueError};

#[test]
fn the_corners_are_kept_as_top_left_and_bottom_right_in_either_order() {
    let normal = CellRange::new(cell("B2"), cell("D5"));
    for (a, b) in [("D5", "B2"), ("B5", "D2"), ("D2", "B5")] {
        assert_eq!(CellRange::new(cell(a), cell(b)), normal);
    }
    assert_eq!((normal.first(), normal.last()), (cell("B2"), cell("D5")));
    assert!(!normal.is_single_cell());
    assert!(CellRange::new(cell("C3"), cell("C3")).is_single_cell());
}

#[test]
fn a_range_is_parsed_from_one_cell_or_two() {
    assert_eq!(range("A1:B2"), CellRange::new(cell("A1"), cell("B2")));
    assert_eq!(range("B2:A1"), range("A1:B2"));
    assert_eq!(range("c3"), CellRange::new(cell("C3"), cell("C3")));
    assert_eq!(
        CellRange::parse("XFD1048576:A1"),
        Ok(range("A1:XFD1048576"))
    );
    assert_eq!(CellRange::parse(""), Err(ValueError::Form));
    assert_eq!(CellRange::parse("A1:"), Err(ValueError::Form));
    assert_eq!(CellRange::parse(":A1"), Err(ValueError::Form));
    assert_eq!(CellRange::parse("A1:B2:C3"), Err(ValueError::Form));
    assert_eq!(CellRange::parse("$A$1:B2"), Err(ValueError::Form));
    assert_eq!(CellRange::parse("A1:XFE1"), Err(ValueError::Range));
    assert_eq!(CellRange::parse("A0"), Err(ValueError::Range));
}

#[test]
fn a_range_is_written_as_two_corners_even_for_one_cell() {
    assert_eq!(range("B2:A1").to_string(), "A1:B2");
    assert_eq!(range("C3").to_string(), "C3:C3");
    assert_eq!(range("A1:XFD1048576").to_string(), "A1:XFD1048576");
}

#[test]
fn ranges_overlap_when_they_share_a_cell_and_not_when_they_touch() {
    let base = range("B2:D4");
    for sharing in ["B2:D4", "C3", "A1:B2", "D4:F6", "A1:Z9", "C1:C9", "B3:D3"] {
        assert!(base.overlaps(range(sharing)), "{sharing}");
        assert!(range(sharing).overlaps(base), "{sharing}");
    }
    for apart in [
        "E2:F4", "A2:A4", "B5:D6", "B1:D1", "E5", "A1", "F1:F9", "A5:A9",
    ] {
        assert!(!base.overlaps(range(apart)), "{apart}");
        assert!(!range(apart).overlaps(base), "{apart}");
    }
}

#[test]
fn ranges_are_ordered_by_top_row_left_column_bottom_row_then_right_column() {
    let sorted = ["C1:A2", "B1:Z9", "A2:B2", "A2:C2", "A2:A3", "B2:B2", "A3"];
    let mut shuffled = sorted.map(range);
    shuffled.reverse();
    shuffled.sort();
    assert_eq!(shuffled, sorted.map(range));
    let (low, high) = (range("A1:A1"), range("B1:B1"));
    assert_eq!(low.cmp(&high), std::cmp::Ordering::Less);
    assert_eq!(high.partial_cmp(&low), Some(std::cmp::Ordering::Greater));
    assert_eq!(
        low.partial_cmp(&range("A1")),
        Some(std::cmp::Ordering::Equal)
    );
}

#[test]
fn equal_ranges_hash_alike_and_different_ranges_are_distinct_in_a_set() {
    let set: std::collections::HashSet<CellRange> =
        ["A1:B2", "B2:A1", "A1:B3", "A2:B2", "B1:B2", "A1:A2"]
            .map(range)
            .into();
    assert_eq!(set.len(), 5);
    let hash = |r: CellRange| {
        let mut hasher = std::hash::DefaultHasher::new();
        std::hash::Hash::hash(&r, &mut hasher);
        std::hash::Hasher::finish(&hasher)
    };
    assert_ne!(hash(range("A1:B2")), hash(range("A1:B3")));
}

fn corner() -> impl Strategy<Value = CellRef> {
    (1_u16..=8, 1_u32..=8).prop_map(|(c, r)| CellRef::new(c, r).unwrap())
}

proptest! {
    #[test]
    fn the_corners_are_normalized_and_overlap_is_a_shared_cell(
        a in corner(), b in corner(), c in corner(), d in corner(),
    ) {
        let one = CellRange::new(a, b);
        let two = CellRange::new(c, d);
        prop_assert_eq!(one, CellRange::new(b, a));
        prop_assert!(one.first().column() <= one.last().column());
        prop_assert!(one.first().row() <= one.last().row());
        prop_assert_eq!(CellRange::parse(&one.to_string()), Ok(one));
        let shares = (1..=8_u16).any(|column| (1..=8_u32).any(|row| {
            let inside = |r: CellRange| {
                (r.first().column()..=r.last().column()).contains(&column)
                    && (r.first().row()..=r.last().row()).contains(&row)
            };
            inside(one) && inside(two)
        }));
        prop_assert_eq!(one.overlaps(two), shares);
    }
}

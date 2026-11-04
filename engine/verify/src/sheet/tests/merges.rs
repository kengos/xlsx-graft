//! The merged cells of a sheet: those that overlap, and how many are kept
//! to check.

use super::{COUNTS, check, check_with, worksheet};
use crate::sheet::merges::{Range, dropped};
use xlsx_graft_diagnostics::{Diagnostic, Echo, Limit};

/// A worksheet merging `ranges`, in order.
fn merging(ranges: &[&str]) -> String {
    let cells: String = ranges
        .iter()
        .map(|range| format!(r#"<mergeCell ref="{range}"/>"#))
        .collect();
    worksheet(&format!("<sheetData/><mergeCells>{cells}</mergeCells>"))
}

/// `merged_cells_overlap` of `reference` with `other`.
fn overlap(reference: &str, other: &str) -> Diagnostic {
    Diagnostic::MergedCellsOverlap {
        reference: Echo::new(reference),
        other: Echo::new(other),
    }
}

#[test]
fn merged_ranges_that_overlap_are_refused() {
    // Excel for Mac 16.114.3 offers to repair both, dropping one.
    assert_eq!(
        check(&merging(&["G1:H2", "H2:I3"])),
        [overlap("H2:I3", "G1:H2")]
    );
    assert_eq!(
        check(&merging(&["G1:H2", "G1:H2"])),
        [overlap("G1:H2", "G1:H2")]
    );
}

#[test]
fn merged_ranges_that_touch_at_an_edge_or_a_corner_pass() {
    // Excel for Mac 16.114.3 takes both as they are.
    assert_eq!(check(&merging(&["G1:H2", "I3:J4"])), []);
    assert_eq!(check(&merging(&["G1:H2", "I1:J2"])), []);
    assert_eq!(check(&merging(&["G1:H2", "G3:H4", "F1:F9", "A1"])), []);
}

#[test]
fn the_range_that_comes_later_in_the_sheet_is_named_first() {
    // Read from the top down, G2:H3 is taken after G1:G2, which comes
    // later in the sheet.
    assert_eq!(
        check(&merging(&["G2:H3", "G1:G2"])),
        [overlap("G1:G2", "G2:H3")]
    );
}

#[test]
fn an_overlap_is_found_on_either_side_and_after_ranges_close() {
    // A5:B5 overlaps B1:C9 on its right; D1:D2 is closed before D5:E6 is
    // taken, which D6:D9 overlaps on its left; and G1:H2, its corners
    // given the other way round as H2:I3's are, overlaps it on its left,
    // though it is taken first. They come in the order of the sheet.
    let ranges = [
        "B1:C9", "D1:D2", "A5:B5", "E6:D5", "I3:H2", "H1:G2", "D6:D9",
    ];
    let expected = [
        overlap("A5:B5", "B1:C9"),
        overlap("G1:H2", "H2:I3"),
        overlap("D6:D9", "D5:E6"),
    ];
    assert_eq!(check(&merging(&ranges)), expected);
}

#[test]
fn the_range_that_comes_later_is_dropped_so_one_that_overlaps_only_it_is_found() {
    // G1:G2 is dropped for G2:H3, which comes before it, so H3:H4, which
    // overlaps G2:H3 alone, is found though G1:G2 was taken first.
    let expected = [overlap("G1:G2", "G2:H3"), overlap("H3:H4", "G2:H3")];
    assert_eq!(check(&merging(&["G2:H3", "G1:G2", "H3:H4"])), expected);
    // A range that comes first is kept over every later one it overlaps.
    let expected = [overlap("A1:A2", "A2:C2"), overlap("C1:C2", "A2:C2")];
    assert_eq!(check(&merging(&["A2:C2", "A1:A2", "C1:C2"])), expected);
}

#[test]
fn a_range_dropped_is_no_reason_to_drop_another() {
    // A1:B3 is dropped for B3:C3, so A2:A4, which overlaps A1:B3 alone, is
    // kept, and A4:A5, which overlaps it, dropped.
    let ranges = ["B3:C3", "A1:B3", "A2:A4", "A4:A5"];
    let expected = [overlap("A1:B3", "B3:C3"), overlap("A4:A5", "A2:A4")];
    assert_eq!(check(&merging(&ranges)), expected);
    let ranges = ["C1:C3", "A2:C2", "A1:A3", "A3:A3"];
    let expected = [overlap("A2:C2", "C1:C3"), overlap("A3:A3", "A1:A3")];
    assert_eq!(check(&merging(&ranges)), expected);
}

#[test]
fn the_same_range_many_times_is_dropped_all_but_once() {
    // As many as the diagnostics keep.
    let ranges = vec!["A1:XFD1048576"; 1_000];
    let found = check_with(&merging(&ranges), COUNTS, 1_000);
    assert_eq!(found.len(), 999);
    assert!(
        found
            .iter()
            .all(|found| *found == overlap("A1:XFD1048576", "A1:XFD1048576"))
    );
}

/// The ranges, as `(top, bottom, left, right)`.
fn ranges(corners: &[(u32, u32, u16, u16)]) -> Vec<Range> {
    (corners.iter())
        .map(|&(top, bottom, left, right)| Range {
            top,
            bottom,
            left,
            right,
        })
        .collect()
}

/// Whether `a` and `b` share a cell.
fn overlaps(a: &Range, b: &Range) -> bool {
    a.top <= b.bottom && b.top <= a.bottom && a.left <= b.right && b.left <= a.right
}

/// Checks `dropped` against every pair of `ranges`, taken in order: a range
/// is dropped exactly when it overlaps one taken before it, and what it is
/// dropped for is such a one.
fn assert_greedy(ranges: &[Range]) {
    let found = dropped(ranges);
    let mut taken: Vec<usize> = Vec::new();
    for (index, range) in ranges.iter().enumerate() {
        let before = taken.iter().any(|&other| overlaps(&ranges[other], range));
        match found[index] {
            Some(other) => {
                assert!(before, "{ranges:?}: {index} is dropped");
                assert!(taken.contains(&other), "{ranges:?}: {index} for {other}");
                assert!(
                    overlaps(&ranges[other], range),
                    "{ranges:?}: {index} for {other}"
                );
            }
            None => {
                assert!(!before, "{ranges:?}: {index} is kept");
                taken.push(index);
            }
        }
    }
}

/// Every range of a sheet of `rows` by `columns`.
fn every_range(rows: u32, columns: u16) -> Vec<(u32, u32, u16, u16)> {
    let mut every = Vec::new();
    for top in 1..=rows {
        for bottom in top..=rows {
            for left in 1..=columns {
                for right in left..=columns {
                    every.push((top, bottom, left, right));
                }
            }
        }
    }
    every
}

#[test]
fn every_layout_of_two_and_three_ranges_on_three_rows_and_columns_is_taken_in_order() {
    let every = every_range(3, 3);
    for a in &every {
        for b in &every {
            assert_greedy(&ranges(&[*a, *b]));
            for c in &every {
                assert_greedy(&ranges(&[*a, *b, *c]));
            }
        }
    }
}

#[test]
fn every_layout_of_four_ranges_on_two_rows_and_three_columns_is_taken_in_order() {
    let every = every_range(2, 3);
    for a in &every {
        for b in &every {
            for c in &every {
                for d in &every {
                    assert_greedy(&ranges(&[*a, *b, *c, *d]));
                }
            }
        }
    }
}

#[test]
fn sparse_ranges_across_the_sheet_at_random_are_taken_in_order() {
    // Ranges tall and wide over every column, most kept: many open at once
    // on a row, and many opening while the one checked is open, marked on
    // a tree 14 levels deep.
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = |below: u64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 33) % below
    };
    for _ in 0..10 {
        let corners: Vec<_> = (0..2_000)
            .map(|_| {
                let top = u32::try_from(next(2_000) + 1).unwrap();
                let bottom = top + u32::try_from(next(200)).unwrap();
                let left = u16::try_from(next(16_384) + 1).unwrap();
                let right = left
                    .saturating_add(u16::try_from(next(3_000)).unwrap())
                    .min(16_384);
                (top, bottom, left, right)
            })
            .collect();
        let ranges = ranges(&corners);
        assert_greedy(&ranges);
        let kept = dropped(&ranges)
            .iter()
            .filter(|other| other.is_none())
            .count();
        assert!(kept > 100, "{kept} kept");
    }
}

#[test]
fn many_ranges_at_random_are_taken_in_order() {
    // Enough ranges for blocks many levels deep.
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = |below: u64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 33) % below
    };
    for count in [70, 300, 2_000] {
        for _ in 0..20 {
            let corners: Vec<_> = (0..count)
                .map(|_| {
                    let (top, left) = (next(12) + 1, next(12) + 1);
                    let (height, width) = (next(3), next(3));
                    let left = u16::try_from(left).unwrap();
                    let width = u16::try_from(width).unwrap();
                    (
                        u32::try_from(top).unwrap(),
                        u32::try_from(top + height).unwrap(),
                        left,
                        left + width,
                    )
                })
                .collect();
            assert_greedy(&ranges(&corners));
        }
    }
}

#[test]
fn a_range_that_does_not_read_is_not_checked() {
    let ranges = ["A1:B2", "A1:", "$A$1:B2", "A1:B2:C3", "XFE1:XFE2", "A0"];
    assert_eq!(check(&merging(&ranges)), []);
    let xml = worksheet(r#"<mergeCells><mergeCell/><mergeCell ref="A1:B2"/></mergeCells>"#);
    assert_eq!(check(&xml), []);
}

#[test]
fn merged_ranges_are_kept_up_to_the_limit_and_one_past_it_is_refused() {
    let ranges = ["A1:B2", "C1:D2", "E1:F2"];
    assert_eq!(check_with(&merging(&ranges), COUNTS, 3), []);
    // A `mergeCell` whose `ref` does not read counts all the same.
    let unread = ["A1:B2", "bad", "C1:D2", "E1:F2"];
    let expected = [Diagnostic::MergedCellsTooMany {
        count: 4,
        limit: Limit::default_of(3),
    }];
    assert_eq!(check_with(&merging(&unread), COUNTS, 3), expected);
    let past = ["A1:B2", "C1:D2", "E1:F2", "A1:B2"];
    let expected = [Diagnostic::MergedCellsTooMany {
        count: 4,
        limit: Limit::default_of(3),
    }];
    // The range past the limit is not checked, so its overlap is not found.
    assert_eq!(check_with(&merging(&past), COUNTS, 3), expected);
}

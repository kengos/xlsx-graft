//! `ChildPlacer`: where the kinds a caller wants are due, what each child
//! is to the caller, and the properties of any sequence.
//!
//! All three properties hold for any sequence of root children, in or out of
//! the table order: every wanted kind is due once, in table order; a child's
//! slot follows from whether its kind is wanted and already due; and where a
//! kind is due (no later kind has started, `found` says whether the kind
//! started, a found kind is due at the start after its last element, a
//! missing kind before a child of a later kind). The third is checked on
//! sequences in table order and on sequences in any order.

use crate::sheet::RootChild::{self, Other, Table, Wrapped};
use crate::sheet::WorksheetChild::{self, *};
use crate::sheet::{ChildPlacer, Due, Placement, Slot};
use proptest::prelude::*;

fn due(child: WorksheetChild, found: bool) -> Due {
    Due { child, found }
}

fn placement(due: Vec<Due>, slot: Slot) -> Placement {
    Placement { due, slot }
}

/// What `start` answers for each of `starts`, then what `finish` answers.
fn run(wanted: &[WorksheetChild], starts: &[RootChild]) -> (Vec<Placement>, Vec<Due>) {
    let mut placer = ChildPlacer::new(wanted.iter().copied());
    let placed = starts.iter().map(|&c| placer.start(c)).collect();
    (placed, placer.finish())
}

#[test]
fn a_kind_the_part_has_is_wanted_where_it_is_and_due_at_the_next_kind() {
    let (placed, rest) = run(
        &[SheetViews],
        &[Table(SheetPr), Table(SheetViews), Table(SheetData)],
    );
    assert_eq!(
        placed,
        [
            placement(vec![], Slot::Pass),
            placement(vec![], Slot::Wanted),
            placement(vec![due(SheetViews, true)], Slot::Pass),
        ]
    );
    assert_eq!(rest, []);
}

#[test]
fn a_kind_the_part_lacks_is_due_before_the_first_kind_after_it() {
    let (placed, rest) = run(
        &[SheetViews],
        &[Table(SheetPr), Table(SheetData), Table(PageMargins)],
    );
    assert_eq!(
        placed,
        [
            placement(vec![], Slot::Pass),
            placement(vec![due(SheetViews, false)], Slot::Pass),
            placement(vec![], Slot::Pass),
        ]
    );
    assert_eq!(rest, []);
}

#[test]
fn a_repeating_kind_is_due_after_its_last_element() {
    let starts = [
        Table(MergeCells),
        Table(ConditionalFormatting),
        Table(ConditionalFormatting),
        Table(DataValidations),
    ];
    let (placed, rest) = run(&[ConditionalFormatting], &starts);
    assert_eq!(
        placed,
        [
            placement(vec![], Slot::Pass),
            placement(vec![], Slot::Wanted),
            placement(vec![], Slot::Wanted),
            placement(vec![due(ConditionalFormatting, true)], Slot::Pass),
        ]
    );
    assert_eq!(rest, []);
}

#[test]
fn an_other_element_passes_and_makes_no_missing_kind_due() {
    let (placed, rest) = run(&[Dimension], &[Other, Table(SheetData), Other]);
    assert_eq!(
        placed,
        [
            placement(vec![], Slot::Pass),
            placement(vec![due(Dimension, false)], Slot::Pass),
            placement(vec![], Slot::Pass),
        ]
    );
    assert_eq!(rest, []);
}

#[test]
fn a_kind_the_part_has_is_due_at_the_start_of_an_other_element() {
    let (placed, rest) = run(&[Dimension], &[Table(Dimension), Other, Table(SheetData)]);
    assert_eq!(
        placed,
        [
            placement(vec![], Slot::Wanted),
            placement(vec![due(Dimension, true)], Slot::Pass),
            placement(vec![], Slot::Pass),
        ]
    );
    assert_eq!(rest, []);
}

#[test]
fn a_kind_missing_before_a_wrapped_child_is_due_before_the_wrapper() {
    for wrapped in [OleObjects, Controls] {
        let (placed, rest) = run(
            &[Picture],
            &[Table(LegacyDrawing), Wrapped(wrapped), Table(TableParts)],
        );
        assert_eq!(
            placed,
            [
                placement(vec![], Slot::Pass),
                placement(vec![due(Picture, false)], Slot::Pass),
                placement(vec![], Slot::Pass),
            ]
        );
        assert_eq!(rest, []);
    }
}

#[test]
fn a_kind_missing_after_the_wrapped_kind_is_not_due_at_the_wrapper() {
    let (placed, rest) = run(&[TableParts], &[Wrapped(OleObjects), Table(ExtLst)]);
    assert_eq!(
        placed,
        [
            placement(vec![], Slot::Pass),
            placement(vec![due(TableParts, false)], Slot::Pass),
        ]
    );
    assert_eq!(rest, []);
}

#[test]
fn a_kind_the_part_has_is_due_before_a_wrapper() {
    let wrappers: [&[RootChild]; 3] = [
        &[Wrapped(OleObjects)],
        &[Wrapped(Controls)],
        &[Wrapped(OleObjects), Wrapped(Controls)],
    ];
    for wrapped in wrappers {
        let mut starts = vec![Table(LegacyDrawing)];
        starts.extend(wrapped);
        let (placed, rest) = run(&[LegacyDrawing], &starts);
        assert_eq!(placed[0], placement(vec![], Slot::Wanted));
        assert_eq!(
            placed[1],
            placement(vec![due(LegacyDrawing, true)], Slot::Pass)
        );
        for later in &placed[2..] {
            assert_eq!(*later, placement(vec![], Slot::Pass));
        }
        assert_eq!(rest, []);
    }
}

#[test]
fn a_wanted_kind_that_is_wrapped_is_written_where_it_is() {
    let (placed, rest) = run(&[OleObjects], &[Wrapped(OleObjects), Table(TableParts)]);
    assert_eq!(
        placed,
        [
            placement(vec![], Slot::Wrapped),
            placement(vec![due(OleObjects, true)], Slot::Pass),
        ]
    );
    assert_eq!(rest, []);
}

#[test]
fn a_kind_after_its_place_was_passed_is_late_and_not_found() {
    let (placed, rest) = run(
        &[SheetViews],
        &[
            Table(SheetData),
            Table(SheetViews),
            Wrapped(SheetViews),
            Table(SheetViews),
        ],
    );
    assert_eq!(
        placed,
        [
            placement(vec![due(SheetViews, false)], Slot::Pass),
            placement(vec![], Slot::Late),
            placement(vec![], Slot::Late),
            placement(vec![], Slot::Late),
        ]
    );
    assert_eq!(rest, []);
}

#[test]
fn a_part_out_of_order_still_makes_a_found_kind_due_at_the_next_child() {
    let (placed, rest) = run(
        &[Drawing],
        &[Table(Drawing), Table(SheetData), Table(Drawing)],
    );
    assert_eq!(
        placed,
        [
            placement(vec![], Slot::Wanted),
            placement(vec![due(Drawing, true)], Slot::Pass),
            placement(vec![], Slot::Late),
        ]
    );
    assert_eq!(rest, []);
}

#[test]
fn finish_returns_the_kinds_not_yet_due_in_table_order() {
    let wanted = [TableParts, Drawing, SheetData, SheetPr];
    let (placed, rest) = run(&wanted, &[Table(SheetPr), Table(SheetData), Table(Drawing)]);
    assert_eq!(
        placed[2],
        placement(vec![due(SheetData, true)], Slot::Wanted)
    );
    assert_eq!(rest, [due(Drawing, true), due(TableParts, false)]);
    let (_, rest) = run(&wanted, &[]);
    let kinds: Vec<_> = rest.iter().map(|d| (d.child, d.found)).collect();
    assert_eq!(
        kinds,
        [
            (SheetPr, false),
            (SheetData, false),
            (Drawing, false),
            (TableParts, false)
        ]
    );
}

#[test]
fn a_kind_is_due_once_and_a_kind_not_wanted_passes() {
    let (placed, rest) = run(
        &[Cols, PageSetup],
        &[Table(SheetData), Table(PageMargins), Table(SheetData)],
    );
    assert_eq!(placed[0], placement(vec![due(Cols, false)], Slot::Pass));
    assert_eq!(placed[1], placement(vec![], Slot::Pass));
    assert_eq!(placed[2], placement(vec![], Slot::Pass));
    assert_eq!(rest, [due(PageSetup, false)]);
}

#[test]
fn a_new_sheet_writes_the_kinds_before_sheet_data_then_after_it() {
    let mut placer = ChildPlacer::new([SheetViews, Cols, MergeCells, PageMargins]);
    let before = placer.start(Table(SheetData));
    assert_eq!(before.slot, Slot::Pass);
    assert_eq!(before.due, [due(SheetViews, false), due(Cols, false)]);
    assert_eq!(
        placer.finish(),
        [due(MergeCells, false), due(PageMargins, false)]
    );
}

fn child_strategy() -> impl Strategy<Value = WorksheetChild> {
    (0..WorksheetChild::ALL.len()).prop_map(|i| WorksheetChild::ALL[i])
}

fn root_child_strategy() -> impl Strategy<Value = RootChild> {
    prop_oneof![
        child_strategy().prop_map(Table),
        child_strategy().prop_map(Wrapped),
        Just(Other),
    ]
}

/// Kinds in non-decreasing table order, each as a table child or a wrapper,
/// with up to two `Other` elements before each and one at the end.
fn ordered_strategy() -> impl Strategy<Value = Vec<RootChild>> {
    (
        prop::collection::vec((child_strategy(), any::<bool>(), 0..3usize), 0..20),
        0..2usize,
    )
        .prop_map(|(mut items, last)| {
            items.sort_by_key(|&(kind, _, _)| kind);
            let mut starts = Vec::new();
            for (kind, wrapped, others) in items {
                starts.extend(std::iter::repeat_n(Other, others));
                starts.push(if wrapped { Wrapped(kind) } else { Table(kind) });
            }
            starts.extend(std::iter::repeat_n(Other, last));
            starts
        })
}

fn kind_of(child: RootChild) -> Option<WorksheetChild> {
    match child {
        Table(kind) | Wrapped(kind) => Some(kind),
        Other => None,
    }
}

proptest! {
    #[test]
    fn every_wanted_kind_is_due_once_in_table_order(
        wanted in prop::collection::vec(child_strategy(), 0..12),
        starts in prop::collection::vec(root_child_strategy(), 0..40),
    ) {
        let mut placer = ChildPlacer::new(wanted.iter().copied());
        let mut dues: Vec<Due> = Vec::new();
        for &start in &starts {
            let placed = placer.start(start);
            prop_assert!(placed.due.windows(2).all(|pair| pair[0].child < pair[1].child));
            dues.extend(placed.due);
        }
        let rest = placer.finish();
        prop_assert!(rest.windows(2).all(|pair| pair[0].child < pair[1].child));
        dues.extend(rest);
        let mut expected: Vec<_> = wanted.clone();
        expected.sort();
        expected.dedup();
        let mut got: Vec<_> = dues.iter().map(|d| d.child).collect();
        got.sort();
        prop_assert_eq!(got, expected);
    }

    #[test]
    fn a_kind_is_due_where_the_table_order_and_the_part_say(
        wanted in prop::collection::vec(child_strategy(), 0..12),
        starts in prop_oneof![
            ordered_strategy(),
            prop::collection::vec(root_child_strategy(), 0..40),
        ],
    ) {
        let mut placer = ChildPlacer::new(wanted.iter().copied());
        let mut started: Vec<WorksheetChild> = Vec::new();
        for (index, &start) in starts.iter().enumerate() {
            let placed = placer.start(start);
            for d in &placed.due {
                // No kind after the due kind has started.
                prop_assert!(started.iter().all(|&s| s <= d.child));
                prop_assert_eq!(d.found, started.contains(&d.child));
                if d.found {
                    // The first start after a run of elements of the kind.
                    prop_assert!(index > 0);
                    prop_assert_eq!(kind_of(starts[index - 1]), Some(d.child));
                } else {
                    // Before a table or wrapped child of a later kind.
                    prop_assert!(kind_of(start).is_some_and(|k| k > d.child));
                }
            }
            started.extend(kind_of(start));
        }
        for d in placer.finish() {
            prop_assert!(started.iter().all(|&s| s <= d.child));
            prop_assert_eq!(d.found, started.contains(&d.child));
            if d.found {
                prop_assert_eq!(starts.last().copied().and_then(kind_of), Some(d.child));
            }
        }
    }

    #[test]
    fn a_child_is_wanted_until_its_kind_is_due_then_late(
        wanted in prop::collection::vec(child_strategy(), 0..12),
        starts in prop::collection::vec(root_child_strategy(), 0..40),
    ) {
        let mut placer = ChildPlacer::new(wanted.iter().copied());
        let mut due_kinds: Vec<WorksheetChild> = Vec::new();
        for &c in &starts {
            let placed = placer.start(c);
            due_kinds.extend(placed.due.iter().map(|d| d.child));
            let slot = match kind_of(c) {
                Some(k) if wanted.contains(&k) && due_kinds.contains(&k) => Slot::Late,
                Some(k) if wanted.contains(&k) && matches!(c, Wrapped(_)) => Slot::Wrapped,
                Some(k) if wanted.contains(&k) => Slot::Wanted,
                _ => Slot::Pass,
            };
            prop_assert_eq!(placed.slot, slot);
        }
    }
}

//! Where the workbook views point after a delete, and what `open` makes of
//! the pointers the workbook has.

use super::{open, open_with};
use crate::structure::edit::state::{Pointer, ViewEntry};

const HIDDEN: &str = r#"state="hidden""#;

fn views(edit: &crate::structure::edit::StructureEdit<'_>) -> Vec<ViewEntry> {
    edit.state.views.clone()
}

#[test]
fn a_pointer_past_the_sheets_or_not_written_is_kept_as_a_raw_pointer() {
    let books =
        r#"<bookViews><workbookView activeTab="9" firstSheet="2"/><workbookView/></bookViews>"#;
    let edit = open_with(books, &[("A", ""), ("B", ""), ("C", "")], "");
    assert_eq!(
        views(&edit),
        [
            ViewEntry {
                active: Pointer::Raw,
                first: Pointer::Key(2)
            },
            ViewEntry {
                active: Pointer::Key(0),
                first: Pointer::Key(0)
            },
        ]
    );
    assert!(views(&open(&["A"], "")).is_empty());
}

#[test]
fn the_next_visible_sheet_after_a_deleted_one_is_pointed_at_else_the_previous_visible() {
    let books = r#"<bookViews><workbookView activeTab="1" firstSheet="3"/></bookViews>"#;
    let sheets = [("A", ""), ("B", ""), ("C", HIDDEN), ("D", "")];
    // B is active: C is hidden, so D is next. D is first: A is the last
    // visible sheet before it.
    let mut edit = open_with(books, &sheets, "");
    edit.delete_sheet("B").unwrap();
    assert_eq!(
        views(&edit),
        [ViewEntry {
            active: Pointer::Key(3),
            first: Pointer::Key(3)
        }]
    );
    edit.delete_sheet("D").unwrap();
    assert_eq!(
        views(&edit),
        [ViewEntry {
            active: Pointer::Key(0),
            first: Pointer::Key(0)
        }]
    );
}

#[test]
fn a_view_that_points_at_a_sheet_not_deleted_or_past_the_sheets_stays() {
    let books = r#"<bookViews><workbookView activeTab="0" firstSheet="9"/></bookViews>"#;
    let mut edit = open_with(books, &[("A", ""), ("B", "")], "");
    edit.delete_sheet("B").unwrap();
    assert_eq!(
        views(&edit),
        [ViewEntry {
            active: Pointer::Key(0),
            first: Pointer::Raw
        }]
    );
    // A move changes no pointer: it names the sheet by its key.
    let mut edit = open_with(books, &[("A", ""), ("B", "")], "");
    edit.move_sheet("A", 1).unwrap();
    assert_eq!(views(&edit)[0].active, Pointer::Key(0));
}

#[test]
fn a_pointer_equal_to_the_number_of_sheets_is_kept_as_a_raw_pointer() {
    let books = r#"<bookViews><workbookView activeTab="2" firstSheet="2"/></bookViews>"#;
    let edit = open_with(books, &[("A", ""), ("B", "")], "");
    assert_eq!(
        views(&edit),
        [ViewEntry {
            active: Pointer::Raw,
            first: Pointer::Raw
        }]
    );
}

#[test]
fn the_last_visible_sheet_before_the_deleted_one_is_pointed_at_when_none_is_after_it() {
    let books = r#"<bookViews><workbookView activeTab="2" firstSheet="2"/></bookViews>"#;
    let mut edit = open_with(books, &[("A", ""), ("B", ""), ("C", "")], "");
    edit.delete_sheet("C").unwrap();
    // B (key 1) is the nearest visible sheet before C, not A (key 0).
    assert_eq!(
        views(&edit),
        [ViewEntry {
            active: Pointer::Key(1),
            first: Pointer::Key(1)
        }]
    );
}

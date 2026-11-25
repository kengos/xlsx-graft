//! `add_sheet`, `rename_sheet`, `delete_sheet` and `move_sheet`: the state
//! each leaves, and that a call that fails leaves the state as it was.

use super::{defined, fingerprint, names_of, open, open_with, sheet_names};
use crate::structure::edit::state::{Origin, Scope};
use crate::structure::model::SheetState;
use crate::structure::{SheetNameFault, StructureError};
use crate::{PartWriter, WriteError};
use std::io::Write;

/// A content for a sheet added.
struct Content;

impl PartWriter for Content {
    fn write(&mut self, out: &mut dyn Write) -> Result<(), WriteError> {
        Ok(out.write_all(b"<worksheet/>")?)
    }
}

#[test]
fn a_sheet_is_added_last_or_at_a_position_with_the_next_sheet_id_and_key() {
    let mut edit = open(&["A", "B"], "");
    edit.add_sheet("C", None, None).unwrap();
    edit.add_sheet("D", Some(0), Some(Box::new(Content)))
        .unwrap();
    edit.add_sheet("E", Some(2), None).unwrap();
    assert_eq!(sheet_names(&edit), ["D", "A", "E", "B", "C"]);
    let got: Vec<_> = (edit.state.sheets.iter())
        .map(|sheet| (sheet.key, sheet.sheet_id, sheet.state, sheet.renamed))
        .collect();
    let visible = SheetState::Visible;
    assert_eq!(
        got,
        [
            (3, 4, visible, false),
            (0, 1, visible, false),
            (4, 5, visible, false),
            (1, 2, visible, false),
            (2, 3, visible, false),
        ]
    );
    let added: Vec<_> = (edit.state.sheets.iter())
        .filter_map(|sheet| match sheet.origin {
            Origin::Added { index } => Some((sheet.name.as_str(), index)),
            Origin::Original { .. } => None,
        })
        .collect();
    assert_eq!(added, [("D", 1), ("E", 2), ("C", 0)]);
    assert_eq!(
        (edit.state.added.iter())
            .map(Option::is_some)
            .collect::<Vec<_>>(),
        [false, true, false]
    );
    assert_eq!(edit.state.next_sheet_id, Some(6));
}

#[test]
fn the_original_sheets_keep_their_part_and_relationship() {
    let edit = open(&["A", "B"], "");
    let origins: Vec<_> = (edit.state.sheets.iter())
        .map(|sheet| sheet.origin.clone())
        .collect();
    assert!(matches!(
        &origins[1],
        Origin::Original { relationship_id, .. } if relationship_id == "rId2"
    ));
    assert_eq!(edit.state.next_key, 2);
    assert_eq!(edit.snapshot.workbook.sheets().len(), 2);
}

#[test]
fn a_sheet_id_is_the_largest_plus_one_and_none_is_left_past_the_largest_id() {
    let mut edit = open_with(
        "",
        &[("A", r#"sheetId="7""#), ("B", r#"sheetId="4294967294""#)],
        "",
    );
    edit.add_sheet("C", None, None).unwrap();
    assert_eq!(edit.state.sheets[2].sheet_id, u32::MAX);
    assert_eq!(edit.state.next_sheet_id, None);
    let before = fingerprint(&edit);
    assert_eq!(
        edit.add_sheet("D", None, None),
        Err(StructureError::SheetIdExhausted)
    );
    assert_eq!(fingerprint(&edit), before);
    let mut edit = open_with("", &[("A", r#"sheetId="4294967295""#)], "");
    assert_eq!(
        edit.add_sheet("B", None, None),
        Err(StructureError::SheetIdExhausted)
    );
    // A delete does not give its id back.
    let mut edit = open(&["A", "B"], "");
    edit.delete_sheet("B").unwrap();
    edit.add_sheet("C", None, None).unwrap();
    assert_eq!(edit.state.sheets[1].sheet_id, 3);
}

#[test]
fn an_add_that_fails_changes_nothing() {
    let mut edit = open(&["A", "B"], "");
    let before = fingerprint(&edit);
    assert_eq!(
        edit.add_sheet("", None, None),
        Err(StructureError::SheetNameInvalid {
            name: String::new(),
            reason: SheetNameFault::Empty
        })
    );
    assert_eq!(
        edit.add_sheet("b", Some(0), None),
        Err(StructureError::SheetNameTaken {
            name: "b".into(),
            other: "B".into()
        })
    );
    assert_eq!(
        edit.add_sheet("C", Some(3), Some(Box::new(Content))),
        Err(StructureError::PositionOutOfRange { at: 3, sheets: 2 })
    );
    assert_eq!(fingerprint(&edit), before);
    // The position after the last sheet is allowed.
    edit.add_sheet("C", Some(2), None).unwrap();
}

#[test]
fn a_sheet_is_renamed_found_by_its_name_letter_case_aside() {
    let mut edit = open(&["A", "B"], "");
    edit.rename_sheet("b", "Beta").unwrap();
    assert_eq!(sheet_names(&edit), ["A", "Beta"]);
    let flags: Vec<_> = edit
        .state
        .sheets
        .iter()
        .map(|sheet| sheet.renamed)
        .collect();
    assert_eq!(flags, [false, true]);
    // A letter case of its own name is not a name another sheet has.
    edit.rename_sheet("Beta", "BETA").unwrap();
    assert_eq!(sheet_names(&edit), ["A", "BETA"]);
}

#[test]
fn a_rename_that_fails_changes_nothing() {
    let mut edit = open(&["A", "B"], &defined(&[("N", "", "A!$A$1")]));
    let before = fingerprint(&edit);
    assert_eq!(
        edit.rename_sheet("Z", "C"),
        Err(StructureError::SheetMissing { name: "Z".into() })
    );
    assert_eq!(
        edit.rename_sheet("A", "b"),
        Err(StructureError::SheetNameTaken {
            name: "b".into(),
            other: "B".into()
        })
    );
    assert_eq!(
        edit.rename_sheet("A", "a:b"),
        Err(StructureError::SheetNameInvalid {
            name: "a:b".into(),
            reason: SheetNameFault::Character
        })
    );
    assert_eq!(fingerprint(&edit), before);
}

#[test]
fn a_sheet_is_deleted_with_its_names_and_its_part_is_kept_for_apply() {
    let names = defined(&[
        ("Book", "", "A!$A$1"),
        ("OfA", "0", "A!$A$1"),
        ("OfB", "1", "B!$A$1"),
        ("Lost", "7", "B!$A$1"),
    ]);
    let mut edit = open(&["A", "B", "C"], &names);
    edit.state.names.push(super::super::state::NameEntry {
        written: "Added".into(),
        scope: Scope::Sheet(0),
        text: super::super::state::Text::Changed("C!A1".into()),
        origin: None,
        name_changed: false,
        unresolved: None,
    });
    edit.delete_sheet("a").unwrap();
    assert_eq!(sheet_names(&edit), ["B", "C"]);
    assert_eq!(edit.state.deleted.len(), 1);
    assert_eq!(edit.state.deleted[0].name, "A");
    // OfA and the added name are removed, the one name of the workbook, the one of B
    // and the one of a sheet number the workbook does not have stay.
    let kept: Vec<_> = names_of(&edit)
        .into_iter()
        .map(|(name, scope, _)| (name, scope))
        .collect();
    assert_eq!(
        kept,
        [
            ("Book".to_owned(), Scope::Book),
            ("OfB".to_owned(), Scope::Sheet(1)),
            ("Lost".to_owned(), Scope::Raw(7)),
        ]
    );
    assert_eq!(
        edit.state.removed_names.iter().copied().collect::<Vec<_>>(),
        [1]
    );
    // The sheet B is deleted next: its name is removed, the sheet C's number is
    // still the sheet's key.
    edit.delete_sheet("B").unwrap();
    assert_eq!(
        edit.state.removed_names.iter().copied().collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(edit.state.deleted.len(), 2);
}

#[test]
fn a_delete_that_fails_changes_nothing() {
    let mut edit = open_with("", &[("A", ""), ("B", r#"state="hidden""#)], "");
    let before = fingerprint(&edit);
    assert_eq!(
        edit.delete_sheet("Z"),
        Err(StructureError::SheetMissing { name: "Z".into() })
    );
    assert_eq!(
        edit.delete_sheet("a"),
        Err(StructureError::LastVisibleSheet { name: "A".into() })
    );
    assert_eq!(fingerprint(&edit), before);
    // A hidden sheet may be deleted while another one is visible.
    edit.delete_sheet("B").unwrap();
    let before = fingerprint(&edit);
    assert_eq!(
        edit.delete_sheet("A"),
        Err(StructureError::LastSheet { name: "A".into() })
    );
    assert_eq!(fingerprint(&edit), before);
}

#[test]
fn a_sheet_is_moved_and_its_scoped_names_keep_its_key_as_their_scope() {
    let names = defined(&[("OfB", "1", "B!$A$1")]);
    let mut edit = open(&["A", "B", "C"], &names);
    edit.move_sheet("c", 0).unwrap();
    assert_eq!(sheet_names(&edit), ["C", "A", "B"]);
    edit.move_sheet("A", 2).unwrap();
    assert_eq!(sheet_names(&edit), ["C", "B", "A"]);
    assert_eq!(names_of(&edit)[0].1, Scope::Sheet(1));
    // The name is still removed with the sheet B when it is deleted.
    edit.delete_sheet("B").unwrap();
    assert!(names_of(&edit).is_empty());
    assert_eq!(edit.state.removed_names.len(), 1);
}

#[test]
fn a_move_that_fails_changes_nothing() {
    let mut edit = open(&["A", "B"], "");
    let before = fingerprint(&edit);
    assert_eq!(
        edit.move_sheet("Z", 0),
        Err(StructureError::SheetMissing { name: "Z".into() })
    );
    assert_eq!(
        edit.move_sheet("A", 2),
        Err(StructureError::PositionOutOfRange { at: 2, sheets: 2 })
    );
    assert_eq!(fingerprint(&edit), before);
    edit.move_sheet("A", 1).unwrap();
    assert_eq!(sheet_names(&edit), ["B", "A"]);
}

//! `Change`, `Deleted` and `SheetIds`: where each name stood when a sheet
//! was deleted (the four sides, and the 2 bits each name takes), the
//! numbers of the names, and what a delete writes for a 3-D reference whose
//! ends are the deleted sheet, a sheet next to it and a sheet spelled
//! another way.

use super::parts::WORKSHEET;
use super::{NS, applied_with_report, defined, formulas, open, package};
use crate::structure::edit::StructureEdit;
use crate::structure::edit::change::{Change, Deleted, SheetIds, Side};
use crate::structure::edit::state::SheetEntry;

/// The `Deleted` of the delete that is the `index`th change of `edit`.
fn deleted_of(edit: &StructureEdit<'_>, index: usize) -> Deleted {
    match &edit.state.changes[index] {
        Change::Delete(deleted) => deleted.clone(),
        other @ Change::Rename { .. } => panic!("not a delete: {other:?}"),
    }
}

#[test]
fn a_delete_keeps_the_side_of_each_name_and_a_name_numbered_later_is_absent() {
    let mut edit = open(&["Alpha", "Beta", "Gamma"], "");
    edit.delete_sheet("beta").unwrap();
    edit.rename_sheet("Gamma", "Fresh").unwrap();
    let (deleted, ids) = (deleted_of(&edit, 0), &edit.state.sheet_ids);
    assert_eq!(&*deleted.name, "Beta");
    let side = |name: &str| deleted.side(ids, name);
    assert_eq!(side("ALPHA"), Side::Before);
    assert_eq!(side("Beta"), Side::Deleted);
    assert_eq!(side("gamma"), Side::After);
    // A name no sheet had, and a name numbered after the delete.
    assert_eq!(side("Zed"), Side::Absent);
    assert_eq!(side("Fresh"), Side::Absent);
    assert_eq!(deleted.prev, ids.get("alpha"));
    assert_eq!(deleted.next, ids.get("gamma"));
}

#[test]
fn a_delete_next_to_a_sheet_with_a_long_name_holds_the_number_of_the_name_and_not_its_text() {
    // The first sheet's name is 100,000 bytes and the sheet before each
    // deleted one. A `Deleted` has exactly four fields; the sheets beside
    // the deleted one are numbers, and the text it holds is the name of the
    // deleted sheet and one byte for each four names.
    let long = "L".repeat(100_000);
    let shorts: Vec<String> = (0..20).map(|n| format!("S{n}")).collect();
    let mut names: Vec<&str> = vec![long.as_str()];
    names.extend(shorts.iter().map(String::as_str));
    let mut edit = open(&names, "");
    for short in &shorts {
        edit.delete_sheet(short).unwrap();
    }
    let long_id = edit.state.sheet_ids.get(&long);
    for index in 0..shorts.len() {
        let Deleted {
            name,
            sides,
            prev,
            next,
        } = deleted_of(&edit, index);
        let beside: (Option<u32>, Option<u32>) = (prev, next);
        assert_eq!(beside.0, long_id);
        // The sheet after `S{index}` is `S{index + 1}`; the last has none.
        let after = shorts
            .get(index + 1)
            .and_then(|n| edit.state.sheet_ids.get(n));
        assert_eq!(beside.1, after, "{index}");
        assert!(name.len() + sides.len() < 16, "{index}");
    }
}

#[test]
fn a_delete_of_the_first_or_the_last_sheet_has_no_sheet_before_or_after() {
    let mut edit = open(&["Alpha", "Beta", "Gamma"], "");
    edit.delete_sheet("Alpha").unwrap();
    let first = deleted_of(&edit, 0);
    assert_eq!(first.prev, None);
    assert_eq!(first.next, edit.state.sheet_ids.get("beta"));
    edit.delete_sheet("Gamma").unwrap();
    let last = deleted_of(&edit, 1);
    assert_eq!(last.prev, edit.state.sheet_ids.get("beta"));
    assert_eq!(last.next, None);
}

#[test]
fn a_delete_takes_a_byte_for_each_four_names_whatever_the_deletes_before_it() {
    // 4 names: 1 byte; 5 names: 2 bytes; 9 names: 3 bytes.
    for (names, bytes) in [(4_usize, 1), (5, 2), (8, 2), (9, 3)] {
        let sheets: Vec<String> = (0..names).map(|n| format!("S{n}")).collect();
        let borrowed: Vec<&str> = sheets.iter().map(String::as_str).collect();
        let mut edit = open(&borrowed, "");
        edit.delete_sheet("S0").unwrap();
        assert_eq!(deleted_of(&edit, 0).sides.len(), bytes, "{names}");
        // A later delete takes as many bytes as there are names then; the
        // first is as it was.
        edit.rename_sheet("S1", "Fresh").unwrap();
        edit.delete_sheet("S2").unwrap();
        assert_eq!(deleted_of(&edit, 2).sides.len(), (names + 1).div_ceil(4));
        assert_eq!(deleted_of(&edit, 0).sides.len(), bytes, "{names}");
    }
}

#[test]
fn a_name_that_a_sheet_has_is_numbered_once_and_spelled_as_the_latest_sheet_spells_it() {
    let mut ids = SheetIds::default();
    let (alpha, beta) = (ids.id("Alpha"), ids.id("Beta"));
    assert_ne!(alpha, beta);
    assert_eq!(ids.id("ALPHA"), alpha);
    assert_eq!(ids.len(), 2);
    assert_eq!((ids.spelling(alpha), ids.spelling(beta)), ("ALPHA", "Beta"));
    assert_eq!(
        (ids.get("alpha"), ids.get("BETA"), ids.get("Gamma")),
        (Some(alpha), Some(beta), None)
    );
    assert_eq!(ids.spelling(99), "");
}

#[test]
fn the_first_of_two_sheets_with_one_name_stands_for_the_name() {
    let edit = open(&["Alpha", "Beta", "Gamma"], "");
    let mut sheets: Vec<SheetEntry> = edit.state.sheets.clone();
    // The third sheet is called `ALPHA`, as the first is.
    sheets[2].name = "ALPHA".to_owned();
    sheets[2].name_id = sheets[0].name_id;
    let ids = edit.state.sheet_ids.clone();
    let deleted = Deleted::new(&sheets, 1, &ids);
    assert_eq!(deleted.side(&ids, "alpha"), Side::Before);
    assert_eq!(deleted.side(&ids, "Gamma"), Side::Absent);
    assert_eq!(deleted.next, Some(sheets[2].name_id));
}

/// The edit of `sheets` and one defined name for each of `texts`.
fn with(sheets: &[&str], texts: &[&str]) -> StructureEdit<'static> {
    let names: Vec<(String, &str, &str)> = (texts.iter().enumerate())
        .map(|(index, text)| (format!("n{index}"), "", *text))
        .collect();
    let borrowed: Vec<(&str, &str, &str)> = (names.iter())
        .map(|(name, scope, text)| (name.as_str(), *scope, *text))
        .collect();
    open(sheets, &defined(&borrowed))
}

#[test]
fn a_delete_writes_ref_for_a_3d_reference_whose_both_ends_are_the_sheet_whatever_the_spelling() {
    let mut edit = with(&["Alpha", "Beta"], &["Alpha:ALPHA!X1", "alpha:Alpha!X1"]);
    edit.delete_sheet("Alpha").unwrap();
    assert_eq!(formulas(&edit), ["#REF!", "#REF!"]);
    assert!(edit.state.ref_written);
}

#[test]
fn a_delete_leaves_a_3d_reference_alone_when_neither_end_is_the_sheet_or_an_end_is_no_sheet() {
    let mut edit = with(
        &["Alpha", "Beta", "Gamma", "Delta"],
        &[
            "Beta:Beta!X1",
            "Beta:Delta!X1",
            "Alpha:Zed!X1",
            "Zed:Alpha!X1",
        ],
    );
    edit.delete_sheet("Gamma").unwrap();
    assert_eq!(
        formulas(&edit),
        [
            "Beta:Beta!X1",
            "Beta:Delta!X1",
            "Alpha:Zed!X1",
            "Zed:Alpha!X1"
        ]
    );
    assert!(!edit.state.ref_written);
}

#[test]
fn a_delete_moves_the_deleted_end_next_to_it_and_keeps_the_direction() {
    let sheets = ["Alpha", "Beta", "Gamma", "Delta"];
    // Deleting Beta: as the first end it moves to Gamma (the next sheet),
    // as the last end to Alpha (the one before it) and, written the other
    // way round, the same.
    let mut edit = with(
        &sheets,
        &[
            "Beta:Delta!X1",
            "Alpha:Beta!X1",
            "Delta:Beta!X1",
            "Beta:Alpha!X1",
            "Beta:Gamma!X1",
        ],
    );
    edit.delete_sheet("Beta").unwrap();
    assert_eq!(
        formulas(&edit),
        [
            "Gamma:Delta!X1",
            "Alpha!X1",
            "Delta:Gamma!X1",
            "Alpha!X1",
            "Gamma!X1"
        ]
    );
}

#[test]
fn the_end_that_stays_is_spelled_as_the_workbook_spells_the_sheet() {
    let sheets = ["Alpha", "Beta", "Gamma", "Delta"];
    let mut edit = with(
        &sheets,
        &["Alpha:delta!X1", "dELTA:ALPHA!X1", "alpha:GAMMA!X1"],
    );
    edit.delete_sheet("Alpha").unwrap();
    assert_eq!(
        formulas(&edit),
        ["Beta:Delta!X1", "Delta:Beta!X1", "Beta:Gamma!X1"]
    );
    // A sheet spelled another way by a rename that changes only the case.
    let mut edit = with(&sheets, &["Alpha:delta!X1"]);
    edit.rename_sheet("Delta", "DELTA").unwrap();
    assert_eq!(formulas(&edit), ["Alpha:DELTA!X1"]);
    edit.delete_sheet("Alpha").unwrap();
    assert_eq!(formulas(&edit), ["Beta:DELTA!X1"]);
}

#[test]
fn a_name_spelled_another_way_after_a_delete_gives_the_same_prefix_in_the_parts_and_the_names() {
    // `Alpha` is deleted, then `Delta` is renamed `Zulu` and `Gamma` is
    // renamed `DELTA`, the name `delta` that `Alpha:delta!` ends with. The
    // formula of the worksheet (rewritten by `apply`, with the names as the
    // renames leave them) and the defined name (rewritten by each call)
    // end with the same prefix.
    let sheet =
        format!(r#"<worksheet xmlns="{NS}"><c r="A1"><f>Alpha:delta!A1</f></c></worksheet>"#);
    let sheets = [("Alpha", ""), ("Beta", ""), ("Gamma", ""), ("Delta", "")];
    let after = defined(&[("n0", "", "Alpha:delta!X1")]);
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (output, _) = applied_with_report(package("", &sheets, &after, &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
        edit.rename_sheet("Delta", "Zulu").unwrap();
        edit.rename_sheet("Gamma", "DELTA").unwrap();
    });
    let worksheet = output.part("/xl/worksheets/sheet2.xml").unwrap();
    let workbook = output.part("/xl/workbook.xml").unwrap();
    assert!(worksheet.contains("<f>Beta:Zulu!A1</f>"), "{worksheet}");
    assert!(workbook.contains(">Beta:Zulu!X1<"), "{workbook}");
}

#[test]
fn a_neighbour_spelled_another_way_after_a_delete_gives_the_same_prefix_in_the_parts_and_the_names()
{
    // `Alpha` is deleted, `Beta` (the sheet beside it) is renamed `Zulu`, and
    // `Delta` is renamed `BETA`, the name `beta` that `Alpha:Gamma!` moved to.
    // The formula of the worksheet and the defined name end with the same
    // prefix.
    let sheet =
        format!(r#"<worksheet xmlns="{NS}"><c r="A1"><f>Alpha:Gamma!A1</f></c></worksheet>"#);
    let sheets = [("Alpha", ""), ("Beta", ""), ("Gamma", ""), ("Delta", "")];
    let after = defined(&[("n0", "", "Alpha:Gamma!X1")]);
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (output, _) = applied_with_report(package("", &sheets, &after, &extra), |edit| {
        edit.delete_sheet("Alpha").unwrap();
        edit.rename_sheet("Beta", "Zulu").unwrap();
        edit.rename_sheet("Delta", "BETA").unwrap();
    });
    let worksheet = output.part("/xl/worksheets/sheet2.xml").unwrap();
    let workbook = output.part("/xl/workbook.xml").unwrap();
    assert!(worksheet.contains("<f>Zulu:Gamma!A1</f>"), "{worksheet}");
    assert!(workbook.contains(">Zulu:Gamma!X1<"), "{workbook}");
}

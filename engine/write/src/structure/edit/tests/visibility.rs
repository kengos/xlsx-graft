//! `set_sheet_state` and `set_active_sheet`: the state each leaves, the
//! errors, that a call that fails leaves the state as it was, and the
//! workbook part `apply` writes for them (`state` of a `sheet`, the
//! `bookViews` of a workbook that has none).

use super::rewrite::{rels, run, sheets, workbook};
use super::{NS, R, applied, fingerprint, open_with, package_raw};
use crate::structure::StructureError;
use crate::structure::edit::StructureEdit;
use crate::structure::edit::state::Pointer;
use crate::structure::model::SheetState;

const HIDDEN: &str = r#"state="hidden""#;

/// The visibility of each sheet, in order.
fn states(edit: &StructureEdit<'_>) -> Vec<SheetState> {
    edit.state.sheets.iter().map(|sheet| sheet.state).collect()
}

/// The `activeTab` and `firstSheet` pointers of each view.
fn views(edit: &StructureEdit<'_>) -> Vec<(Pointer, Pointer)> {
    (edit.state.views.iter())
        .map(|view| (view.active, view.first))
        .collect()
}

#[test]
fn a_sheet_is_hidden_very_hidden_and_shown_and_a_state_it_has_changes_nothing() {
    use SheetState::{Hidden, VeryHidden, Visible};
    let mut edit = open_with("", &[("A", ""), ("B", ""), ("C", "")], "");
    edit.set_sheet_state("b", Hidden).unwrap();
    assert_eq!(states(&edit), [Visible, Hidden, Visible]);
    edit.set_sheet_state("B", VeryHidden).unwrap();
    edit.set_sheet_state("C", VeryHidden).unwrap();
    assert_eq!(states(&edit), [Visible, VeryHidden, VeryHidden]);
    edit.set_sheet_state("b", Visible).unwrap();
    assert_eq!(states(&edit), [Visible, Visible, VeryHidden]);
    // The state a sheet has: no change, even for the only visible sheet.
    let before = fingerprint(&edit);
    edit.set_sheet_state("C", VeryHidden).unwrap();
    edit.set_sheet_state("A", Visible).unwrap();
    assert_eq!(fingerprint(&edit), before);
    let mut only = open_with("", &[("A", ""), ("B", HIDDEN)], "");
    let before = fingerprint(&only);
    only.set_sheet_state("A", SheetState::Visible).unwrap();
    assert_eq!(fingerprint(&only), before);
}

#[test]
fn the_only_visible_sheet_is_not_hidden_and_a_sheet_that_is_missing_is_refused() {
    let mut edit = open_with("", &[("A", ""), ("B", HIDDEN)], "");
    let before = fingerprint(&edit);
    for state in [SheetState::Hidden, SheetState::VeryHidden] {
        let error = edit.set_sheet_state("a", state).unwrap_err();
        assert_eq!(error, StructureError::LastVisibleSheet { name: "A".into() });
    }
    let error = edit.set_sheet_state("C", SheetState::Hidden).unwrap_err();
    assert_eq!(error, StructureError::SheetMissing { name: "C".into() });
    assert_eq!(fingerprint(&edit), before);
    // A hidden sheet can be hidden another way.
    edit.set_sheet_state("B", SheetState::VeryHidden).unwrap();
    assert_eq!(states(&edit), [SheetState::Visible, SheetState::VeryHidden]);
}

#[test]
fn a_view_that_points_at_a_sheet_hidden_points_at_the_next_visible_sheet_else_the_one_before() {
    use Pointer::{Key, Raw};
    let books = concat!(
        r#"<bookViews><workbookView activeTab="1" firstSheet="1"/>"#,
        r#"<workbookView activeTab="0" firstSheet="9"/></bookViews>"#,
    );
    let sheets = [("A", ""), ("B", ""), ("C", HIDDEN), ("D", "")];
    let mut edit = open_with(books, &sheets, "");
    // C is hidden, so D is the next visible sheet after B.
    edit.set_sheet_state("B", SheetState::Hidden).unwrap();
    assert_eq!(views(&edit), [(Key(3), Key(3)), (Key(0), Raw)]);
    // Nothing visible is after D: A is the last visible sheet before it.
    edit.set_sheet_state("D", SheetState::VeryHidden).unwrap();
    assert_eq!(views(&edit), [(Key(0), Key(0)), (Key(0), Raw)]);
    // A sheet that is hidden again or shown moves no view.
    edit.set_sheet_state("D", SheetState::Hidden).unwrap();
    edit.set_sheet_state("B", SheetState::Visible).unwrap();
    assert_eq!(views(&edit), [(Key(0), Key(0)), (Key(0), Raw)]);
}

#[test]
fn a_sheet_is_made_the_active_one_of_every_view_and_the_first_sheet_is_kept_or_moved_to_it() {
    use Pointer::{Key, Raw};
    let books = concat!(
        r#"<bookViews><workbookView activeTab="0" firstSheet="2"/>"#,
        r#"<workbookView activeTab="3" firstSheet="0"/>"#,
        r#"<workbookView activeTab="9" firstSheet="9"/></bookViews>"#,
    );
    let sheets = [("A", ""), ("B", ""), ("C", ""), ("D", "")];
    let mut edit = open_with(books, &sheets, "");
    edit.set_active_sheet("c").unwrap();
    // The first sheet C or before it stays; one that is not a sheet of the
    // workbook (a position past them) is the active sheet.
    assert_eq!(
        views(&edit),
        [(Key(2), Key(2)), (Key(2), Key(0)), (Key(2), Key(2))]
    );
    // A first sheet after the active sheet is the active sheet.
    edit.set_active_sheet("B").unwrap();
    assert_eq!(
        views(&edit),
        [(Key(1), Key(1)), (Key(1), Key(0)), (Key(1), Key(1))]
    );
    let books = r#"<bookViews><workbookView activeTab="9" firstSheet="9"/></bookViews>"#;
    let mut edit = open_with(books, &sheets, "");
    assert_eq!(views(&edit), [(Raw, Raw)]);
    edit.set_active_sheet("D").unwrap();
    assert_eq!(views(&edit), [(Key(3), Key(3))]);
}

#[test]
fn a_sheet_that_is_hidden_or_missing_is_not_made_the_active_one() {
    let sheets = [("A", ""), ("B", HIDDEN), ("C", r#"state="veryHidden""#)];
    let mut edit = open_with("", &sheets, "");
    let before = fingerprint(&edit);
    for name in ["B", "c"] {
        let error = edit.set_active_sheet(name).unwrap_err();
        let name = name.to_uppercase();
        assert_eq!(error, StructureError::SheetHidden { name });
    }
    let error = edit.set_active_sheet("D").unwrap_err();
    assert_eq!(error, StructureError::SheetMissing { name: "D".into() });
    assert_eq!(fingerprint(&edit), before);
    assert_eq!(
        StructureError::SheetHidden { name: "B".into() }.to_string(),
        r#"the sheet "B" is hidden, so it cannot be the active sheet"#
    );
}

#[test]
fn a_workbook_with_no_view_gets_one_that_names_the_active_sheet_and_the_first_visible_one() {
    use Pointer::Key;
    let sheets = [("A", HIDDEN), ("B", ""), ("C", "")];
    for books in ["", "<bookViews/>"] {
        let mut edit = open_with(books, &sheets, "");
        assert!(views(&edit).is_empty());
        edit.set_active_sheet("C").unwrap();
        assert_eq!(views(&edit), [(Key(2), Key(1))]);
        // The next call changes that view and adds no other.
        edit.set_active_sheet("B").unwrap();
        assert_eq!(views(&edit), [(Key(1), Key(1))]);
    }
}

#[test]
fn the_active_sheet_is_written_with_the_views_it_changed_and_an_added_view_is_written_when_it_is_not_the_first()
 {
    let views = |list: &str| format!("<bookViews>{list}</bookViews>");
    let gamma = |edit: &mut StructureEdit<'static>| edit.set_active_sheet("gamma").unwrap();
    let body = format!("{}{}", views(r#"<workbookView activeTab="0"/>"#), sheets(3));
    let got = run(&body, 3, gamma);
    let expected = body.replace(r#"activeTab="0""#, r#"activeTab="2""#);
    assert_eq!(got, workbook(&expected));
    // The first sheet after the active sheet moves to it.
    let body = format!(
        "{}{}",
        views(r#"<workbookView firstSheet="2"/>"#),
        sheets(3)
    );
    let got = run(&body, 3, |edit| edit.set_active_sheet("Beta").unwrap());
    let expected = views(r#"<workbookView firstSheet="1" activeTab="1"/>"#);
    assert_eq!(got, workbook(&format!("{expected}{}", sheets(3))));
}

#[test]
fn a_view_is_added_before_sheets_when_the_active_sheet_is_not_the_first_and_there_is_none() {
    // No bookViews: it is written before `sheets`, after what precedes it.
    let got = run(&format!("<workbookPr/>{}", sheets(3)), 3, |edit| {
        edit.set_active_sheet("Gamma").unwrap();
    });
    let view = r#"<bookViews><workbookView activeTab="2"/></bookViews>"#;
    let expected = format!("<workbookPr/>{view}{}", sheets(3));
    assert_eq!(got, workbook(&expected));
    // The first visible sheet is written as the first sheet when it is not
    // the first.
    let hid = r#"<sheets><sheet name="Alpha" sheetId="1" state="hidden" r:id="rId1"/>"#;
    let body = format!(
        r#"{hid}<sheet name="Beta" sheetId="2" r:id="rId2"/><sheet name="Gamma" sheetId="3" r:id="rId3"/></sheets>"#
    );
    let got = run(&body, 3, |edit| edit.set_active_sheet("Gamma").unwrap());
    let view = r#"<bookViews><workbookView activeTab="2" firstSheet="1"/></bookViews>"#;
    assert_eq!(got, workbook(&format!("{view}{body}")));
    // An empty `bookViews`, as an empty element or with a start tag, holds it.
    let view = r#"<bookViews><workbookView activeTab="2"/></bookViews>"#;
    for empty in ["<bookViews/>", "<bookViews></bookViews>"] {
        let got = run(&format!("{empty}{}", sheets(3)), 3, |edit| {
            edit.set_active_sheet("Gamma").unwrap();
        });
        assert_eq!(got, workbook(&format!("{view}{}", sheets(3))));
    }
    // The root has a prefix.
    let body = concat!(
        r#"<x:sheets><x:sheet name="Alpha" sheetId="1" r:id="rId1"/>"#,
        r#"<x:sheet name="Beta" sheetId="2" r:id="rId2"/></x:sheets>"#,
    );
    let root = format!(r#"<x:workbook xmlns:x="{NS}" xmlns:r="{R}">"#);
    let package = package_raw(&format!("{root}{body}</x:workbook>"), &rels(2), 2, &[], "");
    let output = applied(package, |edit| edit.set_active_sheet("Beta").unwrap());
    let view = r#"<x:bookViews><x:workbookView activeTab="1"/></x:bookViews>"#;
    let expected = format!("{root}{view}{body}</x:workbook>");
    assert_eq!(output.part("/xl/workbook.xml").unwrap(), expected);
}

#[test]
fn the_first_sheet_as_the_active_sheet_adds_no_view_and_leaves_the_part_as_it_is() {
    let text = workbook(&sheets(2));
    let package = || package_raw(&text, &rels(2), 2, &[], "");
    let output = applied(package(), |edit| edit.set_active_sheet("alpha").unwrap());
    assert!(output.holds(&text));
    // A view that is there and already names the sheet is not written again.
    let books = format!(
        r#"<bookViews><workbookView activeTab="1"/></bookViews>{}"#,
        sheets(2)
    );
    let text = workbook(&books);
    let package = package_raw(&text, &rels(2), 2, &[], "");
    let output = applied(package, |edit| edit.set_active_sheet("Beta").unwrap());
    assert!(output.holds(&text));
}

#[test]
fn the_state_of_a_sheet_is_written_in_the_workbook_part() {
    let body = r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/><sheet name="Beta" sheetId="2" state="hidden" r:id="rId2"/><sheet name="Gamma" sheetId="3" r:id="rId3"/></sheets>"#;
    let got = run(body, 3, |edit| {
        edit.set_sheet_state("Alpha", SheetState::VeryHidden)
            .unwrap();
        edit.set_sheet_state("Beta", SheetState::Visible).unwrap();
        edit.set_sheet_state("Gamma", SheetState::Hidden).unwrap();
    });
    // Alpha was the active sheet (no view): the view added names Beta.
    let expected = concat!(
        r#"<bookViews><workbookView activeTab="1" firstSheet="1"/></bookViews>"#,
        r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1" state="veryHidden"/>"#,
        r#"<sheet name="Beta" sheetId="2" r:id="rId2"/>"#,
        r#"<sheet name="Gamma" sheetId="3" r:id="rId3" state="hidden"/></sheets>"#,
    );
    assert_eq!(got, workbook(expected));
    // A sheet renamed and hidden gets both; one added and hidden gets it.
    let got = run(&sheets(1), 1, |edit| {
        edit.add_sheet("New", None, None).unwrap();
        edit.set_sheet_state("New", SheetState::Hidden).unwrap();
        edit.add_sheet("Shown", None, None).unwrap();
        edit.add_sheet("Gone", None, None).unwrap();
        edit.set_sheet_state("Gone", SheetState::VeryHidden)
            .unwrap();
        edit.rename_sheet("Alpha", "Z").unwrap();
        edit.set_sheet_state("Z", SheetState::Hidden).unwrap();
    });
    assert!(
        got.contains(r#"<sheet name="Z" sheetId="1" r:id="rId1" state="hidden"/>"#),
        "{got}"
    );
    assert!(
        got.contains(r#"state="hidden"/><sheet name="Shown" sheetId="3" r:id="rId3"/>"#),
        "{got}"
    );
    assert!(got.contains(r#"state="veryHidden"/>"#), "{got}");
}

#[test]
fn a_state_set_and_set_back_leaves_the_workbook_part_as_it_is() {
    let text = workbook(&sheets(2));
    let package = package_raw(&text, &rels(2), 2, &[], "");
    let output = applied(package, |edit| {
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
        edit.set_sheet_state("Beta", SheetState::Visible).unwrap();
    });
    assert!(output.holds(&text));
}

#[test]
fn hiding_the_first_sheet_of_a_workbook_with_no_view_adds_a_view_on_the_next_visible_sheet() {
    use Pointer::Key;
    let sheets = [("A", ""), ("B", HIDDEN), ("C", "")];
    for books in ["", "<bookViews/>"] {
        let mut edit = open_with(books, &sheets, "");
        // Another sheet hidden: the first sheet, which a missing `activeTab`
        // names, stays active, so no view is added.
        edit.set_sheet_state("C", SheetState::Hidden).unwrap();
        assert!(views(&edit).is_empty());
        edit.set_sheet_state("C", SheetState::Visible).unwrap();
        edit.set_sheet_state("A", SheetState::Hidden).unwrap();
        assert_eq!(views(&edit), [(Key(2), Key(2))]);
        // The view that is there is the one the next call changes.
        edit.set_sheet_state("A", SheetState::Visible).unwrap();
        edit.set_active_sheet("A").unwrap();
        assert_eq!(views(&edit), [(Key(0), Key(0))]);
    }
}

#[test]
fn the_view_added_by_hiding_the_first_sheet_is_written_on_the_next_visible_sheet() {
    let got = run(&sheets(3), 3, |edit| {
        edit.set_sheet_state("Alpha", SheetState::Hidden).unwrap();
    });
    let view = r#"<bookViews><workbookView activeTab="1" firstSheet="1"/></bookViews>"#;
    let list = sheets(3).replacen(r#"r:id="rId1"/>"#, r#"r:id="rId1" state="hidden"/>"#, 1);
    assert_eq!(got, workbook(&format!("{view}{list}")));
    // A sheet that is not the first: the state only.
    let got = run(&sheets(3), 3, |edit| {
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
    });
    assert!(!got.contains("bookViews"), "{got}");
}

#[test]
fn a_view_is_added_in_a_book_views_that_holds_no_view_before_its_other_children() {
    let books = "<bookViews><extLst/></bookViews>";
    let got = run(&format!("{books}{}", sheets(3)), 3, |edit| {
        edit.set_active_sheet("Gamma").unwrap();
    });
    let expected = r#"<bookViews><workbookView activeTab="2"/><extLst/></bookViews>"#;
    assert_eq!(got, workbook(&format!("{expected}{}", sheets(3))));
}

#[test]
fn a_view_is_added_in_a_book_views_that_follows_sheets_and_no_second_one_is_written() {
    for views in ["<bookViews/>", "<bookViews></bookViews>"] {
        let body = format!("{}{views}", sheets(3));
        let got = run(&body, 3, |edit| edit.set_active_sheet("Gamma").unwrap());
        let view = r#"<bookViews><workbookView activeTab="2"/></bookViews>"#;
        assert_eq!(got, workbook(&format!("{}{view}", sheets(3))));
    }
}

#[test]
fn a_sheet_element_whose_state_does_not_change_is_written_as_it_was() {
    let empty = r#"<sheet  name='Alpha' sheetId='1' r:id='rId1' />"#;
    let start = r#"<sheet  name='Gamma' sheetId='3' r:id='rId3'></sheet>"#;
    let body =
        format!(r#"<sheets>{empty}<sheet name="Beta" sheetId="2" r:id="rId2"/>{start}</sheets>"#);
    let got = run(&body, 3, |edit| {
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
    });
    let hidden = r#"<sheet name="Beta" sheetId="2" r:id="rId2" state="hidden"/>"#;
    assert_eq!(
        got,
        workbook(&format!("<sheets>{empty}{hidden}{start}</sheets>"))
    );
}

//! `tabSelected` of the `sheetView`s: which worksheets `apply` writes again
//! when the active sheet changes, a sheet is hidden or a delete points a view
//! at another sheet, and that a part with nothing to change stays as it was.

use super::parts::{WORKSHEET, kept};
use super::rewrite::{rels, sheets, workbook};
use super::{NS, Output, R, applied_with_report, package_raw};
use crate::structure::edit::StructureEdit;
use crate::structure::model::SheetState;
use crate::structure::{KeptReason, StructureReport};
use crate::{PartWriter, WriteError};
use std::io::Write;

const ON: &str = r#"<sheetView workbookViewId="0" tabSelected="1"/>"#;
const OFF: &str = r#"<sheetView workbookViewId="0"/>"#;
const ONE_VIEW: &str = r#"<bookViews><workbookView activeTab="0"/></bookViews>"#;

/// A worksheet of `views` (the children of `sheetViews`) and `data`.
fn worksheet(views: &str, data: &str) -> String {
    format!(
        r#"<worksheet xmlns="{NS}"><sheetViews>{views}</sheetViews><sheetData>{data}</sheetData></worksheet>"#
    )
}

/// The package of the workbook part `body`, one worksheet for each of
/// `texts`, after `act`.
fn tabbed(body: &str, texts: &[String], act: impl FnOnce(&mut StructureEdit<'static>)) -> Output {
    tabbed_with_report(body, texts, act).0
}

/// As [`tabbed`], with the report `apply` returned.
fn tabbed_with_report(
    body: &str,
    texts: &[String],
    act: impl FnOnce(&mut StructureEdit<'static>),
) -> (Output, StructureReport) {
    let names: Vec<String> = (1..=texts.len())
        .map(|n| format!("xl/worksheets/sheet{n}.xml"))
        .collect();
    let extra: Vec<(&str, &str, &str)> = (names.iter().zip(texts))
        .map(|(name, text)| (name.as_str(), WORKSHEET, text.as_str()))
        .collect();
    let package = package_raw(&workbook(body), &rels(texts.len()), texts.len(), &extra, "");
    applied_with_report(package, act)
}

/// The data of the worksheet number `n`: one row, so no two worksheets of a
/// test have the same text, and a test can tell which one was written.
fn row(n: usize) -> String {
    format!(r#"<row r="{n}"/>"#)
}

/// The worksheet number `n` (from 1) whose `sheetView`s are `views`.
fn sheet_n(n: usize, views: &str) -> String {
    worksheet(views, &row(n))
}

/// The worksheets whose `sheetView`s are `views`, each as one string, in
/// the shape of [`sheet_n`].
fn plain(views: &[&str]) -> Vec<String> {
    (views.iter().zip(1..))
        .map(|(views, n)| sheet_n(n, views))
        .collect()
}

fn part(output: &Output, n: usize) -> String {
    output
        .part(&format!("/xl/worksheets/sheet{n}.xml"))
        .unwrap()
}

#[test]
fn the_active_sheet_gets_tab_selected_and_the_sheet_that_was_active_loses_it() {
    let body = format!("{ONE_VIEW}{}", sheets(3));
    let texts = plain(&[ON, OFF, OFF]);
    let output = tabbed(&body, &texts, |edit| {
        edit.set_active_sheet("Beta").unwrap();
    });
    assert_eq!(part(&output, 1), sheet_n(1, OFF));
    assert_eq!(part(&output, 2), sheet_n(2, ON));
    // The third worksheet is not written again.
    assert!(output.holds(&texts[2]));
}

#[test]
fn a_tab_selected_that_is_false_is_set_to_one_and_one_that_is_true_is_left_or_removed() {
    let body = format!("{ONE_VIEW}{}", sheets(3));
    let view = |value: &str| format!(r#"<sheetView workbookViewId="0" tabSelected="{value}"/>"#);
    // Alpha is selected and loses it, Beta gets it: `false` becomes `1`,
    // `true` is left, and `0` on a sheet that loses nothing is left.
    for (before, after) in [("0", ON), ("false", ON), ("1", ON), ("true", &view("true"))] {
        let texts = plain(&[ON, &view(before), &view("0")]);
        let output = tabbed(&body, &texts, |edit| edit.set_active_sheet("Beta").unwrap());
        assert_eq!(part(&output, 2), sheet_n(2, after), "{before}");
        assert_eq!(part(&output, 3), sheet_n(3, &view("0")), "{before}");
    }
    // Alpha's `true` is removed; `0` and `false` are not there to remove.
    let texts = plain(&[&view("true"), OFF, OFF]);
    let output = tabbed(&body, &texts, |edit| edit.set_active_sheet("Beta").unwrap());
    assert_eq!(part(&output, 1), sheet_n(1, OFF));
    let texts = plain(&[&view("0"), OFF, OFF]);
    let output = tabbed(&body, &texts, |edit| edit.set_active_sheet("Beta").unwrap());
    assert!(output.holds(&texts[0]));
}

#[test]
fn a_sheet_view_of_a_view_that_did_not_change_or_with_no_readable_id_is_left_alone() {
    let body = format!("{ONE_VIEW}{}", sheets(2));
    let views = concat!(
        r#"<sheetView workbookViewId="0" tabSelected="1"/>"#,
        r#"<sheetView workbookViewId="5" tabSelected="1"/>"#,
        r#"<sheetView tabSelected="1"/>"#,
        r#"<sheetView workbookViewId="x" tabSelected="1"/>"#,
    );
    let beta = r#"<sheetView workbookViewId="0"/><sheetView workbookViewId="5"/>"#;
    let texts = plain(&[views, beta]);
    let output = tabbed(&body, &texts, |edit| edit.set_active_sheet("Beta").unwrap());
    assert_eq!(
        part(&output, 1),
        sheet_n(1, &views.replacen(r#" tabSelected="1""#, "", 1))
    );
    assert_eq!(
        part(&output, 2),
        sheet_n(2, &beta.replacen("/>", r#" tabSelected="1"/>"#, 1))
    );
}

#[test]
fn each_view_selects_its_active_sheet_and_clears_the_others() {
    let books =
        r#"<bookViews><workbookView activeTab="0"/><workbookView activeTab="1"/></bookViews>"#;
    let body = format!("{books}{}", sheets(3));
    let pair = |a: &str, b: &str| {
        format!(r#"<sheetView workbookViewId="0"{a}/><sheetView workbookViewId="1"{b}/>"#)
    };
    let on = r#" tabSelected="1""#;
    let texts = plain(&[&pair(on, ""), &pair("", on), &pair("", "")]);
    let output = tabbed(&body, &texts, |edit| {
        edit.set_active_sheet("Gamma").unwrap()
    });
    assert_eq!(part(&output, 1), sheet_n(1, &pair("", "")));
    assert_eq!(part(&output, 2), sheet_n(2, &pair("", "")));
    assert_eq!(part(&output, 3), sheet_n(3, &pair(on, on)));
}

#[test]
fn a_sheet_hidden_loses_tab_selected_and_no_other_worksheet_is_rewritten_for_it() {
    // Beta is in a group of selected sheets, and the active sheet stays.
    let body = format!("{ONE_VIEW}{}", sheets(3));
    let texts = plain(&[ON, ON, ON]);
    for state in [SheetState::Hidden, SheetState::VeryHidden] {
        let output = tabbed(&body, &texts, |edit| {
            edit.set_sheet_state("Beta", state).unwrap();
        });
        assert_eq!(part(&output, 2), sheet_n(2, OFF));
        assert!(output.holds(&texts[0]));
        assert!(output.holds(&texts[2]));
    }
}

#[test]
fn a_sheet_hidden_that_was_active_moves_tab_selected_to_the_sheet_that_becomes_active() {
    let body = format!("{ONE_VIEW}{}", sheets(3));
    let texts = plain(&[ON, OFF, OFF]);
    let output = tabbed(&body, &texts, |edit| {
        edit.set_sheet_state("Alpha", SheetState::Hidden).unwrap();
    });
    assert_eq!(part(&output, 1), sheet_n(1, OFF));
    assert_eq!(part(&output, 2), sheet_n(2, ON));
    assert!(output.holds(&texts[2]));
}

#[test]
fn a_delete_that_points_the_view_at_another_sheet_moves_tab_selected_to_it() {
    let body = format!("{ONE_VIEW}{}", sheets(3));
    let texts = plain(&[ON, OFF, OFF]);
    let output = tabbed(&body, &texts, |edit| edit.delete_sheet("Alpha").unwrap());
    assert_eq!(part(&output, 2), sheet_n(2, ON));
    assert!(output.holds(&texts[2]));
    assert!(
        !output
            .names()
            .iter()
            .any(|name| name.ends_with("sheet1.xml"))
    );
}

#[test]
fn an_active_sheet_that_does_not_change_writes_no_worksheet() {
    let body = format!("{ONE_VIEW}{}", sheets(3));
    // Gamma is selected too (a group) and differs from Alpha, so a write of
    // either is told from the other.
    let mut texts = plain(&[ON, OFF]);
    texts.push(worksheet(ON, r#"<row r="1"/>"#));
    let calls: [fn(&mut StructureEdit<'static>); 4] = [
        |edit| edit.set_active_sheet("Alpha").unwrap(),
        |edit| edit.move_sheet("Alpha", 2).unwrap(),
        |edit| edit.set_sheet_state("Beta", SheetState::Visible).unwrap(),
        |edit| edit.add_sheet("New", Some(0), None).unwrap(),
    ];
    for call in calls {
        let output = tabbed(&body, &texts, call);
        for text in &texts {
            assert!(output.holds(text));
        }
    }
}

#[test]
fn a_sheet_that_was_hidden_before_the_edit_and_is_active_is_left_as_it_is() {
    let hidden = r#"<sheet name="Beta" sheetId="2" state="hidden" r:id="rId2"/>"#;
    let list = format!(
        r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/>{hidden}<sheet name="Gamma" sheetId="3" r:id="rId3"/></sheets>"#
    );
    let body = format!(r#"<bookViews><workbookView activeTab="1"/></bookViews>{list}"#);
    let texts = plain(&[ON, ON, ON]);
    let output = tabbed(&body, &texts, |edit| {
        edit.set_sheet_state("Gamma", SheetState::Hidden).unwrap();
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
    });
    // Gamma was visible and is hidden now; Alpha and Beta are as they were.
    assert_eq!(part(&output, 3), sheet_n(3, OFF));
    assert!(output.holds(&texts[0]));
    assert!(output.holds(&texts[1]));
}

/// A content for a sheet added: a worksheet whose view is selected.
struct Selected;

impl PartWriter for Selected {
    fn write(&mut self, out: &mut dyn Write) -> Result<(), WriteError> {
        Ok(out.write_all(worksheet(ON, "").as_bytes())?)
    }
}

#[test]
fn a_sheet_added_is_not_given_or_stripped_of_tab_selected() {
    let body = format!("{ONE_VIEW}{}", sheets(2));
    let texts = plain(&[ON, OFF]);
    let output = tabbed(&body, &texts, |edit| {
        edit.add_sheet("New", None, Some(Box::new(Selected)))
            .unwrap();
        edit.set_active_sheet("Beta").unwrap();
    });
    assert_eq!(part(&output, 2), sheet_n(2, ON));
    assert_eq!(part(&output, 3), worksheet(ON, ""));
    let output = tabbed(&body, &texts, |edit| {
        edit.add_sheet("New", None, None).unwrap();
        edit.set_active_sheet("New").unwrap();
    });
    assert_eq!(part(&output, 1), sheet_n(1, OFF));
    assert!(!part(&output, 3).contains("tabSelected"));
}

#[test]
fn a_worksheet_with_a_rename_and_a_tab_change_is_written_once_with_both() {
    let body = format!("{ONE_VIEW}{}", sheets(2));
    let cell = r#"<row r="1"><c r="A1"><f>Alpha!A1</f></c></row>"#;
    let texts = vec![worksheet(ON, ""), worksheet(OFF, cell)];
    let output = tabbed(&body, &texts, |edit| {
        edit.rename_sheet("Alpha", "New Name").unwrap();
        edit.set_active_sheet("Beta").unwrap();
    });
    let cell = cell.replace("Alpha!", "'New Name'!");
    assert_eq!(part(&output, 2), worksheet(ON, &cell));
    assert_eq!(part(&output, 1), worksheet(OFF, ""));
    // With no tab change, only the formulas are written.
    let output = tabbed(&body, &texts, |edit| {
        edit.rename_sheet("Alpha", "New Name").unwrap();
    });
    assert_eq!(part(&output, 2), worksheet(OFF, &cell));
    assert!(output.holds(&texts[0]));
}

#[test]
fn an_active_tab_past_the_sheets_is_not_a_change_and_no_worksheet_is_written() {
    let body = format!(
        r#"<bookViews><workbookView activeTab="9"/></bookViews>{}"#,
        sheets(2)
    );
    let texts = plain(&[ON, OFF]);
    let output = tabbed(&body, &texts, |edit| {
        edit.rename_sheet("Alpha", "New Name").unwrap();
    });
    assert!(output.holds(&texts[0]));
    assert!(output.holds(&texts[1]));
}

#[test]
fn a_hide_alone_leaves_the_formulas_of_the_sheet_it_writes_and_reports_nothing() {
    let body = format!("{ONE_VIEW}{}", sheets(2));
    let cell =
        r#"<row r="1"><c r="A1"><f>Alpha!A1+Nowhere!B2</f></c><c r="B1"><f>"x</f></c></row>"#;
    let texts = vec![worksheet(ON, ""), worksheet(ON, cell)];
    let (output, report) = tabbed_with_report(&body, &texts, |edit| {
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
    });
    assert_eq!(part(&output, 2), worksheet(OFF, cell));
    assert!(output.holds(&texts[0]));
    assert_eq!(kept(&report), []);
}

#[test]
fn a_sheet_part_that_cannot_be_read_is_reported_when_only_its_tab_changes() {
    let body = format!("{ONE_VIEW}{}", sheets(2));
    let texts = vec![worksheet(ON, ""), "<worksheet><sheetViews>".to_owned()];
    let (output, report) = tabbed_with_report(&body, &texts, |edit| {
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
    });
    let reasons: Vec<_> = kept(&report).iter().map(|k| (k.0, k.4)).collect();
    assert_eq!(
        reasons,
        [("/xl/worksheets/sheet2.xml", KeptReason::Unreadable)]
    );
    assert!(output.holds(&texts[1]));
}

#[test]
fn a_hide_and_a_show_of_the_active_sheet_leave_the_view_and_the_tab_on_the_next_sheet() {
    let body = format!("{ONE_VIEW}{}", sheets(3));
    let texts = plain(&[ON, OFF, OFF]);
    let output = tabbed(&body, &texts, |edit| {
        edit.set_sheet_state("Alpha", SheetState::Hidden).unwrap();
        edit.set_sheet_state("Alpha", SheetState::Visible).unwrap();
    });
    assert_eq!(part(&output, 1), sheet_n(1, OFF));
    assert_eq!(part(&output, 2), sheet_n(2, ON));
    assert!(output.holds(&texts[2]));
    let workbook = output.part("/xl/workbook.xml").unwrap();
    assert!(
        workbook.contains(r#"<workbookView activeTab="1" firstSheet="1"/>"#),
        "{workbook}"
    );
}

#[test]
fn the_active_sheet_of_a_workbook_with_no_view_gets_tab_selected_and_the_first_sheet_loses_it() {
    let body = sheets(3);
    let texts = plain(&[ON, OFF, OFF]);
    let output = tabbed(&body, &texts, |edit| {
        edit.set_active_sheet("Gamma").unwrap()
    });
    assert_eq!(part(&output, 1), sheet_n(1, OFF));
    assert_eq!(part(&output, 3), sheet_n(3, ON));
    assert!(output.holds(&texts[1]));
}

#[test]
fn a_sheet_made_active_and_then_deleted_or_hidden_hands_tab_selected_to_the_sheet_after_it() {
    let body = format!("{ONE_VIEW}{}", sheets(3));
    let texts = plain(&[ON, OFF, OFF]);
    // Deleted, with no sheet after it: the sheet before it, Beta.
    let output = tabbed(&body, &texts, |edit| {
        edit.set_active_sheet("Gamma").unwrap();
        edit.delete_sheet("Gamma").unwrap();
    });
    assert_eq!(part(&output, 1), sheet_n(1, OFF));
    assert_eq!(part(&output, 2), sheet_n(2, ON));
    // Hidden: the next visible sheet, Gamma.
    let output = tabbed(&body, &texts, |edit| {
        edit.set_active_sheet("Beta").unwrap();
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
    });
    assert_eq!(part(&output, 1), sheet_n(1, OFF));
    assert_eq!(part(&output, 2), sheet_n(2, OFF));
    assert_eq!(part(&output, 3), sheet_n(3, ON));
}

#[test]
fn hiding_the_first_sheet_of_a_workbook_with_no_view_moves_tab_selected_to_the_next_sheet() {
    let texts = plain(&[ON, OFF, OFF]);
    let output = tabbed(&sheets(3), &texts, |edit| {
        edit.set_sheet_state("Alpha", SheetState::Hidden).unwrap();
    });
    assert_eq!(part(&output, 1), sheet_n(1, OFF));
    assert_eq!(part(&output, 2), sheet_n(2, ON));
    assert!(output.holds(&texts[2]));
    // Another sheet hidden: the first sheet stays active.
    let output = tabbed(&sheets(3), &texts, |edit| {
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
    });
    assert!(output.holds(&texts[0]));
    assert!(output.holds(&texts[2]));
}

#[test]
fn many_views_select_and_clear_the_tabs_of_every_sheet() {
    let views = 40;
    let books: String = (0..views)
        .map(|_| r#"<workbookView activeTab="0"/>"#)
        .collect();
    let body = format!("<bookViews>{books}</bookViews>{}", sheets(3));
    let each = |on: bool| -> String {
        let tab = if on { r#" tabSelected="1""# } else { "" };
        (0..views)
            .map(|id| format!(r#"<sheetView workbookViewId="{id}"{tab}/>"#))
            .collect()
    };
    let texts = plain(&[&each(true), &each(false), &each(false)]);
    let output = tabbed(&body, &texts, |edit| edit.set_active_sheet("Beta").unwrap());
    assert_eq!(part(&output, 1), sheet_n(1, &each(false)));
    assert_eq!(part(&output, 2), sheet_n(2, &each(true)));
    assert!(output.holds(&texts[2]));
}

const CHARTSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml";

#[test]
fn the_sheet_view_of_a_chartsheet_gets_and_loses_tab_selected() {
    let chart = |view: &str| {
        format!(r#"<chartsheet xmlns="{NS}"><sheetViews>{view}</sheetViews></chartsheet>"#)
    };
    let list = concat!(
        r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/>"#,
        r#"<sheet name="Chart" sheetId="2" r:id="rId2"/></sheets>"#,
    );
    let rels = format!(
        r#"<Relationship Id="rId1" Type="{R}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{R}/chartsheet" Target="chartsheets/sheet2.xml"/>"#
    );
    let run = |active: &str, sheet: &str, chart_text: &str, act: &str| {
        let body = format!(r#"<bookViews><workbookView activeTab="{active}"/></bookViews>{list}"#);
        let extra = [
            ("xl/worksheets/sheet1.xml", WORKSHEET, sheet),
            ("xl/chartsheets/sheet2.xml", CHARTSHEET, chart_text),
        ];
        let package = package_raw(&workbook(&body), &rels, 0, &extra, "");
        let (output, _) = applied_with_report(package, |edit| edit.set_active_sheet(act).unwrap());
        output
    };
    // The chartsheet loses it when the worksheet becomes active.
    let output = run("1", &worksheet(OFF, ""), &chart(ON), "Alpha");
    assert_eq!(
        output.part("/xl/chartsheets/sheet2.xml").unwrap(),
        chart(OFF)
    );
    assert_eq!(part(&output, 1), worksheet(ON, ""));
    // And gets it when it becomes active.
    let output = run("0", &worksheet(ON, ""), &chart(OFF), "Chart");
    assert_eq!(
        output.part("/xl/chartsheets/sheet2.xml").unwrap(),
        chart(ON)
    );
    assert_eq!(part(&output, 1), worksheet(OFF, ""));
}

#[test]
fn a_sheet_part_whose_tab_does_not_change_is_not_read() {
    let body = format!("{ONE_VIEW}{}", sheets(3));
    // Beta is hidden: only its part is read. The broken third part is not
    // reported as unreadable.
    let broken = "<worksheet><sheetViews>".to_owned();
    let texts = vec![worksheet(ON, ""), worksheet(ON, ""), broken];
    let (_, report) = tabbed_with_report(&body, &texts, |edit| {
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
    });
    assert_eq!(kept(&report), []);
}

#[test]
fn the_first_sheet_as_the_active_sheet_of_a_workbook_with_no_view_writes_no_worksheet() {
    // Alpha is the active sheet a missing `activeTab` names, so no tab moves.
    let texts = vec![worksheet(OFF, ""), worksheet(ON, r#"<row r="1"/>"#)];
    let output = tabbed(&sheets(2), &texts, |edit| {
        edit.set_active_sheet("Alpha").unwrap();
    });
    assert!(output.holds(&texts[0]));
    assert!(output.holds(&texts[1]));
}

#[test]
fn a_walk_with_no_rename_or_delete_looks_only_at_sheet_views() {
    // Beta holds a formula and a formula attribute (`controlPr@linkedCell`)
    // past the 64 KiB a walk holds, and a malformed formula that names a
    // sheet. No sheet is renamed or deleted, so none is looked at: nothing is reported and their bytes are written as they were.
    let long = format!("{}1", "1+".repeat(40_000));
    let cells = format!(
        r#"<row r="1"><c r="A1"><f>{long}</f></c><c r="B1"><f>Alpha!"x</f></c></row><controlPr linkedCell="{long}"/>"#
    );
    let body = format!("{ONE_VIEW}{}", sheets(2));
    let texts = vec![worksheet(ON, ""), worksheet(OFF, &cells)];
    let (output, report) = tabbed_with_report(&body, &texts, |edit| {
        edit.set_active_sheet("Beta").unwrap();
    });
    assert_eq!(kept(&report), []);
    assert_eq!(part(&output, 2), worksheet(ON, &cells));
    assert_eq!(part(&output, 1), worksheet(OFF, ""));
}

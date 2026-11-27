//! The bytes each element is written as, with and without a prefix on the
//! element names.

use super::{range, written};
use crate::sheet::{
    ColumnWidth, Columns, FrozenPanes, MergedCells, PasswordHash, RowHeight, SheetProtection,
};

fn merged(ranges: &[&str]) -> MergedCells {
    MergedCells::new(ranges.iter().map(|r| range(r)).collect()).unwrap()
}

#[test]
fn merged_cells_are_a_count_and_a_merge_cell_each_in_the_order_given() {
    let cells = merged(&["D4:E5", "A1:B2"]);
    let out = written(|out| cells.write(out, ""));
    assert_eq!(
        out,
        r#"<mergeCells count="2"><mergeCell ref="D4:E5"/><mergeCell ref="A1:B2"/></mergeCells>"#
    );
    let out = written(|out| merged(&["C3:D4"]).write(out, "x:"));
    assert_eq!(
        out,
        r#"<x:mergeCells count="1"><x:mergeCell ref="C3:D4"/></x:mergeCells>"#
    );
    assert_eq!(written(|out| merged(&[]).write(out, "x:")), "");
}

fn columns(widths: &[(u16, u16, f64)]) -> Columns {
    let widths = widths
        .iter()
        .map(|&(a, b, w)| ColumnWidth::new(a, b, w).unwrap());
    Columns::new(widths.collect()).unwrap()
}

#[test]
fn columns_are_a_col_each_with_the_width_written_as_a_cell_number() {
    let cols = columns(&[(1, 3, 12.5), (5, 5, 0.0), (7, 9, 255.0), (20, 16_384, 1e-8)]);
    let out = written(|out| cols.write(out, ""));
    assert_eq!(
        out,
        concat!(
            r#"<cols><col min="1" max="3" width="12.5" customWidth="1"/>"#,
            r#"<col min="5" max="5" width="0" customWidth="1"/>"#,
            r#"<col min="7" max="9" width="255" customWidth="1"/>"#,
            r#"<col min="20" max="16384" width="1E-8" customWidth="1"/></cols>"#,
        )
    );
    let out = written(|out| columns(&[(2, 2, 9.0)]).write(out, "s:"));
    assert_eq!(
        out,
        r#"<s:cols><s:col min="2" max="2" width="9" customWidth="1"/></s:cols>"#
    );
    assert_eq!(written(|out| columns(&[]).write(out, "")), "");
}

#[test]
fn a_row_height_gives_the_attributes_of_the_row_start_tag() {
    let attributes = RowHeight::new(3, 15.75).unwrap().attributes();
    assert_eq!(
        attributes,
        [("ht", "15.75".to_owned()), ("customHeight", "1".to_owned())]
    );
    let attributes = RowHeight::new(1, 1e-8).unwrap().attributes();
    assert_eq!(attributes[0], ("ht", "1E-8".to_owned()));
    let attributes = RowHeight::new(1, 409.0).unwrap().attributes();
    assert_eq!(attributes[0], ("ht", "409".to_owned()));
}

fn views(rows: u32, columns: u16, prefix: &str) -> String {
    let panes = FrozenPanes::new(rows, columns).unwrap();
    written(|out| panes.write_sheet_views(out, prefix))
}

#[test]
fn frozen_panes_are_a_pane_and_a_selection_at_the_top_left_cell_outside_the_frozen_rows_and_columns()
 {
    assert_eq!(
        views(0, 0, ""),
        r#"<sheetViews><sheetView workbookViewId="0"/></sheetViews>"#
    );
    assert_eq!(
        views(0, 0, "x:"),
        r#"<x:sheetViews><x:sheetView workbookViewId="0"/></x:sheetViews>"#
    );
    assert_eq!(
        views(2, 1, ""),
        concat!(
            r#"<sheetViews><sheetView workbookViewId="0">"#,
            r#"<pane xSplit="1" ySplit="2" topLeftCell="B3" activePane="bottomRight" state="frozen"/>"#,
            r#"<selection pane="bottomRight" activeCell="B3" sqref="B3"/>"#,
            r#"</sheetView></sheetViews>"#,
        )
    );
    assert_eq!(
        views(1, 0, "x:"),
        concat!(
            r#"<x:sheetViews><x:sheetView workbookViewId="0">"#,
            r#"<x:pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/>"#,
            r#"<x:selection pane="bottomLeft" activeCell="A2" sqref="A2"/>"#,
            r#"</x:sheetView></x:sheetViews>"#,
        )
    );
    assert_eq!(
        views(0, 3, ""),
        concat!(
            r#"<sheetViews><sheetView workbookViewId="0">"#,
            r#"<pane xSplit="3" topLeftCell="D1" activePane="topRight" state="frozen"/>"#,
            r#"<selection pane="topRight" activeCell="D1" sqref="D1"/>"#,
            r#"</sheetView></sheetViews>"#,
        )
    );
    assert!(views(1_048_575, 16_383, "").contains(r#"topLeftCell="XFD1048576""#));
}

#[test]
fn the_pane_and_selection_attributes_are_none_without_a_freeze() {
    let none = FrozenPanes::new(0, 0).unwrap();
    assert_eq!(none.pane_attributes(), None);
    assert_eq!(none.selection_attributes(), None);
    let panes = FrozenPanes::new(4, 2).unwrap();
    let text = |pairs: &[(&'static str, String)]| {
        (pairs.iter())
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert_eq!(
        text(&panes.pane_attributes().unwrap()),
        "xSplit=2 ySplit=4 topLeftCell=C5 activePane=bottomRight state=frozen"
    );
    assert_eq!(
        text(&panes.selection_attributes().unwrap()),
        "pane=bottomRight activeCell=C5 sqref=C5"
    );
}

fn protection(change: impl FnOnce(&mut SheetProtection)) -> String {
    let mut protection = SheetProtection::default();
    change(&mut protection);
    written(|out| protection.write(out, ""))
}

#[test]
fn a_protection_writes_only_what_differs_from_the_schema_defaults() {
    assert_eq!(
        protection(|_| ()),
        r#"<sheetProtection sheet="1" objects="1" scenarios="1"/>"#
    );
    let out = protection(|p| {
        p.allow_select_locked_cells = false;
        p.allow_select_unlocked_cells = false;
    });
    assert_eq!(
        out,
        concat!(
            r#"<sheetProtection sheet="1" objects="1" scenarios="1" "#,
            r#"selectLockedCells="1" selectUnlockedCells="1"/>"#
        )
    );
}

#[test]
fn a_protection_that_allows_everything_turns_every_default_off() {
    let out = protection(|p| {
        *p = SheetProtection {
            allow_format_cells: true,
            allow_format_columns: true,
            allow_format_rows: true,
            allow_insert_columns: true,
            allow_insert_rows: true,
            allow_insert_hyperlinks: true,
            allow_delete_columns: true,
            allow_delete_rows: true,
            allow_sort: true,
            allow_auto_filter: true,
            allow_pivot_tables: true,
            allow_edit_objects: true,
            allow_edit_scenarios: true,
            ..SheetProtection::default()
        };
    });
    assert_eq!(
        out,
        concat!(
            r#"<sheetProtection sheet="1" formatCells="0" formatColumns="0" formatRows="0" "#,
            r#"insertColumns="0" insertRows="0" insertHyperlinks="0" deleteColumns="0" "#,
            r#"deleteRows="0" sort="0" autoFilter="0" pivotTables="0"/>"#
        )
    );
}

#[test]
fn each_allowance_changes_its_own_attribute_and_no_other() {
    type Change = fn(&mut SheetProtection);
    let cases: [(Change, &str, bool); 15] = [
        (|p| p.allow_format_cells = true, r#" formatCells="0""#, true),
        (
            |p| p.allow_format_columns = true,
            r#" formatColumns="0""#,
            true,
        ),
        (|p| p.allow_format_rows = true, r#" formatRows="0""#, true),
        (
            |p| p.allow_insert_columns = true,
            r#" insertColumns="0""#,
            true,
        ),
        (|p| p.allow_insert_rows = true, r#" insertRows="0""#, true),
        (
            |p| p.allow_insert_hyperlinks = true,
            r#" insertHyperlinks="0""#,
            true,
        ),
        (
            |p| p.allow_delete_columns = true,
            r#" deleteColumns="0""#,
            true,
        ),
        (|p| p.allow_delete_rows = true, r#" deleteRows="0""#, true),
        (|p| p.allow_sort = true, r#" sort="0""#, true),
        (|p| p.allow_auto_filter = true, r#" autoFilter="0""#, true),
        (|p| p.allow_pivot_tables = true, r#" pivotTables="0""#, true),
        (|p| p.allow_edit_objects = true, r#" objects="1""#, false),
        (
            |p| p.allow_edit_scenarios = true,
            r#" scenarios="1""#,
            false,
        ),
        (
            |p| p.allow_select_locked_cells = false,
            r#" selectLockedCells="1""#,
            true,
        ),
        (
            |p| p.allow_select_unlocked_cells = false,
            r#" selectUnlockedCells="1""#,
            true,
        ),
    ];
    let base = protection(|_| ());
    for (change, attribute, written) in cases {
        let out = protection(change);
        assert_eq!(out.contains(attribute), written, "{attribute}: {out}");
        assert_eq!(
            out.matches('=').count().abs_diff(base.matches('=').count()),
            1,
            "{out}"
        );
    }
}

#[test]
fn a_password_hash_comes_first_with_the_legacy_attribute_never_written() {
    let salt = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let protection = SheetProtection {
        password: Some(PasswordHash::sha512("password", salt, 1).unwrap()),
        ..SheetProtection::default()
    };
    let out = written(|out| protection.write(out, "ss:"));
    assert_eq!(
        out,
        concat!(
            r#"<ss:sheetProtection algorithmName="SHA-512" "#,
            r#"hashValue="NZDQ+jZ6mR+5ccpyiafYnruBqBmEmQtYT0CwkzKLh4BM7grXD6j4MxoSxmy9vtOUDwD13ydo2XOsckpiBoEAJg==" "#,
            r#"saltValue="AAECAwQFBgcICQoLDA0ODw==" spinCount="1" "#,
            r#"sheet="1" objects="1" scenarios="1"/>"#
        )
    );
    assert!(!out.contains(" password="));
}

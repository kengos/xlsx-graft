//! Choosing a sheet by name or index and opening its rows, and the code
//! and words of each refusal.

use super::{NS, R, open, rel, workbook, worksheet};
use crate::{Refusal, SheetChoice, SheetKind};
use xlsx_graft_diagnostics::Diagnostics;

#[test]
fn a_sheet_is_chosen_by_name_or_index_and_its_rows_open_only_on_a_worksheet() {
    let chart = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet";
    let related = rel("rId8", chart, "chartsheets/sheet1.xml");
    let xml = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}"><sheets><sheet name="A" sheetId="1" r:id="rId1"/><sheet name="C" sheetId="2" r:id="rId8"/><sheet name="M" sheetId="3" r:id="rId7"/><sheet name="a" sheetId="4" r:id="rId1"/></sheets></workbook>"#
    );
    let mut items = workbook(&[("A", &worksheet(""))], false, &related, vec![]);
    items[2] = super::Item::stored("xl/workbook.xml", xml.as_bytes());
    items.push(super::Item::stored(
        "xl/chartsheets/sheet1.xml",
        b"<chartsheet/>",
    ));
    let book = open(items, &mut Diagnostics::new());
    let chosen = |choice| {
        book.choose(choice)
            .map(|(index, sheet)| (index, sheet.name()))
    };
    assert_eq!(chosen(SheetChoice::Index(1)), Ok((1, "C")));
    assert_eq!(chosen(SheetChoice::Name("c")), Ok((1, "C")));
    let refused = [
        (SheetChoice::Name("X"), Refusal::NoSuchName("X".to_owned())),
        (SheetChoice::Name("A"), Refusal::Duplicate("A".to_owned())),
        (
            SheetChoice::Index(4),
            Refusal::NoSuchIndex { index: 4, count: 4 },
        ),
    ];
    for (choice, refusal) in refused {
        assert_eq!(chosen(choice), Err(refusal), "{choice:?}");
    }
    assert!(book.open_rows(0).is_ok());
    let not_worksheet = Refusal::NotWorksheet {
        name: "C".to_owned(),
        kind: SheetKind::Chartsheet,
    };
    assert_eq!(book.open_rows(1).err(), Some(not_worksheet));
    let unreadable = Refusal::SheetUnreadable("M".to_owned());
    assert_eq!(book.open_rows(2).err(), Some(unreadable));
    let none = Refusal::NoSuchIndex { index: 9, count: 4 };
    assert_eq!(book.open_rows(9).err(), Some(none));
}

#[test]
fn each_refusal_has_its_code_and_says_why() {
    let named = |name: &str| name.to_owned();
    let refusals = [
        (
            Refusal::WorkbookUnreadable,
            "workbook_unreadable",
            "the workbook cannot be read; the diagnostics say why",
        ),
        (
            Refusal::NoSuchName(named("X")),
            "sheet_not_found",
            "no sheet is named \"X\"",
        ),
        (
            Refusal::NoSuchIndex { index: 3, count: 2 },
            "sheet_not_found",
            "there is no sheet at index 3: the workbook has 2, from index 0",
        ),
        (
            Refusal::Duplicate(named("A")),
            "sheet_ambiguous",
            "more than one sheet is named \"A\", letter case aside",
        ),
        (
            Refusal::NotWorksheet {
                name: named("C"),
                kind: SheetKind::Chartsheet,
            },
            "sheet_not_worksheet",
            "sheet \"C\" is a chartsheet, which holds no cells",
        ),
        (
            Refusal::NotWorksheet {
                name: named("D"),
                kind: SheetKind::Dialogsheet,
            },
            "sheet_not_worksheet",
            "sheet \"D\" is a dialog sheet, which holds no cells",
        ),
        (
            Refusal::NotWorksheet {
                name: named("M"),
                kind: SheetKind::Macrosheet,
            },
            "sheet_not_worksheet",
            "sheet \"M\" is a macro sheet, whose cells are not read",
        ),
        (
            Refusal::SheetUnreadable(named("S")),
            "sheet_unreadable",
            "sheet \"S\" cannot be read; the diagnostics say why",
        ),
    ];
    for (refusal, code, message) in refusals {
        assert_eq!(
            (refusal.code(), refusal.to_string()),
            (code, message.to_owned())
        );
    }
}

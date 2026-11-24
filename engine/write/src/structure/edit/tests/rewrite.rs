//! The workbook part `apply` writes, as exact XML: what each call changes in
//! `sheets`, `definedNames`, `bookViews` and `calcPr`, with the markup around
//! left as it was, for roots with and without a prefix, Transitional and
//! Strict.

use super::{NS, R, applied, package_raw};

const STRICT_NS: &str = "http://purl.oclc.org/ooxml/spreadsheetml/main";
const STRICT_R: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships";

/// The worksheet relationships `rId1` to `rId<n>`.
fn rels(n: usize) -> String {
    (1..=n)
        .map(|i| {
            format!(
                r#"<Relationship Id="rId{i}" Type="{R}/worksheet" Target="worksheets/sheet{i}.xml"/>"#
            )
        })
        .collect()
}

/// A sheet element of the sheet number `n`, named `name`.
fn sheet(n: usize, name: &str) -> String {
    format!(r#"<sheet name="{name}" sheetId="{n}" r:id="rId{n}"/>"#)
}

/// The sheets `Alpha`, `Beta` and `Gamma`, the first `count` of them.
fn sheets(count: usize) -> String {
    let names = ["Alpha", "Beta", "Gamma"];
    let list: String = (names.iter().zip(1..=count))
        .map(|(name, n)| sheet(n, name))
        .collect();
    format!("<sheets>{list}</sheets>")
}

/// A Transitional workbook part of `body`.
fn workbook(body: &str) -> String {
    format!(r#"<workbook xmlns="{NS}" xmlns:r="{R}">{body}</workbook>"#)
}

/// The workbook part `body` of `count` sheets, after `act`.
fn run(
    body: &str,
    count: usize,
    act: impl FnOnce(&mut crate::structure::StructureEdit<'static>),
) -> String {
    let package = package_raw(&workbook(body), &rels(count), count, &[], "");
    applied(package, act).part("/xl/workbook.xml").unwrap()
}

#[test]
fn a_rename_writes_the_name_and_the_formulas_that_name_it() {
    let body = format!(
        r#"{}<definedNames><definedName name="N" comment="c">Alpha!$A$1</definedName></definedNames>"#,
        sheets(2)
    );
    let got = run(&body, 2, |edit| {
        edit.rename_sheet("alpha", "A&B").unwrap();
    });
    let expected = workbook(concat!(
        r#"<sheets><sheet name="A&amp;B" sheetId="1" r:id="rId1"/>"#,
        r#"<sheet name="Beta" sheetId="2" r:id="rId2"/></sheets>"#,
        r#"<definedNames><definedName name="N" comment="c">'A&amp;B'!$A$1</definedName></definedNames>"#,
    ));
    assert_eq!(got, expected);
}

#[test]
fn an_added_sheet_takes_its_place_with_the_next_id_and_the_relationship_id_the_edit_gave() {
    // The relationship `rId2` is not a worksheet's, so the new sheet's is
    // `rId3`; the parts `SHEET2.xml` and `Sheet3.xml` are there under another
    // letter case, so the new part is `sheet4.xml`.
    let rels = format!(
        r#"<Relationship Id="rId1" Type="{R}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{R}/styles" Target="styles.xml"/>"#
    );
    let package = package_raw(
        &workbook(&sheets(1)),
        &rels,
        1,
        &[
            ("xl/styles.xml", "", "<styleSheet/>"),
            ("xl/worksheets/SHEET2.xml", "", "<worksheet/>"),
            ("xl/worksheets/Sheet3.xml", "", "<worksheet/>"),
        ],
        "",
    );
    let output = applied(package, |edit| {
        edit.add_sheet("New", Some(0), None).unwrap();
    });
    let expected = workbook(concat!(
        r#"<sheets><sheet name="New" sheetId="2" r:id="rId3"/>"#,
        r#"<sheet name="Alpha" sheetId="1" r:id="rId1"/></sheets>"#,
    ));
    assert_eq!(output.part("/xl/workbook.xml").unwrap(), expected);
    assert_eq!(
        output.part("/xl/worksheets/sheet4.xml").unwrap(),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"{NS}\" xmlns:r=\"{R}\"><sheetData/></worksheet>"
        )
    );
    assert!(output.workbook_relationships().contains(&(
        "rId3".to_owned(),
        format!("{R}/worksheet"),
        "/xl/worksheets/sheet4.xml".to_owned()
    )));
}

#[test]
fn a_root_with_a_prefix_gets_elements_with_that_prefix() {
    let body = concat!(
        r#"<x:sheets><x:sheet name="Alpha" sheetId="1" r:id="rId1"/></x:sheets>"#,
        r#"<x:definedNames><x:definedName name="Old">1</x:definedName></x:definedNames>"#,
    );
    let body = format!(r#"{body}<z xmlns=""/>"#);
    let text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<x:workbook xmlns:x=\"{NS}\" xmlns:r=\"{R}\">{body}</x:workbook>\n"
    );
    let package = package_raw(&text, &rels(1), 1, &[], "");
    let output = applied(package, |edit| {
        edit.add_sheet("New", None, None).unwrap();
        edit.set_defined_name("N", None, "Alpha!$A$1").unwrap();
        edit.delete_sheet("Alpha").unwrap();
        edit.add_sheet("Other", None, None).unwrap();
        edit.remove_defined_name("Old", None).unwrap();
    });
    let expected = format!(
        concat!(
            r#"<x:workbook xmlns:x="{}" xmlns:r="{}"><x:sheets>"#,
            r#"<x:sheet name="New" sheetId="2" r:id="rId2"/>"#,
            r#"<x:sheet name="Other" sheetId="3" r:id="rId3"/></x:sheets>"#,
            r#"<x:definedNames><x:definedName name="N">#REF!$A$1</x:definedName></x:definedNames>"#,
            r#"<z xmlns=""/><x:calcPr fullCalcOnLoad="1"/></x:workbook>"#,
        ),
        NS, R
    );
    let expected = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{expected}\n");
    assert_eq!(output.part("/xl/workbook.xml").unwrap(), expected);
}

#[test]
fn a_strict_workbook_gets_strict_sheets_and_the_prefix_the_first_sheet_declares() {
    let body = format!(
        r#"<sheets><sheet name="Alpha" sheetId="1" xmlns:q="{STRICT_R}" q:id="rId1"/></sheets>"#
    );
    let text = format!(r#"<workbook xmlns="{STRICT_NS}">{body}</workbook>"#);
    let rels = format!(
        r#"<Relationship Id="rId1" Type="{STRICT_R}/worksheet" Target="worksheets/sheet1.xml"/>"#
    );
    let package = package_raw(&text, &rels, 1, &[], "");
    let output = applied(package, |edit| {
        edit.add_sheet("New", None, None).unwrap();
    });
    let expected = format!(
        concat!(
            r#"<workbook xmlns="{}"><sheets>"#,
            r#"<sheet name="Alpha" sheetId="1" xmlns:q="{}" q:id="rId1"/>"#,
            r#"<sheet name="New" sheetId="2" xmlns:q="{}" q:id="rId2"/></sheets></workbook>"#,
        ),
        STRICT_NS, STRICT_R, STRICT_R
    );
    assert_eq!(output.part("/xl/workbook.xml").unwrap(), expected);
    assert_eq!(
        output.part("/xl/worksheets/sheet2.xml").unwrap(),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"{STRICT_NS}\" xmlns:r=\"{STRICT_R}\"><sheetData/></worksheet>"
        )
    );
    assert!(output.workbook_relationships().contains(&(
        "rId2".to_owned(),
        format!("{STRICT_R}/worksheet"),
        "/xl/worksheets/sheet2.xml".to_owned()
    )));
}

#[test]
fn a_defined_names_element_is_added_before_the_first_element_that_follows_it() {
    let name = r#"<definedNames><definedName name="N">1</definedName></definedNames>"#;
    let set = |edit: &mut crate::structure::StructureEdit<'static>| {
        edit.set_defined_name("N", None, "1").unwrap();
    };
    for (after, expected) in [
        (
            r#"<calcPr calcId="1"/>"#.to_owned(),
            format!(r#"{name}<calcPr calcId="1"/>"#),
        ),
        ("<extLst/>".to_owned(), format!("{name}<extLst/>")),
        (
            "<oleSize ref=\"A1\"/><extLst/>".to_owned(),
            format!("{name}<oleSize ref=\"A1\"/><extLst/>"),
        ),
        (String::new(), name.to_owned()),
    ] {
        let got = run(&format!("{}{after}", sheets(1)), 1, set);
        assert_eq!(got, workbook(&format!("{}{expected}", sheets(1))));
    }
}

#[test]
fn defined_names_are_dropped_with_their_element_added_into_an_empty_one_and_rebuilt_as_an_empty_element_or_with_start_and_end_tags()
 {
    // Every name removed: the element is removed.
    let body = format!(
        r#"{}<definedNames><definedName name="N">1</definedName></definedNames>"#,
        sheets(1)
    );
    let got = run(&body, 1, |edit| {
        edit.remove_defined_name("n", None).unwrap()
    });
    assert_eq!(got, workbook(&sheets(1)));
    // An empty element gets the name inside.
    let body = format!("{}<definedNames/>", sheets(1));
    let got = run(&body, 1, |edit| {
        edit.set_defined_name("N", None, "1").unwrap()
    });
    let names = r#"<definedNames><definedName name="N">1</definedName></definedNames>"#;
    assert_eq!(got, workbook(&format!("{}{names}", sheets(1))));
    // A name in the form with a start and an end tag, and one in the empty
    // form, removed and replaced; the attributes stay.
    let body = format!(
        r#"{}<definedNames><definedName name="E"></definedName><definedName name="F" hidden="1"/><definedName name="G">3</definedName></definedNames>"#,
        sheets(1)
    );
    let got = run(&body, 1, |edit| {
        edit.remove_defined_name("E", None).unwrap();
        edit.set_defined_name("F", None, "2").unwrap();
    });
    let names = r#"<definedNames><definedName name="F" hidden="1">2</definedName><definedName name="G">3</definedName></definedNames>"#;
    assert_eq!(got, workbook(&format!("{}{names}", sheets(1))));
}

#[test]
fn a_name_is_written_with_its_escapes_made_and_the_xml_escaped() {
    let got = run(&sheets(1), 1, |edit| {
        edit.set_defined_name("a_x0041_b", None, "\"a<b&c\"")
            .unwrap();
    });
    let names = r#"<definedNames><definedName name="a_x005F_x0041_b">"a&lt;b&amp;c"</definedName></definedNames>"#;
    assert_eq!(got, workbook(&format!("{}{names}", sheets(1))));
}

#[test]
fn local_sheet_ids_are_written_as_the_sheets_positions_after_a_move_and_a_delete() {
    let names = concat!(
        r#"<definedNames><definedName name="OfA" localSheetId="0">Alpha!$A$1</definedName>"#,
        r#"<definedName name="OfB" localSheetId="1">Beta!$A$1</definedName>"#,
        r#"<definedName name="Raw" localSheetId="9">1</definedName>"#,
        r#"<definedName name="Book">1</definedName></definedNames>"#,
    );
    let body = format!("{}{names}", sheets(3));
    let got = run(&body, 3, |edit| edit.move_sheet("Beta", 0).unwrap());
    let expected = concat!(
        r#"<sheets><sheet name="Beta" sheetId="2" r:id="rId2"/>"#,
        r#"<sheet name="Alpha" sheetId="1" r:id="rId1"/>"#,
        r#"<sheet name="Gamma" sheetId="3" r:id="rId3"/></sheets>"#,
        r#"<definedNames><definedName name="OfA" localSheetId="1">Alpha!$A$1</definedName>"#,
        r#"<definedName name="OfB" localSheetId="0">Beta!$A$1</definedName>"#,
        r#"<definedName name="Raw" localSheetId="9">1</definedName>"#,
        r#"<definedName name="Book">1</definedName></definedNames>"#,
    );
    assert_eq!(got, workbook(expected));
    let got = run(&body, 3, |edit| edit.delete_sheet("Alpha").unwrap());
    let expected = concat!(
        r#"<sheets><sheet name="Beta" sheetId="2" r:id="rId2"/>"#,
        r#"<sheet name="Gamma" sheetId="3" r:id="rId3"/></sheets>"#,
        r#"<definedNames><definedName name="OfB" localSheetId="0">Beta!$A$1</definedName>"#,
        r#"<definedName name="Raw" localSheetId="9">1</definedName>"#,
        r#"<definedName name="Book">1</definedName></definedNames>"#,
    );
    assert_eq!(got, workbook(expected));
}

#[test]
fn the_views_are_pointed_at_the_sheets_again_and_the_attributes_they_lacked_are_added() {
    let views = |list: &str| format!("<bookViews>{list}</bookViews>");
    let before = |list: &str| format!("{}{}", views(list), sheets(3));
    // Alpha is deleted: Gamma, the third sheet, is the second.
    let body = before(r#"<workbookView activeTab="2" firstSheet="2"/>"#);
    let got = run(&body, 3, |edit| edit.delete_sheet("Alpha").unwrap());
    let expected = format!(
        r#"{}<sheets><sheet name="Beta" sheetId="2" r:id="rId2"/><sheet name="Gamma" sheetId="3" r:id="rId3"/></sheets>"#,
        views(r#"<workbookView activeTab="1" firstSheet="1"/>"#)
    );
    assert_eq!(got, workbook(&expected));
    // The sheet the view points at is deleted: the next sheet, Gamma, is at the
    // position the sheet that was deleted was at, so the attribute stays 1. Left
    // pointing at the sheet that was deleted, the view would be written as 0.
    let body = before(r#"<workbookView activeTab="1"/>"#);
    let got = run(&body, 3, |edit| edit.delete_sheet("Beta").unwrap());
    assert!(got.starts_with(&format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}"><bookViews><workbookView activeTab="1"/></bookViews>"#
    )));
    // The next sheet is hidden: the one after it, Delta, is the third sheet
    // when Beta is deleted, so the attribute is 2.
    let four = concat!(
        r#"<bookViews><workbookView activeTab="1"/></bookViews><sheets>"#,
        r#"<sheet name="Alpha" sheetId="1" r:id="rId1"/><sheet name="Beta" sheetId="2" r:id="rId2"/>"#,
        r#"<sheet name="Hid" sheetId="3" state="hidden" r:id="rId3"/>"#,
        r#"<sheet name="Delta" sheetId="4" r:id="rId4"/></sheets>"#,
    );
    let got = run(four, 4, |edit| edit.delete_sheet("Beta").unwrap());
    assert!(got.contains(r#"<bookViews><workbookView activeTab="2"/></bookViews>"#));
    // Alpha moves to the second place: the views that name Alpha point at its
    // new position, the one past the sheets stays, the one with a start tag
    // and an end tag is written as one.
    let body = before(concat!(
        r#"<workbookView activeTab="0"/>"#,
        r#"<workbookView activeTab="1"></workbookView>"#,
        r#"<workbookView activeTab="9" firstSheet="9"/>"#,
    ));
    let got = run(&body, 3, |edit| edit.move_sheet("Alpha", 1).unwrap());
    let expected = views(concat!(
        r#"<workbookView activeTab="1" firstSheet="1"/>"#,
        r#"<workbookView activeTab="0" firstSheet="1"></workbookView>"#,
        r#"<workbookView activeTab="9" firstSheet="9"/>"#,
    ));
    assert!(got.starts_with(&format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}">{expected}"#
    )));
}

#[test]
fn calc_pr_gets_full_calc_on_load_when_a_delete_wrote_ref() {
    let names = r#"<definedNames><definedName name="N">Beta!$A$1</definedName></definedNames>"#;
    let rewritten = r#"<definedNames><definedName name="N">#REF!$A$1</definedName></definedNames>"#;
    let one = r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/></sheets>"#;
    for (calc, expected) in [
        (
            r#"<calcPr calcId="1"/>"#,
            r#"<calcPr calcId="1" fullCalcOnLoad="1"/>"#,
        ),
        (
            r#"<calcPr fullCalcOnLoad="0" calcId="1"/>"#,
            r#"<calcPr fullCalcOnLoad="1" calcId="1"/>"#,
        ),
        (
            r#"<calcPr calcId="1"></calcPr>"#,
            r#"<calcPr calcId="1" fullCalcOnLoad="1"></calcPr>"#,
        ),
    ] {
        let body = format!("{}{names}{calc}", sheets(2));
        let got = run(&body, 2, |edit| edit.delete_sheet("Beta").unwrap());
        assert_eq!(got, workbook(&format!("{one}{rewritten}{expected}")));
    }
    // A rename writes no `#REF!`: calcPr stays.
    let body = format!(r#"{}{names}<calcPr calcId="1"/>"#, sheets(2));
    let got = run(&body, 2, |edit| edit.rename_sheet("Beta", "Delta").unwrap());
    assert!(got.ends_with(r#"<calcPr calcId="1"/></workbook>"#));
}

#[test]
fn sheets_written_over_several_lines_or_with_an_end_tag_are_written_in_the_new_order() {
    let body = concat!(
        "<sheets>\n  <sheet name=\"Alpha\" sheetId=\"1\" r:id=\"rId1\"></sheet>\n",
        "  <sheet name=\"Beta\" sheetId=\"2\" r:id=\"rId2\"/>\n</sheets>"
    );
    let got = run(body, 2, |edit| {
        edit.rename_sheet("Alpha", "Gamma").unwrap();
        edit.move_sheet("Gamma", 1).unwrap();
    });
    let expected = concat!(
        r#"<sheets><sheet name="Beta" sheetId="2" r:id="rId2"/>"#,
        r#"<sheet name="Gamma" sheetId="1" r:id="rId1"></sheet></sheets>"#,
    );
    assert_eq!(got, workbook(expected));
}

#[test]
fn what_a_sheet_element_holds_is_written_with_it() {
    let body = concat!(
        r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"><ext a="1">t</ext></sheet>"#,
        r#"<sheet name="Beta" sheetId="2" r:id="rId2"/></sheets>"#,
    );
    let got = run(body, 2, |edit| edit.move_sheet("Alpha", 1).unwrap());
    let expected = concat!(
        r#"<sheets><sheet name="Beta" sheetId="2" r:id="rId2"/>"#,
        r#"<sheet name="Alpha" sheetId="1" r:id="rId1"><ext a="1">t</ext></sheet></sheets>"#,
    );
    assert_eq!(got, workbook(expected));
}

#[test]
fn a_defined_name_the_edit_did_not_change_is_written_as_it_was() {
    // `Keep` is not changed by the edit, so its single-quoted attributes
    // are not written again; `Other` is changed, so it is.
    let body = format!(
        r#"{}<definedNames><definedName name='Keep' hidden='1'>1</definedName><definedName name="Other">2</definedName></definedNames>"#,
        sheets(1)
    );
    let got = run(&body, 1, |edit| {
        edit.set_defined_name("Other", None, "3").unwrap();
    });
    let names = r#"<definedNames><definedName name='Keep' hidden='1'>1</definedName><definedName name="Other">3</definedName></definedNames>"#;
    assert_eq!(got, workbook(&format!("{}{names}", sheets(1))));
}

#[test]
fn a_defined_names_element_stays_when_the_edit_changes_no_name() {
    let body = format!("{}<definedNames/>", sheets(1));
    let got = run(&body, 1, |edit| {
        edit.rename_sheet("Alpha", "Gamma").unwrap();
    });
    let expected = format!("<sheets>{}</sheets><definedNames/>", sheet(1, "Gamma"));
    assert_eq!(got, workbook(&expected));
}

#[test]
fn an_added_sheet_takes_the_relationship_id_attribute_of_the_first_sheet_among_others() {
    // The first sheet has an attribute `id` that is in no namespace before
    // its `r:id`: only `r:id` is the relationship id.
    let body = r#"<sheets><sheet name="Alpha" sheetId="1" id="x" r:id="rId1"/></sheets>"#;
    let got = run(body, 1, |edit| {
        edit.add_sheet("New", None, None).unwrap();
    });
    let expected = concat!(
        r#"<sheets><sheet name="Alpha" sheetId="1" id="x" r:id="rId1"/>"#,
        r#"<sheet name="New" sheetId="2" r:id="rId2"/></sheets>"#,
    );
    assert_eq!(got, workbook(expected));
}

#[test]
fn only_the_name_of_the_sheet_element_is_rewritten_when_it_holds_elements_of_its_own() {
    // The `ext` with an end tag and the empty `x` inside `Alpha` are written
    // as they were, without a `name`, and stay inside `Alpha`.
    let body = concat!(
        r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"><ext a="1">t</ext><x/></sheet>"#,
        r#"<sheet name="Beta" sheetId="2" r:id="rId2"/></sheets>"#,
    );
    let got = run(body, 2, |edit| edit.rename_sheet("Alpha", "Gamma").unwrap());
    let expected = concat!(
        r#"<sheets><sheet name="Gamma" sheetId="1" r:id="rId1"><ext a="1">t</ext><x/></sheet>"#,
        r#"<sheet name="Beta" sheetId="2" r:id="rId2"/></sheets>"#,
    );
    assert_eq!(got, workbook(expected));
}

#[test]
fn children_of_sheets_that_are_no_sheet_of_the_namespace_are_written_after_the_sheets() {
    // The element in another namespace before the sheets, and the second
    // sheet, have an `id` attribute of the relationships namespace under
    // another prefix: the added sheet is written as the first sheet writes
    // its own, `r:id`.
    let body = format!(
        concat!(
            r#"<sheets><x:foo xmlns:x="urn:x" xmlns:q="{}" q:id="z" a="1"/>"#,
            r#"<sheet name="Alpha" sheetId="1" r:id="rId1"/>"#,
            r#"<sheet name="Beta" sheetId="2" xmlns:q="{}" q:id="rId2"/>"#,
            r#"<mc:AlternateContent xmlns:mc="urn:mc"><x/></mc:AlternateContent></sheets>"#,
        ),
        R, R
    );
    let got = run(&body, 2, |edit| {
        edit.rename_sheet("Alpha", "Gamma").unwrap();
        edit.move_sheet("Beta", 0).unwrap();
        edit.add_sheet("New", None, None).unwrap();
    });
    let expected = format!(
        concat!(
            r#"<sheets><sheet name="Beta" sheetId="2" xmlns:q="{}" q:id="rId2"/>"#,
            r#"<sheet name="Gamma" sheetId="1" r:id="rId1"/>"#,
            r#"<sheet name="New" sheetId="3" r:id="rId3"/>"#,
            r#"<x:foo xmlns:x="urn:x" xmlns:q="{}" q:id="z" a="1"/>"#,
            r#"<mc:AlternateContent xmlns:mc="urn:mc"><x/></mc:AlternateContent></sheets>"#,
        ),
        R, R
    );
    assert_eq!(got, workbook(&expected));
}

#[test]
fn calc_pr_is_added_before_the_first_element_that_follows_it_and_after_defined_names() {
    let calc = r#"<calcPr fullCalcOnLoad="1"/>"#;
    let one = r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/></sheets>"#;
    let two = format!(
        "{}{}",
        r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/>"#,
        r#"<sheet name="Beta" sheetId="2" r:id="rId2"/></sheets>"#
    );
    for (after, tail) in [
        ("<oleSize ref=\"A1\"/>", "<oleSize ref=\"A1\"/>"),
        ("<extLst/>", "<extLst/>"),
        (
            "<oleSize ref=\"A1\"/><extLst/>",
            "<oleSize ref=\"A1\"/><extLst/>",
        ),
        ("", ""),
    ] {
        // A delete that writes `#REF!` and no definedNames element to put it in.
        let body = format!(
            r#"{two}<definedNames><definedName name="N">Beta!$A$1</definedName></definedNames>{after}"#
        );
        let got = run(&body, 2, |edit| edit.delete_sheet("Beta").unwrap());
        let rewritten =
            r#"<definedNames><definedName name="N">#REF!$A$1</definedName></definedNames>"#;
        assert_eq!(got, workbook(&format!("{one}{rewritten}{calc}{tail}")));
        // Both elements added in one edit: definedNames first.
        let body = format!("{two}{after}");
        let got = run(&body, 2, |edit| {
            edit.set_defined_name("N", None, "Beta!$A$1").unwrap();
            edit.delete_sheet("Beta").unwrap();
        });
        assert_eq!(got, workbook(&format!("{one}{rewritten}{calc}{tail}")));
    }
}

#[test]
fn a_name_added_to_a_part_whose_defined_names_come_after_calc_pr_is_written_once_in_that_element() {
    // The model does not check the order of the root's children, so the part
    // has its one `definedNames` after `calcPr`: the name added is written in
    // it, not in a second `definedNames` before `calcPr`.
    let body = format!(
        r#"{}<calcPr/><definedNames><definedName name="A">1</definedName></definedNames>"#,
        sheets(1)
    );
    let got = run(&body, 1, |edit| {
        edit.set_defined_name("B", None, "2").unwrap();
    });
    let expected = workbook(&format!(
        r#"{}<calcPr/><definedNames><definedName name="A">1</definedName><definedName name="B">2</definedName></definedNames>"#,
        sheets(1)
    ));
    assert_eq!(got, expected);
}

#[test]
fn a_comment_or_a_processing_instruction_between_the_sheets_is_written_after_them() {
    let body = concat!(
        r#"<sheets><!-- c --><sheet name="Alpha" sheetId="1" r:id="rId1"/>"#,
        r#"<?pi x?></sheets>"#
    );
    let got = run(body, 1, |edit| {
        edit.add_sheet("New", None, None).unwrap();
    });
    let expected = concat!(
        r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/>"#,
        r#"<sheet name="New" sheetId="2" r:id="rId2"/><!-- c --><?pi x?></sheets>"#
    );
    assert_eq!(got, workbook(expected));
}

#[test]
fn a_comment_inside_a_sheet_element_stays_inside_it() {
    let body =
        r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"><!-- in --></sheet></sheets>"#;
    let got = run(body, 1, |edit| {
        edit.add_sheet("New", None, None).unwrap();
    });
    let expected = concat!(
        r#"<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"><!-- in --></sheet>"#,
        r#"<sheet name="New" sheetId="2" r:id="rId2"/></sheets>"#
    );
    assert_eq!(got, workbook(expected));
}

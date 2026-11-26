//! What `apply` does to the parts of the package: the parts a delete leaves
//! out and the ones it keeps, `calcChain`, `docProps/app.xml`, a sheet added
//! and deleted, and the parts an edit does not touch.

use super::{NS, R, applied, package_raw};
use crate::structure::model::SheetState;

const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const EXTENDED: &str = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties";
const STRICT_EXTENDED: &str = "http://purl.oclc.org/ooxml/officeDocument/extendedProperties";
const APP_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties";
const STRICT_APP_TYPE: &str =
    "http://purl.oclc.org/ooxml/officeDocument/relationships/extendedProperties";
const APP_MEDIA: &str = "application/vnd.openxmlformats-officedocument.extended-properties+xml";

/// A workbook part of the sheets `Alpha` and `Beta`.
fn workbook() -> String {
    format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}"><sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/><sheet name="Beta" sheetId="2" r:id="rId2"/></sheets></workbook>"#
    )
}

/// The relationships of the two sheets, then `more`.
fn rels(more: &str) -> String {
    let sheets: String = (1..=2)
        .map(|n| {
            format!(
                r#"<Relationship Id="rId{n}" Type="{R}/worksheet" Target="worksheets/sheet{n}.xml"/>"#
            )
        })
        .collect();
    format!("{sheets}{more}")
}

/// A Relationships part of `body`.
fn relationships(body: &str) -> String {
    format!(r#"<Relationships xmlns="{REL}">{body}</Relationships>"#)
}

#[test]
fn a_delete_leaves_out_what_only_the_deleted_sheet_targeted_and_keeps_the_rest() {
    let sheet1 = relationships(&format!(
        r#"<Relationship Id="rId1" Type="{R}/drawing" Target="../drawings/drawing1.xml"/><Relationship Id="rId2" Type="{R}/comments" Target="../comments1.xml"/>"#
    ));
    let sheet2 = relationships(&format!(
        r#"<Relationship Id="rId1" Type="{R}/image" Target="../media/shared.png"/>"#
    ));
    let drawing = relationships(&format!(
        r#"<Relationship Id="rId1" Type="{R}/image" Target="../media/image1.png"/><Relationship Id="rId2" Type="{R}/image" Target="../media/shared.png"/><Relationship Id="rId3" Type="{R}/image" Target="../media/pkg.png"/>"#
    ));
    let package = package_raw(
        &workbook(),
        &rels(""),
        2,
        &[
            ("xl/worksheets/_rels/sheet1.xml.rels", "", &sheet1),
            ("xl/worksheets/_rels/sheet2.xml.rels", "", &sheet2),
            (
                "xl/drawings/drawing1.xml",
                "application/vnd.openxmlformats-officedocument.drawing+xml",
                "<d/>",
            ),
            ("xl/drawings/_rels/drawing1.xml.rels", "", &drawing),
            (
                "xl/comments1.xml",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml",
                "<c/>",
            ),
            ("xl/media/image1.png", "image/png", "x"),
            ("xl/media/shared.png", "image/png", "x"),
            ("xl/media/pkg.png", "image/png", "x"),
        ],
        &format!(r#"<Relationship Id="rId9" Type="{R}/image" Target="xl/media/pkg.png"/>"#),
    );
    let output = applied(package, |edit| edit.delete_sheet("Alpha").unwrap());
    let names = output.names();
    for kept in [
        "/xl/worksheets/sheet2.xml",
        "/xl/worksheets/_rels/sheet2.xml.rels",
        "/xl/media/shared.png",
        "/xl/media/pkg.png",
        "/xl/workbook.xml",
    ] {
        assert!(names.iter().any(|name| name == kept), "{kept} in {names:?}");
    }
    for gone in [
        "/xl/worksheets/sheet1.xml",
        "/xl/worksheets/_rels/sheet1.xml.rels",
        "/xl/drawings/drawing1.xml",
        "/xl/drawings/_rels/drawing1.xml.rels",
        "/xl/comments1.xml",
        "/xl/media/image1.png",
    ] {
        assert!(
            !names.iter().any(|name| name == gone),
            "{gone} in {names:?}"
        );
    }
}

#[test]
fn two_parts_that_target_each_other_stay_when_a_deleted_sheet_targets_one_of_them() {
    // `a.xml` and `b.xml` target each other and only the deleted sheet's
    // part targets `a.xml`. Nothing that stays targets them, yet both stay:
    // each is targeted by the other, which is not left out, and `removals`
    // keeps such a cycle rather than determining that it is unreachable.
    let sheet1 = relationships(&format!(
        r#"<Relationship Id="rId1" Type="{R}/x" Target="../a.xml"/>"#
    ));
    let a = relationships(&format!(
        r#"<Relationship Id="rId1" Type="{R}/x" Target="b.xml"/>"#
    ));
    let b = relationships(&format!(
        r#"<Relationship Id="rId1" Type="{R}/x" Target="a.xml"/>"#
    ));
    let package = package_raw(
        &workbook(),
        &rels(""),
        2,
        &[
            ("xl/worksheets/_rels/sheet1.xml.rels", "", &sheet1),
            ("xl/a.xml", "", "<a/>"),
            ("xl/_rels/a.xml.rels", "", &a),
            ("xl/b.xml", "", "<b/>"),
            ("xl/_rels/b.xml.rels", "", &b),
        ],
        "",
    );
    let output = applied(package, |edit| edit.delete_sheet("Alpha").unwrap());
    let names = output.names();
    assert!(names.contains(&"/xl/a.xml".to_owned()));
    assert!(names.contains(&"/xl/b.xml".to_owned()));
}

#[test]
fn the_calc_chain_part_is_removed_by_a_delete_and_kept_by_the_other_calls() {
    let chain = format!(r#"<Relationship Id="rId7" Type="{R}/calcChain" Target="calcChain.xml"/>"#);
    let media = "application/vnd.openxmlformats-officedocument.spreadsheetml.calcChain+xml";
    let package = || {
        package_raw(
            &workbook(),
            &rels(&chain),
            2,
            &[("xl/calcChain.xml", media, "<calcChain/>")],
            "",
        )
    };
    let has = |output: &super::Output| output.names().contains(&"/xl/calcChain.xml".to_owned());
    assert!(!has(&applied(package(), |edit| edit
        .delete_sheet("Beta")
        .unwrap())));
    assert!(has(&applied(package(), |edit| {
        edit.rename_sheet("Beta", "Gamma").unwrap();
        edit.move_sheet("Gamma", 0).unwrap();
        edit.add_sheet("New", None, None).unwrap();
    })));
    // Deleting a sheet this edit added does not delete a sheet the opened
    // workbook had.
    assert!(has(&applied(package(), |edit| {
        edit.add_sheet("New", None, None).unwrap();
        edit.delete_sheet("New").unwrap();
    })));
}

#[test]
fn a_sheet_added_and_deleted_is_not_added_and_the_workbook_part_is_not_written_again() {
    let package = package_raw(&workbook(), &rels(""), 2, &[], "");
    let output = applied(package, |edit| {
        edit.add_sheet("New", None, None).unwrap();
        edit.delete_sheet("New").unwrap();
    });
    assert!(output.holds(&workbook()));
    assert!(!output.names().iter().any(|name| name.contains("sheet3")));
}

#[test]
fn calls_that_leave_the_sheets_as_they_were_write_neither_the_workbook_part_nor_the_app_part() {
    let app = format!(
        r#"<Properties xmlns="{EXTENDED}"><TitlesOfParts/><Application>X</Application></Properties>"#
    );
    let link = format!(r#"<Relationship Id="rId2" Type="{APP_TYPE}" Target="docProps/app.xml"/>"#);
    let package = || {
        package_raw(
            &workbook(),
            &rels(""),
            2,
            &[("docProps/app.xml", APP_MEDIA, &app)],
            &link,
        )
    };
    let calls: [fn(&mut super::super::StructureEdit<'static>); 4] = [
        // Moved to the place it has.
        |edit| edit.move_sheet("Alpha", 0).unwrap(),
        // Renamed to the name it has, and to it in another letter case.
        |edit| edit.rename_sheet("Alpha", "Alpha").unwrap(),
        |edit| edit.rename_sheet("alpha", "Alpha").unwrap(),
        // Renamed and renamed back.
        |edit| {
            edit.rename_sheet("Alpha", "Gamma").unwrap();
            edit.rename_sheet("Gamma", "Alpha").unwrap();
        },
    ];
    for call in calls {
        let output = applied(package(), call);
        assert!(output.holds(&workbook()));
        assert!(output.holds(&app));
    }
}

#[test]
fn nothing_changes_for_an_edit_that_calls_nothing() {
    let package = package_raw(&workbook(), &rels(""), 2, &[], "");
    let output = applied(package, |_| {});
    assert!(output.holds(&workbook()));
}

#[test]
fn the_app_part_is_written_without_its_sheet_lists_when_the_sheets_change_and_not_for_a_defined_name()
 {
    let app = format!(
        r#"<Properties xmlns="{EXTENDED}" xmlns:vt="urn:vt"><Application>X</Application><HeadingPairs><vt:vector/></HeadingPairs><TitlesOfParts><vt:vector/></TitlesOfParts><Company/></Properties>"#
    );
    let link = format!(r#"<Relationship Id="rId2" Type="{APP_TYPE}" Target="docProps/app.xml"/>"#);
    let package = || {
        package_raw(
            &workbook(),
            &rels(""),
            2,
            &[("docProps/app.xml", APP_MEDIA, &app)],
            &link,
        )
    };
    let output = applied(package(), |edit| {
        edit.rename_sheet("Beta", "Gamma").unwrap()
    });
    let expected = format!(
        r#"<Properties xmlns="{EXTENDED}" xmlns:vt="urn:vt"><Application>X</Application><Company/></Properties>"#
    );
    assert_eq!(output.part("/docProps/app.xml").unwrap(), expected);
    // Setting a defined name does not change the sheets.
    let output = applied(package(), |edit| {
        edit.set_defined_name("N", None, "1").unwrap();
    });
    assert!(output.holds(&app));
    // Hiding a sheet changes the workbook part but not the sheets the app
    // part lists, so the app part is not written again.
    let output = applied(package(), |edit| {
        edit.set_sheet_state("Beta", SheetState::Hidden).unwrap();
    });
    assert!(output.holds(&app));
    assert!(!output.holds(&workbook()));
}

#[test]
fn only_the_titles_that_are_children_of_the_root_are_left_out_of_the_app_part() {
    let app = format!(
        r#"<Properties xmlns="{EXTENDED}" xmlns:vt="urn:vt"><TitlesOfParts><vt:vector/></TitlesOfParts><Manager><TitlesOfParts><vt:vector/></TitlesOfParts><HeadingPairs/></Manager></Properties>"#
    );
    let link = format!(r#"<Relationship Id="rId2" Type="{APP_TYPE}" Target="docProps/app.xml"/>"#);
    let package = package_raw(
        &workbook(),
        &rels(""),
        2,
        &[("docProps/app.xml", APP_MEDIA, &app)],
        &link,
    );
    let output = applied(package, |edit| {
        edit.rename_sheet("Beta", "Gamma").unwrap();
    });
    let expected = format!(
        r#"<Properties xmlns="{EXTENDED}" xmlns:vt="urn:vt"><Manager><TitlesOfParts><vt:vector/></TitlesOfParts><HeadingPairs/></Manager></Properties>"#
    );
    assert_eq!(output.part("/docProps/app.xml").unwrap(), expected);
}

#[test]
fn an_app_part_is_left_as_it_is_without_the_titles_in_its_namespace_or_when_it_is_unreadable() {
    let other = format!(
        r#"<Properties xmlns="{EXTENDED}"><TitlesOfParts xmlns="urn:other"/><Application>X</Application></Properties>"#
    );
    let broken = "<Properties><TitlesOfParts>";
    let link = format!(r#"<Relationship Id="rId2" Type="{APP_TYPE}" Target="docProps/app.xml"/>"#);
    for app in [other.as_str(), broken] {
        let package = package_raw(
            &workbook(),
            &rels(""),
            2,
            &[("docProps/app.xml", APP_MEDIA, app)],
            &link,
        );
        let output = applied(package, |edit| edit.delete_sheet("Beta").unwrap());
        assert!(output.holds(app));
    }
}

#[test]
fn a_strict_app_part_is_written_without_its_titles_too() {
    let app = format!(
        r#"<Properties xmlns="{STRICT_EXTENDED}"><TitlesOfParts/><Application>X</Application></Properties>"#
    );
    let link =
        format!(r#"<Relationship Id="rId2" Type="{STRICT_APP_TYPE}" Target="docProps/app.xml"/>"#);
    let package = package_raw(
        &workbook(),
        &rels(""),
        2,
        &[("docProps/app.xml", APP_MEDIA, &app)],
        &link,
    );
    let output = applied(package, |edit| edit.move_sheet("Beta", 0).unwrap());
    let expected = format!(
        r#"<Properties xmlns="{STRICT_EXTENDED}"><Application>X</Application></Properties>"#
    );
    assert_eq!(output.part("/docProps/app.xml").unwrap(), expected);
}

#[test]
fn a_workbook_the_model_cannot_read_is_not_opened() {
    use crate::WriteLimits;
    use crate::structure::StructureEdit;
    use xlsx_graft_diagnostics::Diagnostics;
    // No `sheets`.
    let text = format!(r#"<workbook xmlns="{NS}"/>"#);
    let mut package = package_raw(&text, &rels(""), 2, &[], "");
    let mut diagnostics = Diagnostics::new();
    let edit = StructureEdit::open(&mut package, &WriteLimits::default(), &mut diagnostics);
    assert!(edit.unwrap().is_none());
    assert!(diagnostics.has_errors());
}

#[test]
fn a_media_type_the_package_cannot_hold_is_refused() {
    use super::super::apply::media;
    use crate::WriteError;
    assert!(media("application/xml").is_ok());
    assert!(matches!(
        media("not a media type"),
        Err(WriteError::PartExists { name }) if name == "not a media type"
    ));
}

#[test]
fn a_path_that_is_no_part_name_is_refused() {
    use super::super::apply::part_name;
    use crate::WriteError;
    assert!(part_name("/xl/worksheets/sheet1.xml").is_ok());
    assert!(matches!(
        part_name("xl/no-slash.xml"),
        Err(WriteError::PartExists { name }) if name == "xl/no-slash.xml"
    ));
}

#[test]
fn a_delete_with_the_relationships_not_all_read_is_refused_before_the_package_edit_changes() {
    use super::Output;
    use crate::structure::StructureEdit;
    use crate::{PackageEdit, WriteError, WriteLimits};
    use xlsx_graft_diagnostics::Diagnostics;
    use xlsx_graft_spool::MemorySpools;
    // The Relationships part of the first sheet is not well formed, so the
    // parts the sheet targets are not all known, and a part cannot be removed.
    let mut package = package_raw(
        &workbook(),
        &rels(""),
        2,
        &[("xl/worksheets/_rels/sheet1.xml.rels", "", "<Relationships")],
        "",
    );
    let limits = WriteLimits::default();
    let mut diagnostics = Diagnostics::new();
    let mut edit = StructureEdit::open(&mut package, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    edit.add_sheet("New", None, None).unwrap();
    edit.delete_sheet("Beta").unwrap();
    let mut package_edit = PackageEdit::new(package);
    let error = edit.apply(&mut package_edit).unwrap_err();
    assert!(matches!(error, WriteError::GraphIncomplete), "{error:?}");
    // Nothing was added or removed: the package is written as it was.
    let bytes = package_edit
        .finish(
            Vec::<u8>::new(),
            &mut MemorySpools,
            &limits,
            &mut Diagnostics::new(),
        )
        .unwrap()
        .unwrap();
    let output = Output { bytes };
    assert!(output.holds(&workbook()));
    assert!(!output.names().iter().any(|name| name.contains("sheet3")));
    // A sheet that is only added needs no part removed.
    let mut package = package_raw(
        &workbook(),
        &rels(""),
        2,
        &[("xl/worksheets/_rels/sheet1.xml.rels", "", "<Relationships")],
        "",
    );
    let mut edit = StructureEdit::open(&mut package, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    edit.add_sheet("New", None, None).unwrap();
    let mut package_edit = PackageEdit::new(package);
    edit.apply(&mut package_edit).unwrap();
}

#[test]
fn a_new_sheet_takes_a_number_whose_part_and_relationships_part_names_are_free() {
    // `sheet3.xml` has no part, but the package lists the Relationships part
    // `sheet3.xml.rels` (its source part is missing), so `PackageEdit::add`
    // would refuse `sheet3.xml`. `sheet4.xml` is the name of a folder of a
    // listed part, which the package refuses too. `sheet5.xml` is free.
    let package = package_raw(
        &workbook(),
        &rels(""),
        2,
        &[
            (
                "xl/worksheets/_rels/sheet3.xml.rels",
                "",
                &relationships(""),
            ),
            ("xl/worksheets/sheet4.xml/inner.xml", "", "<inner/>"),
        ],
        "",
    );
    let output = applied(package, |edit| {
        edit.add_sheet("New", None, None).unwrap();
    });
    let names = output.names();
    assert!(names.contains(&"/xl/worksheets/sheet5.xml".to_owned()));
    assert!(!names.contains(&"/xl/worksheets/sheet3.xml".to_owned()));
    assert!(!names.contains(&"/xl/worksheets/sheet4.xml".to_owned()));
}

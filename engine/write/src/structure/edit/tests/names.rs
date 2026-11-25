//! `set_defined_name` and `remove_defined_name`: replace or append, the
//! spelling a name is written with, and each error with the state left as
//! it was.

use super::{NS, TABLE, defined, fingerprint, names_of, open, package};
use crate::WriteLimits;
use crate::structure::edit::StructureEdit;
use crate::structure::edit::state::{NameEntry, Scope, Text};
use crate::structure::{DefinedNameFault, StructureError};
use xlsx_graft_diagnostics::Diagnostics;

#[test]
fn a_name_is_appended_or_replaces_the_one_of_its_scope_and_spelling() {
    let names = defined(&[
        ("Total", "", "A!$A$1"),
        ("Total", "1", "B!$A$1"),
        ("Other", "", "A!$B$1"),
    ]);
    let mut edit = open(&["A", "B"], &names);
    edit.set_defined_name("total", None, "A!$C$3").unwrap();
    edit.set_defined_name("Total", Some("b"), "B!$D$4").unwrap();
    edit.set_defined_name("New", Some("A"), "A!$E$5").unwrap();
    edit.set_defined_name("Same", None, "1").unwrap();
    assert_eq!(
        names_of(&edit),
        [
            ("total".into(), Scope::Book, "A!$C$3".into()),
            ("Total".into(), Scope::Sheet(1), "B!$D$4".into()),
            ("Other".into(), Scope::Book, "A!$B$1".into()),
            ("New".into(), Scope::Sheet(0), "A!$E$5".into()),
            ("Same".into(), Scope::Book, "1".into()),
        ]
    );
    let entries = &edit.state.names;
    // The replaced name keeps the definedName it came from, and says its
    // spelling changed only when it did.
    assert_eq!(
        entries.iter().map(|entry| entry.origin).collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2), None, None]
    );
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.name_changed)
            .collect::<Vec<_>>(),
        [true, false, false, false, false]
    );
}

#[test]
fn a_built_in_name_is_written_with_its_own_spelling_whatever_letters_it_came_in() {
    let mut edit = open(&["A"], "");
    edit.set_defined_name("_XLNM.print_area", Some("A"), "A!$A$1:$B$2")
        .unwrap();
    edit.set_defined_name("_xlnm.PRINT_AREA", Some("A"), "A!$A$1:$C$3")
        .unwrap();
    assert_eq!(
        names_of(&edit),
        [(
            "_xlnm.Print_Area".into(),
            Scope::Sheet(0),
            "A!$A$1:$C$3".into()
        )]
    );
    // An original name written in other letters is the same name.
    let names = defined(&[("_xlnm.print_area", "0", "A!$A$1")]);
    let mut edit = open(&["A"], &names);
    edit.set_defined_name("_xlnm.Print_Area", Some("A"), "A!$B$2")
        .unwrap();
    assert_eq!(edit.state.names.len(), 1);
    assert_eq!(edit.state.names[0].written, "_xlnm.Print_Area");
    assert!(edit.state.names[0].name_changed);
}

#[test]
fn a_name_is_written_with_its_xhhhh_escapes_made() {
    let mut edit = open(&["A"], "");
    edit.set_defined_name("a_x0041_b", None, "\"_x0041_\"")
        .unwrap();
    let (written, _, text) = names_of(&edit).remove(0);
    assert_eq!(written, "a_x005F_x0041_b");
    // The text is stored as the caller gave it, its escapes not made.
    assert_eq!(text, "\"_x0041_\"");
    edit.remove_defined_name("a_x0041_b", None).unwrap();
    assert!(names_of(&edit).is_empty());
}

#[test]
fn a_set_that_fails_changes_nothing() {
    let package = &mut package(
        "",
        &[("A", "")],
        "",
        &[(
            "xl/tables/table1.xml",
            TABLE,
            &format!(r#"<table xmlns="{NS}" name="Tbl" displayName="Show"/>"#),
        )],
    );
    let mut diagnostics = Diagnostics::new();
    let mut edit = StructureEdit::open(package, &WriteLimits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    let before = fingerprint(&edit);
    let invalid = StructureError::DefinedNameInvalid {
        name: "1a".into(),
        reason: DefinedNameFault::Character,
    };
    assert_eq!(edit.set_defined_name("1a", None, "1"), Err(invalid));
    assert_eq!(
        edit.set_defined_name("show", None, "1"),
        Err(StructureError::DefinedNameTable {
            name: "show".into(),
            table: "Show".into()
        })
    );
    assert_eq!(
        edit.set_defined_name("TBL", Some("A"), "1"),
        Err(StructureError::DefinedNameTable {
            name: "TBL".into(),
            table: "Tbl".into()
        })
    );
    assert_eq!(
        edit.set_defined_name("N", Some("Z"), "1"),
        Err(StructureError::SheetMissing { name: "Z".into() })
    );
    assert_eq!(
        edit.set_defined_name("N", None, "\"abc"),
        Err(StructureError::FormulaMalformed {
            formula: "\"abc".into()
        })
    );
    assert_eq!(
        edit.set_defined_name("N", None, "A!A1+Z!A1"),
        Err(StructureError::FormulaSheetMissing {
            formula: "A!A1+Z!A1".into(),
            sheet: "Z".into()
        })
    );
    assert_eq!(
        edit.set_defined_name("N", None, "SUM(A:Y!A1)"),
        Err(StructureError::FormulaSheetMissing {
            formula: "SUM(A:Y!A1)".into(),
            sheet: "Y".into()
        })
    );
    assert_eq!(fingerprint(&edit), before);
}

#[test]
fn a_formula_may_name_a_sheet_in_any_letter_case_or_a_book_that_is_not_this_one() {
    let mut edit = open(&["Data"], "");
    edit.set_defined_name("N", None, "'data'!A1+DATA!A2+[1]Other!A1")
        .unwrap();
    assert_eq!(names_of(&edit).len(), 1);
}

#[test]
fn the_name_that_would_be_the_65537th_is_refused_and_a_replacement_is_not() {
    let names = defined(&[("First", "", "1")]);
    let mut edit = open(&["A"], &names);
    let entry = || NameEntry {
        written: "Filler".into(),
        scope: Scope::Book,
        text: Text::Changed("1".into()),
        origin: None,
        name_changed: false,
        unresolved: None,
    };
    edit.state.names.extend((1..65_536).map(|_| entry()));
    assert_eq!(edit.state.names.len(), 65_536);
    let before = fingerprint(&edit);
    assert_eq!(
        edit.set_defined_name("Last", None, "1"),
        Err(StructureError::DefinedNamesTooMany)
    );
    assert_eq!(fingerprint(&edit), before);
    edit.set_defined_name("first", None, "2").unwrap();
    assert_eq!(edit.state.names.len(), 65_536);
}

#[test]
fn a_name_is_removed_by_its_scope_and_spelling_letter_case_aside() {
    let names = defined(&[
        ("Total", "", "A!$A$1"),
        ("Total", "1", "B!$A$1"),
        ("Keep", "", "1"),
    ]);
    let mut edit = open(&["A", "B"], &names);
    edit.remove_defined_name("TOTAL", Some("b")).unwrap();
    assert_eq!(
        edit.state.removed_names.iter().copied().collect::<Vec<_>>(),
        [1]
    );
    edit.remove_defined_name("total", None).unwrap();
    assert_eq!(
        edit.state.removed_names.iter().copied().collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(names_of(&edit).len(), 1);
    // A name this edit added leaves no original name to remove.
    edit.set_defined_name("Fresh", None, "1").unwrap();
    edit.remove_defined_name("fresh", None).unwrap();
    assert_eq!(edit.state.removed_names.len(), 2);
    assert_eq!(names_of(&edit).len(), 1);
}

#[test]
fn a_remove_that_fails_changes_nothing() {
    let names = defined(&[("Total", "", "A!$A$1"), ("Raw", "9", "1")]);
    let mut edit = open(&["A"], &names);
    let before = fingerprint(&edit);
    assert_eq!(
        edit.remove_defined_name("Total", Some("Z")),
        Err(StructureError::SheetMissing { name: "Z".into() })
    );
    assert_eq!(
        edit.remove_defined_name("Total", Some("A")),
        Err(StructureError::DefinedNameMissing {
            name: "Total".into()
        })
    );
    assert_eq!(
        edit.remove_defined_name("Nope", None),
        Err(StructureError::DefinedNameMissing {
            name: "Nope".into()
        })
    );
    // A name of a sheet number the workbook does not have is in no scope a
    // caller can name.
    assert_eq!(
        edit.remove_defined_name("Raw", None),
        Err(StructureError::DefinedNameMissing { name: "Raw".into() })
    );
    assert_eq!(fingerprint(&edit), before);
}

#[test]
fn a_local_sheet_id_equal_to_the_number_of_sheets_is_kept_as_a_raw_scope() {
    let names = defined(&[("Last", "1", "1"), ("Past", "2", "1")]);
    let edit = open(&["A", "B"], &names);
    let scopes: Vec<Scope> = names_of(&edit)
        .into_iter()
        .map(|(_, scope, _)| scope)
        .collect();
    assert_eq!(scopes, [Scope::Sheet(1), Scope::Raw(2)]);
}

#[test]
fn an_empty_formula_is_malformed() {
    let mut edit = open(&["A"], "");
    let before = fingerprint(&edit);
    for formula in ["", "  ", "\t\n"] {
        assert_eq!(
            edit.set_defined_name("N", None, formula),
            Err(StructureError::FormulaMalformed {
                formula: formula.into()
            })
        );
    }
    assert_eq!(fingerprint(&edit), before);
}

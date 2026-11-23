//! `check_defined_name`: each rule at its boundary, and the order they
//! are checked in.

use crate::structure::{DefinedNameFault, StructureError, check_defined_name};

/// The error for `name` breaking the rule `reason`.
fn invalid(name: &str, reason: DefinedNameFault) -> Result<(), StructureError> {
    Err(StructureError::DefinedNameInvalid {
        name: name.to_owned(),
        reason,
    })
}

#[test]
fn a_name_a_formula_can_name_is_accepted_in_either_scope() {
    let units_255 = "a".repeat(255);
    // 127 pairs are 254 units, and one more letter 255.
    let pairs_127 = format!("{}a", "😀".repeat(127));
    for name in [
        "Total",
        "_x",
        "\\ab",
        "a\\",
        "a.b",
        "a1b",
        "売上・前年",
        "a１",
        // Of the form of a cell reference past the columns it may have.
        "ABCD1",
        "Rx",
        "Cx",
        "AB",
        "A1B",
        "TRUEx",
        "_xlnmx",
        "_xlnm",
        "_xlfnx",
        "xlfn.a",
        "_xlpm",
        units_255.as_str(),
        pairs_127.as_str(),
    ] {
        for scoped in [false, true] {
            assert_eq!(check_defined_name(name, scoped, []), Ok(()), "{name:?}");
        }
    }
}

#[test]
fn each_rule_a_name_breaks_is_named() {
    let units_256 = "a".repeat(256);
    let pairs_128 = "😀".repeat(128);
    let cases = [
        ("", DefinedNameFault::Character),
        ("1a", DefinedNameFault::Character),
        ("a b", DefinedNameFault::Character),
        ("a-b", DefinedNameFault::Character),
        ("a?", DefinedNameFault::Character),
        ("a?b", DefinedNameFault::Character),
        ("a.b?c", DefinedNameFault::Character),
        ("?a", DefinedNameFault::Character),
        ("\\", DefinedNameFault::Character),
        ("\\a", DefinedNameFault::Character),
        ("\\x", DefinedNameFault::Character),
        ("\\é", DefinedNameFault::Character),
        ("a\u{3000}b", DefinedNameFault::Character),
        ("１a", DefinedNameFault::Character),
        ("a\u{fffe}", DefinedNameFault::Xml),
        ("a\u{ffff}", DefinedNameFault::Xml),
        (units_256.as_str(), DefinedNameFault::TooLong),
        // Each pair is two units: 128 are 256.
        (pairs_128.as_str(), DefinedNameFault::TooLong),
        ("A1", DefinedNameFault::Reference),
        ("a1", DefinedNameFault::Reference),
        ("XFD1", DefinedNameFault::Reference),
        ("R", DefinedNameFault::Reference),
        ("r", DefinedNameFault::Reference),
        ("C", DefinedNameFault::Reference),
        ("c", DefinedNameFault::Reference),
        ("RC", DefinedNameFault::Reference),
        ("R1C1", DefinedNameFault::Reference),
        ("r2c", DefinedNameFault::Reference),
        ("R1", DefinedNameFault::Reference),
        ("C12", DefinedNameFault::Reference),
        ("TRUE", DefinedNameFault::Boolean),
        ("false", DefinedNameFault::Boolean),
        ("False", DefinedNameFault::Boolean),
        ("_xlnm.Foo", DefinedNameFault::ReservedPrefix),
        ("_xlnm.Database", DefinedNameFault::ReservedPrefix),
        ("_XLNM.Foo", DefinedNameFault::ReservedPrefix),
        ("_xlnm.", DefinedNameFault::ReservedPrefix),
        ("_xlfn.a", DefinedNameFault::ReservedPrefix),
        ("_xlws.a", DefinedNameFault::ReservedPrefix),
        ("_xludf.a", DefinedNameFault::ReservedPrefix),
        ("_xleta.a", DefinedNameFault::ReservedPrefix),
        ("_xlpm.a", DefinedNameFault::ReservedPrefix),
        ("_XLFN.a", DefinedNameFault::ReservedPrefix),
        ("_xLpM.a", DefinedNameFault::ReservedPrefix),
    ];
    for (name, reason) in cases {
        assert_eq!(
            check_defined_name(name, true, []),
            invalid(name, reason),
            "{name:?}"
        );
    }
}

#[test]
fn the_database_built_in_name_is_refused_in_either_scope() {
    for scoped in [false, true] {
        assert_eq!(
            check_defined_name("_xlnm.Database", scoped, []),
            invalid("_xlnm.Database", DefinedNameFault::ReservedPrefix)
        );
    }
}

#[test]
fn a_built_in_name_is_held_by_a_sheet_only() {
    for name in [
        "_xlnm.Print_Area",
        "_xlnm.Print_Titles",
        "_xlnm._FilterDatabase",
        "_xlnm.Criteria",
        "_xlnm.Extract",
        "_xlnm.Consolidate_Area",
        "_xlnm.Sheet_Title",
        "_XLNM.print_area",
    ] {
        assert_eq!(check_defined_name(name, true, []), Ok(()), "{name}");
        assert_eq!(
            check_defined_name(name, false, []),
            invalid(name, DefinedNameFault::BuiltinScope),
            "{name}"
        );
    }
}

#[test]
fn the_first_rule_in_order_is_the_one_named() {
    let long = format!("1{}", "a".repeat(255));
    assert_eq!(
        check_defined_name(&long, false, ["A"]),
        invalid(&long, DefinedNameFault::Character)
    );
    assert_eq!(
        check_defined_name("_xlnm.Foo", false, ["_xlnm.foo"]),
        invalid("_xlnm.Foo", DefinedNameFault::ReservedPrefix)
    );
    let long_reference = format!("A{}", "1".repeat(255));
    assert_eq!(
        check_defined_name(&long_reference, false, []),
        invalid(&long_reference, DefinedNameFault::TooLong)
    );
    let long_prefix = format!("_xlfn.{}", "a".repeat(250));
    assert_eq!(
        check_defined_name(&long_prefix, false, []),
        invalid(&long_prefix, DefinedNameFault::TooLong)
    );
    let long_xml = format!("{}\u{fffe}", "a".repeat(255));
    assert_eq!(
        check_defined_name(&long_xml, false, []),
        invalid(&long_xml, DefinedNameFault::Xml)
    );
    assert_eq!(
        check_defined_name("A1", false, ["a1"]),
        invalid("A1", DefinedNameFault::Reference)
    );
    assert_eq!(
        check_defined_name("_xlnm.Print_Area", false, ["_XLNM.PRINT_AREA"]),
        invalid("_xlnm.Print_Area", DefinedNameFault::BuiltinScope)
    );
}

#[test]
fn a_name_a_table_has_is_refused_letter_case_aside() {
    let table = |name: &str, table: &str| {
        Err(StructureError::DefinedNameTable {
            name: name.to_owned(),
            table: table.to_owned(),
        })
    };
    for scoped in [false, true] {
        assert_eq!(
            check_defined_name("Sales", scoped, ["Other", "sales", "SALES"]),
            table("Sales", "sales")
        );
        assert_eq!(check_defined_name("Sales", scoped, ["Sales2"]), Ok(()));
    }
}

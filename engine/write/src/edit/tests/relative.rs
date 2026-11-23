//! The relative reference a relationship's `Target` is: the part name from
//! the folder of the relationship's source, on plain values.

use super::part;
use crate::LinkSource;
use crate::edit::link::relative;

fn from_part(source: &str, target: &str) -> String {
    relative(&LinkSource::Part(part(source)), &part(target))
}

#[test]
fn a_target_is_the_part_name_without_the_folders_it_shares_with_the_source() {
    assert_eq!(
        from_part("/xl/workbook.xml", "/xl/worksheets/sheet2.xml"),
        "worksheets/sheet2.xml"
    );
    assert_eq!(
        from_part("/xl/workbook.xml", "/xl/styles.xml"),
        "styles.xml"
    );
    assert_eq!(
        from_part("/xl/worksheets/sheet1.xml", "/xl/worksheets/sheet2.xml"),
        "sheet2.xml"
    );
}

#[test]
fn a_target_climbs_out_of_the_folders_of_the_source_it_does_not_share() {
    assert_eq!(from_part("/xl/a/b.xml", "/xl/c.xml"), "../c.xml");
    assert_eq!(
        from_part("/xl/workbook.xml", "/docProps/app.xml"),
        "../docProps/app.xml"
    );
    assert_eq!(from_part("/xl/a/b/c.xml", "/xl/d/e.xml"), "../../d/e.xml");
    // A folder named as the file of the target is not shared.
    assert_eq!(from_part("/a/b.xml", "/a"), "../a");
}

#[test]
fn a_part_at_the_root_and_the_package_have_no_folder_to_climb_out_of() {
    assert_eq!(
        relative(&LinkSource::Package, &part("/xl/workbook.xml")),
        "xl/workbook.xml"
    );
    assert_eq!(relative(&LinkSource::Package, &part("/a.xml")), "a.xml");
    assert_eq!(from_part("/a.xml", "/b/c.xml"), "b/c.xml");
    assert_eq!(from_part("/a.xml", "/b.xml"), "b.xml");
}

#[test]
fn a_name_stays_in_its_iri_form() {
    assert_eq!(
        from_part("/xl/workbook.xml", "/xl/シート/一.xml"),
        "シート/一.xml"
    );
    assert_eq!(from_part("/xl/workbook.xml", "/xl/a%20b.xml"), "a%20b.xml");
}

#[test]
fn a_first_segment_with_a_colon_is_written_after_a_dot_segment_so_it_is_not_a_scheme() {
    assert_eq!(
        from_part("/xl/workbook.xml", "/xl/a:b/c.xml"),
        "./a:b/c.xml"
    );
    assert_eq!(from_part("/xl/workbook.xml", "/xl/a:b.xml"), "./a:b.xml");
    assert_eq!(
        relative(&LinkSource::Package, &part("/a:b.xml")),
        "./a:b.xml"
    );
    // Only the first segment is read as a scheme, and `..` come before it.
    assert_eq!(from_part("/xl/workbook.xml", "/x/a:b.xml"), "../x/a:b.xml");
    assert_eq!(from_part("/xl/workbook.xml", "/xl/c/a:b.xml"), "c/a:b.xml");
}

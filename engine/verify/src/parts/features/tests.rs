//! Unit tests for finding features: the roles of parts, the names
//! [MS-XLSX] gives each, letter case, namespaces, the attributes that
//! decide and the role an element counts in.

use super::{Role, element, features, media_type, relationship_type};
use std::collections::BTreeSet;
use xlsx_graft_compat::Feature;
use xlsx_graft_xml::events::BytesStart;

/// The x14 namespace.
const X14: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
/// The x15 namespace.
const X15: &str = "http://schemas.microsoft.com/office/spreadsheetml/2010/11/main";
/// The dynamic array namespace.
const XDA: &str = "http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray";
/// The rich data namespace.
const RD: &str = "http://schemas.microsoft.com/office/spreadsheetml/2017/richdata";

/// What the element written `content` (its name and attributes) in
/// `namespace` shows.
fn shows(namespace: &str, content: &str) -> Option<Feature> {
    let name = content.split(' ').next().unwrap();
    let start = BytesStart::from_content(content, name.len());
    element(namespace, name, &start)
}

#[test]
fn a_media_type_names_the_role_of_its_part() {
    let cases = [
        (
            "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
            Some(Role::Worksheet),
        ),
        (
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheetmetadata+xml",
            Some(Role::Metadata),
        ),
        (
            "application/vnd.ms-excel.rdrichvaluestructure+xml",
            Some(Role::RichValueStructure),
        ),
        (
            "application/vnd.ms-excel.threadedcomments+xml",
            Some(Role::ThreadedComments),
        ),
        (
            "application/vnd.ms-excel.slicercache+xml",
            Some(Role::SlicerCache),
        ),
        (
            "application/vnd.ms-excel.timelinecache+xml",
            Some(Role::TimelineCache),
        ),
        (
            "application/vnd.openxmlformats-officedocument.model+data",
            Some(Role::DataModel),
        ),
        ("application/vnd.ms-excel.slicer+xml", None),
        ("application/vnd.ms-excel.timeline+xml", None),
        ("application/vnd.ms-excel.person+xml", None),
        ("application/xml", None),
    ];
    for (essence, role) in cases {
        assert_eq!(media_type(essence), role, "{essence}");
    }
}

#[test]
fn a_relationship_type_names_the_role_of_its_target_in_any_case() {
    let cases = [
        (
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet",
            Some(Role::Worksheet),
        ),
        (
            "http://purl.oclc.org/ooxml/officeDocument/relationships/sheetMetadata",
            Some(Role::Metadata),
        ),
        (
            "HTTP://SCHEMAS.OPENXMLFORMATS.ORG/OFFICEDOCUMENT/2006/RELATIONSHIPS/SHEETMETADATA",
            Some(Role::Metadata),
        ),
        (
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
            None,
        ),
        (
            "http://schemas.microsoft.com/office/2017/06/relationships/rdRichValueStructure",
            Some(Role::RichValueStructure),
        ),
        (
            "http://schemas.microsoft.com/office/2017/10/relationships/threadedComment",
            Some(Role::ThreadedComments),
        ),
        (
            "http://schemas.microsoft.com/office/2007/relationships/slicerCache",
            Some(Role::SlicerCache),
        ),
        (
            "HTTP://SCHEMAS.MICROSOFT.COM/OFFICE/2010/RELATIONSHIPS/TIMELINECACHE",
            Some(Role::TimelineCache),
        ),
        (
            "http://schemas.microsoft.com/office/2007/relationships/slicer",
            None,
        ),
        (
            "http://schemas.microsoft.com/office/2017/10/relationships/person",
            None,
        ),
    ];
    for (rel_type, role) in cases {
        assert_eq!(relationship_type(rel_type), role, "{rel_type}");
    }
}

#[test]
fn an_element_names_a_feature_in_its_own_namespace_only() {
    assert_eq!(shows(X14, "sparkline"), Some(Feature::Sparkline));
    assert_eq!(shows(X15, "tableSlicerCache"), Some(Feature::TableSlicer));
    assert_eq!(shows(X15, "sparkline"), None);
    assert_eq!(shows(X14, "tableSlicerCache"), None);
    assert_eq!(shows(X14, "sparklineGroups"), None);
}

#[test]
fn dynamic_array_properties_count_when_f_dynamic_is_true() {
    for on in ["1", "true", " 1 "] {
        let content = format!("dynamicArrayProperties fDynamic=\"{on}\" fCollapsed=\"0\"");
        assert_eq!(shows(XDA, &content), Some(Feature::DynamicArray), "{on}");
    }
    for off in [
        "dynamicArrayProperties fDynamic=\"0\"",
        "dynamicArrayProperties fDynamic=\"false\"",
        "dynamicArrayProperties fCollapsed=\"1\"",
        "dynamicArrayProperties x:fDynamic=\"1\"",
        "dynamicArrayProperties",
    ] {
        assert_eq!(shows(XDA, off), None, "{off}");
    }
    assert_eq!(shows(X14, "dynamicArrayProperties fDynamic=\"1\""), None);
}

#[test]
fn a_rich_value_structure_of_the_local_image_type_is_a_cell_image() {
    assert_eq!(shows(RD, "s t=\"_localImage\""), Some(Feature::CellImage));
    assert_eq!(shows(RD, "s t=\"_webimage\""), None);
    assert_eq!(shows(RD, "s"), None);
    assert_eq!(shows(X14, "s t=\"_localImage\""), None);
    assert_eq!(shows(RD, "k t=\"_localImage\""), None);
}

#[test]
fn an_element_counts_only_in_the_role_it_extends() {
    let all = || {
        BTreeSet::from([
            Feature::Sparkline,
            Feature::DynamicArray,
            Feature::CellImage,
            Feature::TableSlicer,
        ])
    };
    let of = |roles: &[Role]| features(&roles.iter().copied().collect(), all());
    assert_eq!(of(&[]), BTreeSet::new());
    assert_eq!(of(&[Role::Worksheet]), BTreeSet::from([Feature::Sparkline]));
    assert_eq!(
        of(&[Role::Metadata]),
        BTreeSet::from([Feature::DynamicArray])
    );
    assert_eq!(
        of(&[Role::RichValueStructure]),
        BTreeSet::from([Feature::CellImage])
    );
    assert_eq!(
        of(&[Role::SlicerCache]),
        BTreeSet::from([Feature::TableSlicer])
    );
    let parts = [
        (Role::SlicerCache, Feature::Slicer),
        (Role::TimelineCache, Feature::Timeline),
        (Role::ThreadedComments, Feature::ThreadedComment),
        (Role::DataModel, Feature::DataModel),
    ];
    for (role, feature) in parts {
        let shown = BTreeSet::from([Feature::Slicer, Feature::Timeline]);
        assert_eq!(
            features(&BTreeSet::from([role]), shown),
            BTreeSet::from([feature]),
            "{role:?}"
        );
    }
}

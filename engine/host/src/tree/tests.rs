//! Unit tests for the tree view: the JSON of the workbooks Excel saved in
//! `fixtures/`, and of packages the tests build with what Excel does not
//! write: a part nothing targets, an entry the reader refuses, characters
//! unfit to display.

use super::{tree, tree_bytes, tree_path};
use crate::policy::limits::HostLimits;
use crate::support::{
    CONTENT_TYPES, FIXTURES, json, main_rels, package, rel, rel_of, rels, zip, zip_with_methods,
};
use serde_json::Value;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use xlsx_graft_diagnostics::Limit;

const MAIN: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// The JSON of the tree of the fixture `name`.
fn fixture(name: &str) -> Value {
    let view = tree_path(&Path::new(FIXTURES).join(name), &HostLimits::default());
    json(&view.to_json().unwrap())
}

/// The JSON of the tree of the package `bytes` holds.
fn of_bytes(bytes: &[u8]) -> Value {
    json(&tree_bytes(bytes, &HostLimits::default()).to_json().unwrap())
}

/// The part named `name` of a tree's JSON.
fn part<'a>(value: &'a Value, name: &str) -> &'a Value {
    (value["parts"].as_array().unwrap().iter())
        .find(|part| part["name"] == name)
        .unwrap()
}

/// What each relationship of `list` is, as `<id> <type's last segment> <target>`.
fn listed(list: &Value) -> Vec<String> {
    (list.as_array().unwrap().iter())
        .map(|relationship| {
            let kind = relationship["type"].as_str().unwrap();
            let target = &relationship["target"];
            format!(
                "{} {} {}",
                relationship["id"].as_str().unwrap(),
                kind.rsplit('/').next().unwrap(),
                target
            )
        })
        .collect()
}

/// The code of each diagnostic of a view's JSON.
fn codes(value: &Value) -> Vec<&str> {
    let listed = value["diagnostics"].as_array().unwrap();
    listed.iter().map(|d| d["code"].as_str().unwrap()).collect()
}

#[test]
fn the_tree_of_a_workbook_lists_its_parts_and_each_relationship() {
    let value = fixture("table.xlsx");
    assert_eq!(
        (value["ok"].as_bool(), value["complete"].as_bool()),
        (Some(true), Some(true))
    );
    assert_eq!(value.get("error"), Some(&Value::Null));
    assert_eq!(
        listed(&value["relationships"]),
        [
            r#"rId3 extended-properties {"part":"/docProps/app.xml"}"#,
            r#"rId2 core-properties {"part":"/docProps/core.xml"}"#,
            r#"rId1 officeDocument {"part":"/xl/workbook.xml"}"#,
        ]
    );
    let workbook = part(&value, "/xl/workbook.xml");
    assert_eq!(
        listed(&workbook["relationships"]),
        [
            r#"rId3 styles {"part":"/xl/styles.xml"}"#,
            r#"rId2 theme {"part":"/xl/theme/theme1.xml"}"#,
            r#"rId1 worksheet {"part":"/xl/worksheets/sheet1.xml"}"#,
            r#"rId5 calcChain {"part":"/xl/calcChain.xml"}"#,
            r#"rId4 sharedStrings {"part":"/xl/sharedStrings.xml"}"#,
        ]
    );
    let sheet = part(&value, "/xl/worksheets/sheet1.xml");
    assert_eq!(
        listed(&sheet["relationships"]),
        [r#"rId1 table {"part":"/xl/tables/table1.xml"}"#]
    );
    assert_eq!(
        (&workbook["contentType"], &workbook["contentTypeKnown"]),
        (
            &json(
                r#""application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml""#
            ),
            &Value::Bool(true)
        )
    );
    assert_eq!(
        (
            workbook["readable"].as_bool(),
            workbook["referenced"].as_bool()
        ),
        (Some(true), Some(true))
    );
    assert_eq!(workbook["relationshipsKnown"], true);
    assert!(workbook["size"].as_u64().unwrap() > 0);
    assert!(workbook["compressedSize"].as_u64().unwrap() > 0);
    // The parts are in the order of their zip entries.
    let names: Vec<&str> = (value["parts"].as_array().unwrap().iter())
        .map(|part| part["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "/xl/workbook.xml",
            "/xl/worksheets/sheet1.xml",
            "/xl/theme/theme1.xml",
            "/xl/styles.xml",
            "/xl/sharedStrings.xml",
            "/xl/tables/table1.xml",
            "/xl/calcChain.xml",
            "/docProps/core.xml",
            "/docProps/app.xml",
        ]
    );
}

#[test]
fn the_tree_has_ok_error_and_diagnostics_and_then_complete_relationships_and_parts() {
    let view = tree_path(
        &Path::new(FIXTURES).join("table.xlsx"),
        &HostLimits::default(),
    );
    let text = view.to_json().unwrap();
    assert!(
        text.starts_with(r#"{"ok":true,"error":null,"diagnostics":[],"complete":true,"relationshipsKnown":true,"relationships":[{"id":"rId3","type":"http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties","target":{"part":"/docProps/app.xml"}}"#),
        "{text}"
    );
    // One part, with every key in its order.
    let sheet = concat!(
        r#"{"name":"/xl/worksheets/sheet1.xml","#,
        r#""contentType":"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml","#,
        r#""contentTypeKnown":true,"size":1877,"compressedSize":700,"readable":true,"#,
        r#""referenced":true,"relationshipsKnown":true,"relationships":["#,
        r#"{"id":"rId1","type":"http://schemas.openxmlformats.org/officeDocument/2006/relationships/table","#,
        r#""target":{"part":"/xl/tables/table1.xml"}}]}"#,
    );
    assert!(text.contains(sheet), "{text}");
}

#[test]
fn an_external_target_is_written_under_external() {
    let value = fixture("external-link.xlsx");
    let link = part(&value, "/xl/externalLinks/externalLink1.xml");
    assert_eq!(
        listed(&link["relationships"]),
        [r#"rId1 externalLinkPath {"external":"external-source.xlsx"}"#]
    );
}

#[test]
fn bytes_that_are_not_a_zip_are_an_empty_tree_that_is_not_ok_and_has_no_error() {
    let value = of_bytes(b"this is not a zip");
    assert_eq!(value["ok"], false);
    assert_eq!(value.get("error"), Some(&Value::Null));
    assert_eq!(value["complete"], false);
    assert_eq!(value["relationshipsKnown"], false);
    assert_eq!(value["parts"], json("[]"));
    assert_eq!(value["relationships"], json("[]"));
    assert_eq!(codes(&value), ["not_a_zip"]);
}

#[test]
fn a_path_that_is_missing_or_not_a_file_is_an_io_error_and_an_empty_tree() {
    let limits = HostLimits::default();
    let folder = tempfile::tempdir().unwrap();
    for (path, said) in [
        (folder.path().join("missing.xlsx"), "could not be opened"),
        (folder.path().to_path_buf(), "is not a regular file"),
    ] {
        let value = json(&tree_path(&path, &limits).to_json().unwrap());
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "io");
        assert!(value["error"]["message"].as_str().unwrap().contains(said));
        assert_eq!(value["diagnostics"], json("[]"));
        assert_eq!(value["parts"], json("[]"));
        assert_eq!(value["complete"], false);
        assert_eq!(value["relationshipsKnown"], false);
    }
}

/// A reader whose every read and seek returns `Err`.
struct Gone;

impl Read for Gone {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("the disk went away"))
    }
}

impl Seek for Gone {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::other("the disk went away"))
    }
}

#[test]
fn a_reader_that_fails_is_an_io_error_and_an_empty_tree() {
    let view = tree(Gone, &HostLimits::default());
    let value = json(&view.to_json().unwrap());
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "io");
    assert_eq!(
        value["error"]["message"],
        "the package could not be read: the disk went away"
    );
    assert_eq!(value["parts"], json("[]"));
}

#[test]
fn a_part_no_relationship_targets_is_not_referenced_when_all_were_read() {
    let main = main_rels();
    let value = of_bytes(&package(&[
        ("_rels/.rels", &main),
        ("xl/workbook.xml", "<a/>"),
        ("xl/orphan.xml", "<a/>"),
    ]));
    assert_eq!(value["complete"], true);
    assert_eq!(part(&value, "/xl/workbook.xml")["referenced"], true);
    assert_eq!(part(&value, "/xl/orphan.xml")["referenced"], false);
    assert_eq!(part(&value, "/xl/orphan.xml")["relationshipsKnown"], true);
}

#[test]
fn a_part_without_a_media_type_has_a_null_content_type_that_is_known_and_the_declared_sizes() {
    let value = of_bytes(&package(&[("xl/data.bin", "abc")]));
    // A part whose extension no `Default` maps and no `Override` names has
    // no media type, and that is known.
    let data = part(&value, "/xl/data.bin");
    assert_eq!(data.get("contentType"), Some(&Value::Null));
    assert_eq!(data["contentTypeKnown"], true);
    assert_eq!(data["size"], 3);
    assert_eq!(data["compressedSize"], 3);
}

#[test]
fn when_one_relationships_part_was_not_read_a_part_a_read_relationship_targets_is_still_referenced()
{
    let main = main_rels();
    let value = json(
        &tree_bytes(
            &zip_with_methods(&[
                ("[Content_Types].xml", CONTENT_TYPES, 0),
                ("_rels/.rels", &main, 0),
                ("xl/workbook.xml", "<a/>", 0),
                ("xl/_rels/workbook.xml.rels", &rels(""), 99),
                ("xl/orphan.xml", "<a/>", 0),
            ]),
            &HostLimits::default(),
        )
        .to_json()
        .unwrap(),
    );
    assert_eq!(value["complete"], false);
    assert_eq!(part(&value, "/xl/workbook.xml")["referenced"], true);
    assert_eq!(
        part(&value, "/xl/workbook.xml")["relationshipsKnown"],
        false
    );
    assert_eq!(
        part(&value, "/xl/orphan.xml").get("referenced"),
        Some(&Value::Null)
    );
}

#[test]
fn a_package_without_its_content_types_stream_has_parts_of_unknown_type() {
    let main = main_rels();
    let value = of_bytes(&zip(&[("_rels/.rels", &main), ("xl/workbook.xml", "<a/>")]));
    let workbook = part(&value, "/xl/workbook.xml");
    assert_eq!(workbook.get("contentType"), Some(&Value::Null));
    assert_eq!(workbook["contentTypeKnown"], false);
}

#[test]
fn an_entry_the_reader_refused_is_a_part_that_is_not_readable() {
    let main = main_rels();
    let value = json(
        &tree_bytes(
            &zip_with_methods(&[
                ("[Content_Types].xml", "<Types/>", 0),
                ("_rels/.rels", &main, 0),
                ("xl/workbook.xml", "<a/>", 99),
            ]),
            &HostLimits::default(),
        )
        .to_json()
        .unwrap(),
    );
    let workbook = part(&value, "/xl/workbook.xml");
    assert_eq!(workbook["readable"], false);
    assert_eq!(workbook["size"], 4);
    assert!(codes(&value).contains(&"zip_unsupported_compression"));
}

#[test]
fn a_relationships_part_that_was_not_read_leaves_whether_a_part_is_referenced_unknown() {
    let main = main_rels();
    let value = json(
        &tree_bytes(
            &zip_with_methods(&[
                ("[Content_Types].xml", CONTENT_TYPES, 0),
                ("_rels/.rels", &main, 99),
                ("xl/workbook.xml", "<a/>", 0),
            ]),
            &HostLimits::default(),
        )
        .to_json()
        .unwrap(),
    );
    assert_eq!(value["complete"], false);
    assert_eq!(value["relationshipsKnown"], false);
    let workbook = part(&value, "/xl/workbook.xml");
    assert_eq!(workbook.get("referenced"), Some(&Value::Null));
}

#[test]
fn an_id_or_external_target_loses_bidirectional_formatting_characters_and_is_cut_to_200_characters()
{
    let long = "x".repeat(300);
    let body = format!(
        concat!(
            r#"<Relationship Id="r&#8238;1" Type="{main}" Target="xl/workbook.xml"/>"#,
            r#"<Relationship Id="e1" Type="urn:x/link" Target="http://a.example/&#8238;b" TargetMode="External"/>"#,
            r#"<Relationship Id="e2" Type="urn:x/link" Target="{long}" TargetMode="External"/>"#,
        ),
        main = MAIN,
        long = long
    );
    let value = of_bytes(&package(&[
        ("_rels/.rels", &rels(&body)),
        ("xl/workbook.xml", "<a/>"),
    ]));
    let relationships = value["relationships"].as_array().unwrap();
    assert_eq!(relationships[0]["id"], "r1");
    assert_eq!(relationships[1]["target"]["external"], "http://a.example/b");
    let clipped = relationships[2]["target"]["external"].as_str().unwrap();
    assert_eq!(clipped, format!("{}…", "x".repeat(200)));
}

#[test]
fn the_number_of_diagnostics_listed_is_the_limit_of_the_policy() {
    let main = main_rels();
    let bytes = package(&[
        ("_rels/.rels", &main),
        ("xl/workbook.xml", "<a/>"),
        ("xl/one.xml", "<a/>"),
        ("xl/two.xml", "<a/>"),
        ("xl/three.xml", "<a/>"),
    ]);
    let all = of_bytes(&bytes);
    assert_eq!(codes(&all), ["part_unreferenced"; 3]);
    let limits = HostLimits {
        max_diagnostics: Limit::default_of(1),
        ..HostLimits::default()
    };
    let value = json(&tree_bytes(&bytes, &limits).to_json().unwrap());
    assert_eq!(
        codes(&value),
        ["part_unreferenced", "diagnostics_truncated"]
    );
}

#[test]
fn a_relationship_whose_target_is_not_a_part_is_not_listed_and_its_part_says_so() {
    let body = format!(
        "{}{}",
        rel("rId1", "link", "missing.xml"),
        rel("rId2", "link", "ok.xml")
    );
    let value = of_bytes(&package(&[
        ("_rels/.rels", &main_rels()),
        ("xl/workbook.xml", "<a/>"),
        ("xl/ok.xml", "<a/>"),
        ("xl/_rels/workbook.xml.rels", &rels(&body)),
    ]));
    let workbook = part(&value, "/xl/workbook.xml");
    assert_eq!(
        listed(&workbook["relationships"]),
        [r#"rId2 link {"part":"/xl/ok.xml"}"#]
    );
    assert_eq!(workbook["relationshipsKnown"], false);
    assert_eq!(value["complete"], true);
    assert!(codes(&value).contains(&"relationship_target_missing"));
}

#[test]
fn the_relationships_of_a_relationships_part_without_a_source_are_not_listed_and_complete_is_false()
{
    let value = of_bytes(&package(&[
        ("_rels/.rels", &main_rels()),
        ("xl/workbook.xml", "<a/>"),
        ("xl/hidden.xml", "<a/>"),
        (
            "xl/_rels/nosuch.xml.rels",
            &rels(&rel("rId1", "link", "hidden.xml")),
        ),
    ]));
    assert_eq!(value["complete"], false);
    assert!(codes(&value).contains(&"relationships_source_missing"));
    assert_eq!(part(&value, "/xl/workbook.xml")["referenced"], true);
    assert_eq!(
        part(&value, "/xl/hidden.xml").get("referenced"),
        Some(&Value::Null)
    );
    let text = serde_json::to_string(&value).unwrap();
    assert!(!text.contains(r#""part":"/xl/hidden.xml""#), "{text}");
}

#[test]
fn a_relationship_that_targets_a_relationships_part_is_not_listed_and_the_graph_is_not_complete() {
    let body = format!(
        "{}{}",
        rel_of("rId1", MAIN, "xl/workbook.xml"),
        rel("rId2", "link", "_rels/workbook.xml.rels")
    );
    let value = of_bytes(&package(&[
        ("_rels/.rels", &rels(&body)),
        ("xl/workbook.xml", "<a/>"),
        ("xl/_rels/workbook.xml.rels", &rels("")),
    ]));
    assert_eq!(
        listed(&value["relationships"]),
        [r#"rId1 officeDocument {"part":"/xl/workbook.xml"}"#]
    );
    assert_eq!(value["complete"], false);
    assert!(codes(&value).contains(&"relationship_rels_part"));
}

#[test]
fn a_package_relationship_that_was_not_kept_makes_the_top_relationships_known_false() {
    let body = format!(
        "{}{}",
        rel_of("rId1", MAIN, "xl/workbook.xml"),
        rel("rId2", "link", "missing.xml")
    );
    let value = of_bytes(&package(&[
        ("_rels/.rels", &rels(&body)),
        ("xl/workbook.xml", "<a/>"),
    ]));
    assert_eq!(value["relationshipsKnown"], false);
    assert_eq!(value["complete"], true);
    assert_eq!(
        listed(&value["relationships"]),
        [r#"rId1 officeDocument {"part":"/xl/workbook.xml"}"#]
    );
}

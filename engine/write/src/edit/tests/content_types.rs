//! The rewrite of the content types stream, on plain text: which
//! `Override`s are left out and which are kept, for a stream of any shape
//! the walk over the children of the root meets.

use super::super::content_types::rewrite;
use super::TYPES_NS;
use crate::WriteError;
use std::collections::BTreeSet;
use xlsx_graft_xml::XmlLimits;

/// `stream` rewritten, leaving out the `Override`s whose key is in `drop` and
/// adding `/new.xml`.
fn rewritten(stream: &str, drop: &[&str]) -> Result<String, WriteError> {
    let drop: BTreeSet<String> = drop.iter().map(|key| (*key).to_owned()).collect();
    let add = [("/new.xml".to_owned(), "application/xml".to_owned())];
    let mut out = Vec::new();
    rewrite(
        &mut stream.as_bytes(),
        &mut out,
        &XmlLimits::default(),
        &drop,
        &add,
    )?;
    Ok(String::from_utf8(out).unwrap())
}

fn types(body: &str) -> String {
    format!(r#"<Types xmlns="{TYPES_NS}">{body}</Types>"#)
}

const ADDED: &str = r#"<Override PartName="/new.xml" ContentType="application/xml"/>"#;

#[test]
fn an_override_is_left_out_by_the_key_of_its_part_name_as_the_package_reads_it() {
    let body = concat!(
        r#"<Override PartName="/xl/A.xml" ContentType="a"/>"#,
        r#"<Override PartName="  /xl/b.xml
" ContentType="b"/>"#,
        // Not a part name: its key is the shadow key of the entry name.
        r#"<Override PartName="xl/c.xml" ContentType="c"/>"#,
        r#"<Override PartName="/xl/keep.xml" ContentType="k"/>"#,
    );
    let drop = ["/xl/a.xml", "/xl/b.xml", "/xl/c.xml"];
    assert_eq!(
        rewritten(&types(body), &drop).unwrap(),
        types(&format!(
            r#"<Override PartName="/xl/keep.xml" ContentType="k"/>{ADDED}"#
        ))
    );
}

#[test]
fn an_override_with_no_part_name_is_kept() {
    let body = r#"<Override ContentType="a"/>"#;
    assert_eq!(
        rewritten(&types(body), &["/xl/a.xml"]).unwrap(),
        types(&format!("{body}{ADDED}"))
    );
}

#[test]
fn only_an_override_that_is_a_child_of_the_root_in_its_namespace_is_left_out() {
    let ours = r#"PartName="/xl/a.xml" ContentType="application/xml""#;
    let others = format!(
        concat!(
            // The same part name, on an element that is not an `Override`.
            r#"<Default {ours}/>"#,
            // On an `Override` of another namespace.
            r#"<o:Override xmlns:o="urn:other" {ours}/>"#,
            // On an `Override` that is not a child of the root.
            r#"<Default Extension="a" ContentType="a"><Override {ours}/></Default>"#,
        ),
        ours = ours
    );
    // The one that is dropped has content: its end tag and what is inside go
    // with it, and the next sibling stays.
    let body = format!("<Override {ours}><x>text</x></Override>{others}");
    let kept = rewritten(&types(&body), &["/xl/a.xml"]).unwrap();
    assert_eq!(kept, types(&format!("{others}{ADDED}")));
}

#[test]
fn a_root_with_no_children_gets_the_override() {
    let empty = format!(r#"<Types xmlns="{TYPES_NS}"/>"#);
    assert_eq!(rewritten(&empty, &[]).unwrap(), types(ADDED));
}

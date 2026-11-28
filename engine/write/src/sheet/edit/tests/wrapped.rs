//! The look-ahead into an `mc:AlternateContent`: the kind it wraps places
//! the elements written new before it, any other wrapper is no kind, and a
//! wrapper is written as it was.

use super::{r, sheet, text};

pub(super) const MC: &str =
    r#"xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006""#;
const MERGES: &str = r#"<mergeCells count="1"><mergeCell ref="A1:B2"/></mergeCells>"#;

/// The sheet of `<sheetData/>` and the wrapper `content` (the children of
/// `mc:AlternateContent`), and what `merge` makes of it.
fn merged(content: &str) -> (String, String) {
    let wrapper = format!("<mc:AlternateContent {MC}>{content}</mc:AlternateContent>");
    let got = text(&sheet(&format!("<sheetData/>{wrapper}")), |edit| {
        edit.merge(r("A1:B2")).unwrap();
    });
    (got, wrapper)
}

#[test]
fn a_wrapper_of_a_kind_after_the_new_one_has_it_written_before() {
    let content = r#"<mc:Choice Requires="x14"><oleObjects><oleObject progId="x"/></oleObjects></mc:Choice><mc:Fallback><oleObjects/></mc:Fallback>"#;
    let (got, wrapper) = merged(content);
    assert_eq!(got, sheet(&format!("<sheetData/>{MERGES}{wrapper}")));
    // Text and comments before the first element, and an element with an end
    // tag.
    let content =
        "\n <!--c-->\n<mc:Choice Requires=\"x14\"> text <controls></controls></mc:Choice>";
    let (got, wrapper) = merged(content);
    assert_eq!(got, sheet(&format!("<sheetData/>{MERGES}{wrapper}")));
}

#[test]
fn any_other_wrapper_is_no_kind_and_the_new_element_goes_after_it() {
    for content in [
        r#"<mc:Choice Requires="x"><x:foo xmlns:x="urn:x"/></mc:Choice>"#,
        r#"<mc:Choice Requires="x"><unknownInMain/></mc:Choice>"#,
        r#"<mc:Fallback><oleObjects/></mc:Fallback>"#,
        r#"<mc:Choice Requires="x"/>"#,
        r#"<mc:Choice Requires="x"></mc:Choice>"#,
        "",
    ] {
        let (got, wrapper) = merged(content);
        assert_eq!(
            got,
            sheet(&format!("<sheetData/>{wrapper}{MERGES}")),
            "{content}"
        );
    }
    // An `mc:AlternateContent` with no end tag.
    let wrapper = format!("<mc:AlternateContent {MC}/>");
    let got = text(&sheet(&format!("<sheetData/>{wrapper}")), |edit| {
        edit.merge(r("A1:B2")).unwrap();
    });
    assert_eq!(got, sheet(&format!("<sheetData/>{wrapper}{MERGES}")));
}

#[test]
fn the_look_ahead_stops_after_256_events() {
    let ole = "<oleObjects/></mc:Choice>";
    for (comments, wraps) in [(254, true), (255, false)] {
        let content = format!(
            r#"<mc:Choice Requires="x">{}{ole}"#,
            "<!--c-->".repeat(comments)
        );
        let (got, wrapper) = merged(&content);
        let expected = if wraps {
            format!("<sheetData/>{MERGES}{wrapper}")
        } else {
            format!("<sheetData/>{wrapper}{MERGES}")
        };
        assert_eq!(got, sheet(&expected), "{comments}");
    }
}

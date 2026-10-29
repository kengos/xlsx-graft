//! A worksheet against its schema: the root, the children skipped unread,
//! elements, attributes and text where they may not stand, a child left
//! out, and markup compatibility: `mc:Ignorable`, and a
//! `mc:AlternateContent` right inside `worksheet` or deeper.

use super::super::NS;
use super::{book_of, briefs, clean, on_sheet, read, row, rows};

/// What reading a sheet whose XML is `xml` found, [`briefs`]ed.
fn refused(xml: &str) -> Vec<(String, String)> {
    let (rows, seen) = read(book_of(xml));
    assert_eq!(rows, vec![], "{xml}");
    briefs(&seen)
}

/// What reading a sheet whose `sheetData` holds `body` found, after the
/// rows it read.
fn found(body: &str) -> (usize, Vec<(String, String)>) {
    let (rows, seen) = rows(body);
    (rows.len(), briefs(&seen))
}

#[test]
fn a_root_of_another_name_or_namespace_is_refused() {
    let other = format!(r#"<chartsheet xmlns="{NS}"/>"#);
    assert_eq!(
        refused(&other),
        vec![on_sheet("schema:UnexpectedElement:chartsheet")]
    );
    let unbound = "<worksheet><sheetData/></worksheet>";
    assert_eq!(
        refused(unbound),
        vec![on_sheet("schema:UnexpectedElement:worksheet")]
    );
}

#[test]
fn a_worksheet_without_sheet_data_is_refused() {
    // Whether the children it has come before it or after.
    for children in [r#"<dimension ref="A1"/>"#, "<mergeCells/>", "<extLst/>", ""] {
        let xml = format!(r#"<worksheet xmlns="{NS}">{children}</worksheet>"#);
        let missing = vec![on_sheet("schema:MissingElement:sheetData")];
        assert_eq!(refused(&xml), missing, "{children}");
    }
}

#[test]
fn the_children_around_sheet_data_are_skipped_unread_in_their_order() {
    let xml = format!(
        concat!(
            r#"<worksheet xmlns="{}"><sheetPr><anything/></sheetPr><dimension ref="nonsense"/>"#,
            r#"<cols><col/></cols><cols/><sheetData><row><c><v>1</v></c></row></sheetData>"#,
            r#"<mergeCells><x/></mergeCells><conditionalFormatting/><conditionalFormatting/>"#,
            r#"<legacyDrawing/><tableParts/><extLst><ext/></extLst></worksheet>"#,
        ),
        NS
    );
    let (rows, seen) = read(book_of(&xml));
    assert_eq!((rows, seen), (vec![row(1, &["n:1"])], vec![]));
    // Out of order, twice, and one the schema does not have.
    for misplaced in [
        "<sheetData/><dimension/>",
        "<sheetData/><sheetData/>",
        "<sheetData/><extLst/><extLst/>",
        "<sheetData/><unknown/>",
    ] {
        let xml = format!(r#"<worksheet xmlns="{NS}">{misplaced}</worksheet>"#);
        let (rows, seen) = read(book_of(&xml));
        assert_eq!(rows, vec![]);
        assert_eq!(seen.len(), 1, "{misplaced}");
        assert!(super::brief(&seen[0].0).starts_with("schema:UnexpectedElement:"));
    }
}

#[test]
fn a_cell_s_children_come_in_their_order() {
    assert_eq!(
        clean(r#"<row><c><f>1+1</f><v>2</v><extLst/></c></row><row><extLst/></row>"#),
        vec![row(1, &["n:2"]), row(2, &[])]
    );
    for (body, name) in [
        ("<row><c><v>1</v><f/></c></row>", "f"),
        ("<row><c><v>1</v><v>1</v></c></row>", "v"),
        ("<row><extLst/><c/></row>", "c"),
        ("<row><c><x/></c></row>", "x"),
        ("<row><row/></row>", "row"),
        ("<c/>", "c"),
        (r#"<row><c t="inlineStr"><is><t/><t/></is></c></row>"#, "t"),
        (
            r#"<row><c t="inlineStr"><is><r><t/><rPr/></r></is></c></row>"#,
            "rPr",
        ),
    ] {
        let expected = format!("schema:UnexpectedElement:{name}");
        assert_eq!(found(body), (0, vec![on_sheet(&expected)]), "{body}");
    }
}

#[test]
fn a_run_without_its_text_is_refused() {
    let body = r#"<row><c t="inlineStr"><is><r><rPr/></r></is></c></row>"#;
    assert_eq!(found(body), (0, vec![on_sheet("schema:MissingElement:t")]));
}

#[test]
fn an_attribute_the_schema_does_not_give_is_refused() {
    for (body, name) in [
        (r#"<row><c x="1"/></row>"#, "x"),
        (r#"<row x="1"/>"#, "x"),
        (r#"<row><c><v a="1">1</v></c></row>"#, "a"),
        (r#"<row><c t="inlineStr"><is a="1"/></c></row>"#, "a"),
    ] {
        let expected = format!("schema:UnexpectedAttribute:{name}");
        assert_eq!(found(body), (0, vec![on_sheet(&expected)]), "{body}");
    }
    let xml = format!(r#"<worksheet xmlns="{NS}" x="1"><sheetData/></worksheet>"#);
    assert_eq!(
        refused(&xml),
        vec![on_sheet("schema:UnexpectedAttribute:x")]
    );
    // The values of the attributes not read are not looked at.
    let body = r#"<row spans="x" ht="x" hidden="x"><c cm="x" vm="x" ph="x"><v>1</v></c></row>"#;
    assert_eq!(clean(body), vec![row(1, &["n:1"])]);
}

#[test]
fn text_where_only_elements_stand_is_refused() {
    for (body, name) in [
        ("x", "sheetData"),
        ("<row>x</row>", "row"),
        ("<row><c>x</c></row>", "c"),
        (r#"<row><c t="inlineStr"><is>x</is></c></row>"#, "is"),
        (
            r#"<row><c t="inlineStr"><is><r>x<t/></r></is></c></row>"#,
            "r",
        ),
        ("<row>&#32;</row>", "row"),
        ("<row><![CDATA[ ]]></row>", "row"),
    ] {
        let expected = format!("schema:UnexpectedText:{name}");
        assert_eq!(found(body), (0, vec![on_sheet(&expected)]), "{body}");
    }
    let xml = format!(r#"<worksheet xmlns="{NS}">x<sheetData/></worksheet>"#);
    assert_eq!(
        refused(&xml),
        vec![on_sheet("schema:UnexpectedText:worksheet")]
    );
    // White space written as such, and an empty CDATA section, are not.
    assert_eq!(
        clean("\n <row>\t<![CDATA[]]><c> </c></row>\r\n"),
        vec![row(1, &[""])]
    );
}

/// The markup compatibility namespace.
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
/// A namespace Excel names ignorable.
const X14AC: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/ac";

#[test]
fn what_mc_ignorable_names_is_passed_over() {
    let xml = format!(
        concat!(
            r#"<worksheet xmlns="{}" xmlns:mc="{}" xmlns:x14ac="{}" mc:Ignorable="x14ac" x14ac:a="1">"#,
            r#"<sheetData><row x14ac:dyDescent="0.25"><x14ac:any><c/></x14ac:any>"#,
            r#"<c><v>1</v></c></row></sheetData></worksheet>"#,
        ),
        NS, MC, X14AC
    );
    let (rows, seen) = read(book_of(&xml));
    assert_eq!((rows, seen), (vec![row(1, &["n:1"])], vec![]));
    // Not named, it is refused.
    let body = format!(r#"<row xmlns:x14ac="{X14AC}" x14ac:dyDescent="1"/>"#);
    assert_eq!(
        found(&body),
        (
            0,
            vec![on_sheet("schema:UnexpectedAttribute:x14ac:dyDescent")]
        )
    );
    let body = format!(r#"<row xmlns:mc="{MC}" mc:Ignorable="none"/>"#);
    assert_eq!(
        found(&body),
        (0, vec![on_sheet("schema:InvalidValue:mc:Ignorable")])
    );
}

#[test]
fn alternate_content_is_skipped_right_inside_worksheet_and_refused_deeper() {
    let choice = r#"<mc:Choice Requires="x14"><anything/></mc:Choice><mc:Fallback/>"#;
    let xml = format!(
        r#"<worksheet xmlns="{NS}" xmlns:mc="{MC}"><sheetData/><mc:AlternateContent>{choice}</mc:AlternateContent><mc:AlternateContent/></worksheet>"#
    );
    assert_eq!(read(book_of(&xml)), (vec![], vec![]));
    let body = format!(r#"<row><mc:AlternateContent xmlns:mc="{MC}"/></row>"#);
    assert_eq!(
        found(&body),
        (
            0,
            vec![on_sheet("schema:UnexpectedElement:mc:AlternateContent")]
        )
    );
    let xml =
        format!(r#"<worksheet xmlns="{NS}" xmlns:mc="{MC}"><sheetData/><mc:Other/></worksheet>"#);
    assert_eq!(
        refused(&xml),
        vec![on_sheet("schema:UnexpectedElement:mc:Other")]
    );
}

#[test]
fn xml_the_reader_refuses_stops_after_the_rows_before() {
    let (rows, seen) = rows("<row><c><v>1</v></c></row><row></c></row>");
    assert_eq!(rows, vec![row(1, &["n:1"])]);
    assert_eq!(briefs(&seen), vec![on_sheet("xml_malformed")]);
}

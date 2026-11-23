//! Tests of [`StylesEdit`]: reading a styles part and what it refuses
//! (`scan`), adding cell formats (`assign`) and writing the part again
//! (`rewrite`). This file holds the parts they edit and the helpers.

mod assign;
mod rewrite;
mod scan;

use super::{StyleError, StyleLimits, StylesEdit};
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};

/// The namespace of SpreadsheetML, Transitional.
const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// A styles part in the shape Excel writes: a number format of its own, one
/// font, the two reserved fills, one border, one cell style, two cell
/// formats (the second has the number format) and a `cellStyles`.
fn base() -> String {
    let fills = concat!(
        r#"<fills count="2"><fill><patternFill patternType="none"/></fill>"#,
        r#"<fill><patternFill patternType="gray125"/></fill></fills>"#,
    );
    let xfs = concat!(
        r#"<cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>"#,
        r#"<xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/></cellXfs>"#,
    );
    part(&[
        r#"<numFmts count="1"><numFmt numFmtId="164" formatCode="0.000"/></numFmts>"#,
        r#"<fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>"#,
        fills,
        r#"<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>"#,
        r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>"#,
        xfs,
        r#"<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>"#,
        r#"<dxfs count="0"/>"#,
    ])
}

/// A styles part whose children are `children`.
fn part(children: &[&str]) -> String {
    format!(
        r#"<styleSheet xmlns="{NS}">{}</styleSheet>"#,
        children.concat()
    )
}

/// The part `xml` read under the default limits.
fn read(xml: &str) -> StylesEdit {
    StylesEdit::read(xml.as_bytes(), &StyleLimits::default()).unwrap()
}

/// What reading `xml` refuses with.
fn refused(xml: &str) -> Diagnostic {
    match StylesEdit::read(xml.as_bytes(), &StyleLimits::default()) {
        Err(StyleError::Invalid(diagnostic)) => diagnostic,
        other => panic!("not refused as invalid: {other:?}"),
    }
}

/// Asserts that reading `xml` is refused with `xml_schema_invalid` for
/// `reason`.
fn refused_as(xml: &str, reason: SchemaReason) {
    match refused(xml) {
        Diagnostic::XmlSchemaInvalid { reason: found, .. } => assert_eq!(found, reason, "{xml}"),
        other => panic!("{other:?}"),
    }
}

/// The part `edit` writes from `original`, as text.
fn rewritten(edit: &StylesEdit, original: &str) -> String {
    let mut out = Vec::new();
    edit.rewrite(&mut original.as_bytes(), &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

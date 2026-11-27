//! Tests for `PackageEdit`, driven through its public API on a small
//! package built in memory and read back with `xlsx_graft_package`.
//! `replace`, `add` and `remove` cover what each call writes, `links` the
//! `Id` and `Target` of a relationship added, `failures` the calls refused
//! and the problems and failures `finish` ends in, `holes` the names an edit
//! takes against the names the package holds, and `content_types` the rewrite
//! of the content types stream on plain text. `relative` tests the relative
//! reference on plain values.

mod add;
mod chain;
mod content_types;
mod failures;
mod holes;
mod links;
mod relative;
mod remove;
mod replace;

use crate::tests::{Sink, bounded, stored, written};
use crate::{PackageEdit, PartRewriter, PartWriter, WriteError, WriteLimits};
use std::cell::Cell;
use std::io::{BufRead, Cursor, Write};
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics};
use xlsx_graft_package::{ContentType, Limits, Package, PartName};
use xlsx_graft_spool::MemorySpools;

/// The content types namespace.
pub(super) const TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
/// The relationships namespace.
pub(super) const RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
/// The relationship types the package uses.
const TYPE_BASE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// The declaration Excel writes, then CR LF.
pub(super) const DECLARATION: &str =
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n";

/// A package of the tests, which is a workbook of one sheet: each entry's
/// name and its content.
pub(super) type Entries = Vec<(String, String)>;

/// The package of one sheet that the tests edit. The sheet's relationships
/// part holds a relationship to a resource outside the package; the
/// workbook's holds its relationship to the sheet written as a start tag
/// and an end tag.
pub(super) fn entries() -> Entries {
    let rel = |id: &str, kind: &str, target: &str| {
        format!(r#"<Relationship Id="{id}" Type="{TYPE_BASE}/{kind}" Target="{target}"/>"#)
    };
    let types = format!(
        concat!(
            r#"<Types xmlns="{ns}">"#,
            r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
            r#"<Default Extension="xml" ContentType="application/xml"/>"#,
            r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.ms-excel.sheet.main+xml"/>"#,
            r#"<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.ms-excel.worksheet+xml"/>"#,
            r#"<Override PartName="/docProps/app.xml" ContentType="application/vnd.ms-excel.app+xml"/>"#,
            r#"</Types>"#,
        ),
        ns = TYPES_NS
    );
    let rels = |body: &str| format!(r#"<Relationships xmlns="{RELS_NS}">{body}</Relationships>"#);
    let sheet_rel = format!(
        r#"<Relationship Id="rId1" Type="{TYPE_BASE}/worksheet" Target="worksheets/sheet1.xml"></Relationship>"#
    );
    let link = r#"<Relationship Id="rId1" Type="urn:link" Target="https://example.com/" TargetMode="External"/>"#;
    [
        ("[Content_Types].xml", format!("{DECLARATION}{types}")),
        (
            "_rels/.rels",
            format!(
                "{DECLARATION}{}",
                rels(&format!(
                    "{}{}",
                    rel("rId1", "officeDocument", "xl/workbook.xml"),
                    rel("rId2", "extended-properties", "docProps/app.xml"),
                ))
            ),
        ),
        ("docProps/app.xml", "<Properties/>".to_owned()),
        ("xl/workbook.xml", "<workbook/>".to_owned()),
        (
            "xl/_rels/workbook.xml.rels",
            format!("{DECLARATION}{}", rels(&sheet_rel)),
        ),
        ("xl/worksheets/sheet1.xml", "<worksheet/>".to_owned()),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            format!("{DECLARATION}{}", rels(link)),
        ),
    ]
    .into_iter()
    .map(|(name, content)| (name.to_owned(), content))
    .collect()
}

/// `entries` as the zip bytes, each entry stored.
pub(super) fn zip(entries: &Entries) -> Vec<u8> {
    let pairs: Vec<(&[u8], &[u8])> = (entries.iter())
        .map(|(name, content)| (name.as_bytes(), content.as_bytes()))
        .collect();
    stored(&pairs)
}

/// `entries` as the zip bytes, each entry deflated.
pub(super) fn deflated_zip(entries: &Entries) -> Vec<u8> {
    let pairs: Vec<(&[u8], &[u8])> = (entries.iter())
        .map(|(name, content)| (name.as_bytes(), content.as_bytes()))
        .collect();
    written(&pairs)
}

/// The diagnostics' codes with where each points, as
/// `code <entry, else part, else Relationships part>#<relationshipId>`: the
/// first of the three that the location has, and the relationship's `Id` if it
/// has one.
pub(super) fn found(diagnostics: &Diagnostics) -> Vec<String> {
    (diagnostics.records().iter())
        .map(|record| {
            let location = &record.location;
            let at = (location.entry.as_ref())
                .or(location.part.as_ref())
                .or(location.rels_part.as_ref())
                .map_or("", |echo| echo.as_str());
            let id = (location.relationship_id.as_ref())
                .map_or(String::new(), |id| format!("#{}", id.as_str()));
            format!("{} {at}{id}", record.diagnostic.code())
        })
        .collect()
}

/// The package in `bytes`, read without a diagnostic.
pub(super) fn open(bytes: Vec<u8>) -> Package<Cursor<Vec<u8>>> {
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(Cursor::new(bytes), &Limits::default(), &mut diagnostics);
    assert_eq!(found(&diagnostics), Vec::<String>::new());
    package.unwrap().unwrap()
}

/// The package in `bytes`, whatever the diagnostics: for a package that
/// leaves parts unreferenced.
pub(super) fn open_loose(bytes: Vec<u8>) -> Package<Cursor<Vec<u8>>> {
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(Cursor::new(bytes), &Limits::default(), &mut diagnostics);
    package.unwrap().unwrap()
}

/// The diagnostics reading the package in `bytes` gives.
pub(super) fn reread(bytes: &[u8]) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut diagnostics,
    );
    assert!(package.unwrap().is_some());
    diagnostics
}

/// An edit of the tests' package, stored.
pub(super) fn edit<'r>() -> PackageEdit<'r, Cursor<Vec<u8>>> {
    PackageEdit::new(open(zip(&entries())))
}

/// What `edit.finish` gives, under the default limits and in-memory spools:
/// the zip, when there is one, and the diagnostics pushed.
pub(super) fn finish(edit: PackageEdit<'_, Cursor<Vec<u8>>>) -> (Option<Vec<u8>>, Diagnostics) {
    finish_with(edit, &WriteLimits::default())
}

/// [`finish`] under `limits`.
pub(super) fn finish_with(
    edit: PackageEdit<'_, Cursor<Vec<u8>>>,
    limits: &WriteLimits,
) -> (Option<Vec<u8>>, Diagnostics) {
    let mut diagnostics = Diagnostics::new();
    let written = edit
        .finish(Sink::default(), &mut MemorySpools, limits, &mut diagnostics)
        .unwrap();
    (written.map(|sink| sink.bytes), diagnostics)
}

/// The zip `edit` writes, which must be one.
pub(super) fn written_zip(edit: PackageEdit<'_, Cursor<Vec<u8>>>) -> Vec<u8> {
    let (bytes, diagnostics) = finish(edit);
    assert_eq!(found(&diagnostics), Vec::<String>::new());
    bytes.unwrap()
}

/// The name and inflated content of each entry of the zip `bytes`.
pub(super) fn contents(bytes: &[u8]) -> Entries {
    let mut archive = crate::tests::open(bytes.to_vec());
    let names: Vec<String> = (archive.entries().iter())
        .map(|entry| String::from_utf8_lossy(entry.name()).into_owned())
        .collect();
    (names.into_iter().enumerate())
        .map(|(index, name)| {
            let content = crate::tests::read(&mut archive, index);
            (name, String::from_utf8(content).unwrap())
        })
        .collect()
}

/// The content of the entry `name` of the zip `bytes`.
pub(super) fn content(bytes: &[u8], name: &str) -> String {
    let all = contents(bytes);
    let found = all.into_iter().find(|(entry, _)| entry == name);
    found.unwrap().1
}

/// The part name `name`.
pub(super) fn part(name: &str) -> PartName {
    PartName::parse(name).unwrap()
}

/// The media type `value`.
pub(super) fn media(value: &str) -> ContentType {
    ContentType::parse(value).unwrap()
}

/// A part written by writing `text`.
pub(super) struct Text(pub(super) &'static str);

impl PartWriter for Text {
    fn write(&mut self, out: &mut dyn Write) -> Result<(), WriteError> {
        out.write_all(self.0.as_bytes())?;
        Ok(())
    }
}

impl PartRewriter for Text {
    fn rewrite(
        &mut self,
        _original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        out.write_all(self.0.as_bytes())?;
        Ok(())
    }
}

/// A rewriter that counts its calls and writes the original unchanged.
pub(super) struct Copy(pub(super) Rc<Cell<u32>>);

impl PartRewriter for Copy {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        self.0.set(self.0.get() + 1);
        std::io::copy(&mut bounded(original), out)?;
        Ok(())
    }
}

/// A rewriter or writer that refuses the content with `xml_doctype`.
pub(super) struct Refuse;

impl Refuse {
    pub(super) fn diagnostic() -> Diagnostic {
        Diagnostic::XmlDoctype { offset: 7 }
    }
}

impl PartRewriter for Refuse {
    fn rewrite(
        &mut self,
        _original: &mut dyn BufRead,
        _out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        Err(WriteError::Invalid(Self::diagnostic()))
    }
}

impl PartWriter for Refuse {
    fn write(&mut self, _out: &mut dyn Write) -> Result<(), WriteError> {
        Err(WriteError::Invalid(Self::diagnostic()))
    }
}

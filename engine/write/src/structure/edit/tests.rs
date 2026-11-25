//! Tests for `StructureEdit`'s calls, on workbooks built in memory and
//! opened with `StructureEdit::open`, reading the state the calls leave:
//! `sheets` covers add, rename, delete and move and the errors of each,
//! `names` set and remove of a defined name, `formula` how a rename and a
//! delete rewrite the formulas of the defined names, `views` where the
//! views point after a delete, `apply` what `apply` does to the parts of the
//! package, `parts` the formulas of the other parts it rewrites and the
//! report it returns, and `rewrite` the workbook part it writes.

mod apply;
mod change;
mod cleanup;
mod formula;
mod limits;
mod names;
mod operand;
mod parts;
mod rewrite;
mod sharing;
mod sheets;
mod views;

use super::StructureEdit;
use super::state::{Scope, Text};
use crate::structure::StructureReport;
use crate::tests::stored;
use crate::{PackageEdit, WriteLimits};
use std::io::Cursor;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_spool::MemorySpools;

pub(super) const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub(super) const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const TYPES: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
pub(super) const TABLE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";

/// A sheet as `book` takes it: its name and the attributes after the name
/// (`state="hidden"`, a `sheetId`).
pub(super) type Sheet<'a> = (&'a str, &'a str);

/// The package of a workbook of `sheets` (each with the `r:id` `rId<n>`
/// and the `sheetId` `<n>` unless its attributes give a `sheetId`), `before`
/// the `sheets` element and `after` it, and the parts `extra` (name, media
/// type, content).
pub(super) fn package(
    before: &str,
    sheets: &[Sheet<'_>],
    after: &str,
    extra: &[(&str, &str, &str)],
) -> Package<Cursor<Vec<u8>>> {
    open_zip(zip(before, sheets, after, extra))
}

/// The zip of [`package`], before it is opened.
pub(super) fn zip(
    before: &str,
    sheets: &[Sheet<'_>],
    after: &str,
    extra: &[(&str, &str, &str)],
) -> Vec<u8> {
    let list: String = (sheets.iter().zip(1..))
        .map(|((name, attributes), n)| {
            let id = if attributes.contains("sheetId") {
                String::new()
            } else {
                format!(r#" sheetId="{n}""#)
            };
            format!(r#"<sheet name="{name}"{id} {attributes} r:id="rId{n}"/>"#)
        })
        .collect();
    let workbook = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}">{before}<sheets>{list}</sheets>{after}</workbook>"#
    );
    let rels: String = (1..=sheets.len())
        .map(|n| {
            format!(
                r#"<Relationship Id="rId{n}" Type="{R}/worksheet" Target="worksheets/sheet{n}.xml"/>"#
            )
        })
        .collect();
    zip_raw(&workbook, &rels, sheets.len(), extra, "")
}

/// The package of the workbook part `workbook`, the relationships `rels`
/// of the workbook part (the body of the Relationships part), the
/// worksheets `xl/worksheets/sheet1.xml` to `sheet<sheets>.xml`, the parts
/// `extra` (name, media type, content: no override for an empty media
/// type) and the package relationships `root_rels` after the one to the
/// workbook part (a body, too).
pub(super) fn package_raw(
    workbook: &str,
    rels: &str,
    sheets: usize,
    extra: &[(&str, &str, &str)],
    root_rels: &str,
) -> Package<Cursor<Vec<u8>>> {
    open_zip(zip_raw(workbook, rels, sheets, extra, root_rels))
}

/// The zip of [`package_raw`], before it is opened.
pub(super) fn zip_raw(
    workbook: &str,
    rels: &str,
    sheets: usize,
    extra: &[(&str, &str, &str)],
    root_rels: &str,
) -> Vec<u8> {
    let mut entries: Vec<(String, String, String)> = vec![
        (
            "xl/workbook.xml".into(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml".into(),
            workbook.to_owned(),
        ),
        (
            "xl/_rels/workbook.xml.rels".into(),
            String::new(),
            format!(r#"<Relationships xmlns="{REL}">{rels}</Relationships>"#),
        ),
    ];
    for n in 1..=sheets {
        // A part of `extra` with the name takes the worksheet's place.
        let name = format!("xl/worksheets/sheet{n}.xml");
        if extra.iter().any(|(extra, _, _)| *extra == name) {
            continue;
        }
        entries.push((
            format!("xl/worksheets/sheet{n}.xml"),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml".into(),
            "<worksheet/>".into(),
        ));
    }
    entries.extend(
        (extra.iter())
            .map(|(name, kind, content)| ((*name).into(), (*kind).into(), (*content).into())),
    );
    let overrides: String = (entries.iter())
        .filter(|(_, kind, _)| !kind.is_empty())
        .map(|(name, kind, _)| format!(r#"<Override PartName="/{name}" ContentType="{kind}"/>"#))
        .collect();
    let types = format!(
        r#"<Types xmlns="{TYPES}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>{overrides}</Types>"#
    );
    let root = format!(
        r#"<Relationships xmlns="{REL}"><Relationship Id="rId1" Type="{R}/officeDocument" Target="xl/workbook.xml"/>{root_rels}</Relationships>"#
    );
    let mut all = vec![
        ("[Content_Types].xml".to_owned(), types),
        ("_rels/.rels".to_owned(), root),
    ];
    all.extend(
        entries
            .into_iter()
            .map(|(name, _, content)| (name, content)),
    );
    let pairs: Vec<(&[u8], &[u8])> = (all.iter())
        .map(|(name, content)| (name.as_bytes(), content.as_bytes()))
        .collect();
    stored(&pairs)
}

/// The package of the zip `bytes`.
pub(super) fn open_zip(bytes: Vec<u8>) -> Package<Cursor<Vec<u8>>> {
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(Cursor::new(bytes), &Limits::default(), &mut diagnostics);
    package.unwrap().unwrap()
}

/// The edit of the workbook of `sheets` and `after` (the elements after
/// `sheets`), opened without a diagnostic.
pub(super) fn open_with(before: &str, sheets: &[Sheet<'_>], after: &str) -> StructureEdit<'static> {
    let mut package = package(before, sheets, after, &[]);
    let mut diagnostics = Diagnostics::new();
    let edit = StructureEdit::open(&mut package, &WriteLimits::default(), &mut diagnostics);
    assert!(diagnostics.records().is_empty());
    edit.unwrap().unwrap()
}

/// The edit of the workbook of sheets named `names`, then `after`.
pub(super) fn open(names: &[&str], after: &str) -> StructureEdit<'static> {
    let sheets: Vec<Sheet<'_>> = names.iter().map(|name| (*name, "")).collect();
    open_with("", &sheets, after)
}

/// The `definedNames` element of `names` (name, `localSheetId` or `""`,
/// formula text).
pub(super) fn defined(names: &[(&str, &str, &str)]) -> String {
    let body: String = (names.iter())
        .map(|(name, scope, text)| {
            let scope = if scope.is_empty() {
                String::new()
            } else {
                format!(r#" localSheetId="{scope}""#)
            };
            format!(r#"<definedName name="{name}"{scope} comment="c">{text}</definedName>"#)
        })
        .collect();
    format!("<definedNames>{body}</definedNames>")
}

/// The sheets' names, in order.
pub(super) fn sheet_names(edit: &StructureEdit<'_>) -> Vec<String> {
    (edit.state.sheets.iter())
        .map(|sheet| sheet.name.clone())
        .collect()
}

/// The defined names as (written name, scope, formula), in order.
pub(super) fn names_of(edit: &StructureEdit<'_>) -> Vec<(String, Scope, String)> {
    (edit.state.names.iter())
        .map(|entry| {
            (
                entry.written.clone(),
                entry.scope,
                match &entry.text {
                    Text::Original(text) | Text::Changed(text) => text.clone(),
                },
            )
        })
        .collect()
}

/// The formulas of the defined names, in order.
pub(super) fn formulas(edit: &StructureEdit<'_>) -> Vec<String> {
    names_of(edit)
        .into_iter()
        .map(|(_, _, text)| text)
        .collect()
}

/// Everything a call may change, as text, to compare before and after a
/// call that failed.
pub(super) fn fingerprint(edit: &StructureEdit<'_>) -> String {
    let state = &edit.state;
    format!(
        "{:?}|{:?}|{}|{:?}|{}|{:?}|{:?}|{:?}|{}|{:?}|{:?}",
        state.sheets,
        state.deleted,
        state.next_key,
        state.next_sheet_id,
        state.added.len(),
        state.names,
        state.removed_names,
        state.views,
        state.ref_written,
        state.changes,
        state.sheet_ids,
    )
}

/// Whether the formula of the defined name at `index` is as the workbook has
/// it.
pub(super) fn original(edit: &StructureEdit<'_>, index: usize) -> bool {
    matches!(
        edit.state.names.get(index).map(|entry| &entry.text),
        Some(Text::Original(_))
    )
}

/// The package written by an `apply`, as bytes.
pub(super) struct Output {
    bytes: Vec<u8>,
}

impl Output {
    /// The text of the part `name` (`/xl/workbook.xml`), `None` when the
    /// written package has none.
    pub(super) fn part(&self, name: &str) -> Option<String> {
        let mut diagnostics = Diagnostics::new();
        let mut package = Package::open(
            Cursor::new(self.bytes.clone()),
            &Limits::default(),
            &mut diagnostics,
        )
        .unwrap()
        .unwrap();
        let index = package
            .parts()
            .iter()
            .position(|part| part.name().as_str() == name)?;
        let limit = xlsx_graft_diagnostics::Limit::default_of(1 << 24);
        let bytes = package
            .read_xml_part(index, limit, &mut diagnostics)
            .unwrap();
        Some(String::from_utf8(bytes.unwrap()).unwrap())
    }

    /// The relationships of the workbook part as (`Id`, `Type`, name of the
    /// part it targets), in order.
    pub(super) fn workbook_relationships(&self) -> Vec<(String, String, String)> {
        let mut diagnostics = Diagnostics::new();
        let package = Package::open(
            Cursor::new(self.bytes.clone()),
            &Limits::default(),
            &mut diagnostics,
        )
        .unwrap()
        .unwrap();
        let workbook = package.part("/xl/workbook.xml").unwrap();
        (workbook.relationships().iter())
            .map(|relationship| {
                let target = match relationship.target() {
                    xlsx_graft_package::Target::Internal(index) => {
                        package.parts()[*index].name().as_str().to_owned()
                    }
                    xlsx_graft_package::Target::External(target) => target.clone(),
                };
                (
                    relationship.id().to_owned(),
                    relationship.rel_type().to_owned(),
                    target,
                )
            })
            .collect()
    }

    /// The names of the parts, in order.
    pub(super) fn names(&self) -> Vec<String> {
        let mut diagnostics = Diagnostics::new();
        let package = Package::open(
            Cursor::new(self.bytes.clone()),
            &Limits::default(),
            &mut diagnostics,
        )
        .unwrap()
        .unwrap();
        (package.parts().iter())
            .map(|part| part.name().as_str().to_owned())
            .collect()
    }

    /// Whether the written zip holds `text` as it is: a part copied from a
    /// package whose entries are stored is, a part written again is
    /// deflated.
    pub(super) fn holds(&self, text: &str) -> bool {
        let needle = text.as_bytes();
        self.bytes
            .windows(needle.len())
            .any(|window| window == needle)
    }
}

/// Opens `package` as an edit, lets `act` call it, applies it to a
/// `PackageEdit` of `package` and writes that.
pub(super) fn applied(
    package: Package<Cursor<Vec<u8>>>,
    act: impl FnOnce(&mut StructureEdit<'static>),
) -> Output {
    applied_with_report(package, act).0
}

/// As [`applied`], with the report `apply` returned.
pub(super) fn applied_with_report(
    mut package: Package<Cursor<Vec<u8>>>,
    act: impl FnOnce(&mut StructureEdit<'static>),
) -> (Output, StructureReport) {
    let mut diagnostics = Diagnostics::new();
    let limits = WriteLimits::default();
    let mut edit = StructureEdit::open(&mut package, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    act(&mut edit);
    let mut package_edit = PackageEdit::new(package);
    let report = edit.apply(&mut package_edit).unwrap();
    let bytes = package_edit
        .finish(Vec::new(), &mut MemorySpools, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    assert!(diagnostics.records().is_empty());
    (Output { bytes }, report)
}

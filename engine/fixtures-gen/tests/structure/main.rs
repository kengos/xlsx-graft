//! `xlsx_graft_write::structure::StructureEdit` applied to real workbooks and
//! read back: on every workbook of `fixtures/` and on a crafted workbook of
//! four sheets with defined names, views and a calcChain, each of add,
//! rename, delete, move, set a defined name and remove a defined name is
//! applied, written to memory by `PackageEdit::finish`, and read back with
//! `xlsx-graft-read` (the sheets) and the writer's workbook model (the
//! defined names and the views), and checked by `xlsx-graft-verify`.
//!
//! What each check says:
//!
//! - the sheets, in order, are the ones the call should leave;
//! - each view points at the sheet it pointed at, when that sheet is there,
//!   and the defined names (name, the sheet it is scoped to, text) are the
//!   ones the call implies;
//! - the error diagnostics `verify` reports are the ones it reports for the
//!   workbook before the edit, each counted (a fixture such as `macro.xlsm`
//!   is refused with its VBA project before and after), but for the
//!   `function_denied` that removing a defined name a cell formula calls
//!   (`MyLambda` of `formulas.xlsx`) leaves;
//! - the entries that are removed after a delete are the deleted sheet's part,
//!   the parts only the parts that are removed target, the calcChain part, and
//!   the Relationships part of each; and every other entry has the same
//!   method, CRC-32 and compressed bytes as in the original, but the ones
//!   the call may rewrite: the workbook part, the content types and the
//!   workbook part's Relationships part after an add or a delete, the other
//!   Relationships parts after a delete, and `docProps/app.xml` after a call
//!   that changes the sheets.
//!
//! A fixture is skipped for an operation only when it cannot be done to it:
//! delete needs a sheet whose delete is allowed (the workbook keeps a visible
//! sheet), move needs two sheets.
//!
//! The test is here and not in `xlsx-graft-write` because the writer is
//! published before the reader and the verifier
//! (`scripts/release/crates.txt`), so it cannot depend on them.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use xlsx_graft_cell::{escape, unescape};
use xlsx_graft_diagnostics::{Diagnostics, Severity};
use xlsx_graft_package::{Limits, Package, RelationshipSource, Target, ZipArchive, ZipLimits};
use xlsx_graft_read::{ReadLimits, Workbook, WorkbookPart};
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_verify::{DefaultFunctions, Rules, Targets};
use xlsx_graft_write::structure::{
    KeptReason, StructureEdit, StructureError, StructureReport, model, sheet_prefix,
};
use xlsx_graft_write::{DeflateWriter, PackageEdit, WriteLimits, ZipWriter};

/// The repository's `fixtures/` folder.
fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// The paths of the workbooks in `fixtures/` (`.xlsx` and `.xlsm`), sorted.
fn workbooks() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(folder())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("xlsx" | "xlsm")
            )
        })
        .collect();
    paths.sort();
    paths
}

/// One call of a structure edit.
#[derive(Debug, Clone)]
enum Op {
    Add(String),
    Rename(String, String),
    Delete(String),
    Move(String, usize),
    Set(String, Option<String>, String),
    Remove(String, Option<String>),
}

impl Op {
    /// Makes the call on `edit`.
    fn call(&self, edit: &mut StructureEdit<'_>) -> Result<(), StructureError> {
        match self {
            Self::Add(name) => edit.add_sheet(name, None, None),
            Self::Rename(old, new) => edit.rename_sheet(old, new),
            Self::Delete(name) => edit.delete_sheet(name),
            Self::Move(name, to) => edit.move_sheet(name, *to),
            Self::Set(name, scope, formula) => {
                edit.set_defined_name(name, scope.as_deref(), formula)
            }
            Self::Remove(name, scope) => edit.remove_defined_name(name, scope.as_deref()),
        }
    }

    /// The sheets' names after the call, given the names before it.
    fn sheets(&self, mut names: Vec<String>) -> Vec<String> {
        match self {
            Self::Add(name) => names.push(name.clone()),
            Self::Rename(old, new) => {
                names
                    .iter_mut()
                    .filter(|name| *name == old)
                    .for_each(|name| name.clone_from(new));
            }
            Self::Delete(gone) => names.retain(|name| name != gone),
            Self::Move(name, to) => {
                let at = names.iter().position(|other| other == name).unwrap();
                let moved = names.remove(at);
                names.insert(*to, moved);
            }
            Self::Set(..) | Self::Remove(..) => {}
        }
        names
    }
}

/// What a call left, read back.
struct Written {
    bytes: Vec<u8>,
    report: StructureReport,
}

/// Applies `op` to the workbook in `bytes`, `None` when the edit refuses it.
fn apply(bytes: &[u8], op: &Op) -> Option<Written> {
    let mut diagnostics = Diagnostics::new();
    let limits = WriteLimits::default();
    let mut package = Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    let mut edit = StructureEdit::open(&mut package, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    op.call(&mut edit).ok()?;
    let mut package_edit = PackageEdit::new(package);
    let report = edit.apply(&mut package_edit).unwrap();
    let written = package_edit
        .finish(Vec::new(), &mut MemorySpools, &limits, &mut diagnostics)
        .unwrap()
        .unwrap();
    assert!(
        diagnostics.records().is_empty(),
        "{:?}",
        diagnostics.records()
    );
    Some(Written {
        bytes: written,
        report,
    })
}

/// The sheets' names and whether each is visible, as the reader reads them.
fn sheets(bytes: &[u8]) -> Vec<(String, String)> {
    let mut package = Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    let mut diagnostics = Diagnostics::new();
    let found = Workbook::find(&package, &mut diagnostics).unwrap();
    let read = WorkbookPart::read(
        &mut package,
        &found,
        &ReadLimits::default(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    assert!(
        diagnostics.records().is_empty(),
        "{:?}",
        diagnostics.records()
    );
    (read.sheets().iter())
        .map(|sheet| (sheet.name().to_owned(), format!("{:?}", sheet.state())))
        .collect()
}

/// The workbook model of `bytes`.
fn book(bytes: &[u8]) -> model::Workbook {
    let mut package = Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    let mut diagnostics = Diagnostics::new();
    let book = model::Workbook::read(&mut package, &WriteLimits::default(), &mut diagnostics)
        .unwrap()
        .unwrap();
    assert!(
        diagnostics.records().is_empty(),
        "{:?}",
        diagnostics.records()
    );
    book
}

/// The codes of the diagnostics of error severity `verify` pushes for
/// `bytes`, one for each diagnostic, sorted.
fn errors(bytes: &[u8]) -> Vec<String> {
    let mut diagnostics = Diagnostics::new();
    xlsx_graft_verify::verify(
        Cursor::new(bytes),
        &xlsx_graft_verify::Limits::default(),
        &Rules::default(),
        &DefaultFunctions,
        &Targets::default(),
        &mut diagnostics,
    )
    .unwrap();
    let mut codes: Vec<String> = (diagnostics.records().iter())
        .filter(|record| record.diagnostic.severity() == Severity::Error)
        .map(|record| record.diagnostic.code().as_str().to_owned())
        .collect();
    codes.sort();
    codes
}

/// Whether a worksheet of `bytes` calls `name` in a formula (`name(`, letter
/// case aside).
fn calls(bytes: &[u8], name: &str) -> bool {
    let mut diagnostics = Diagnostics::new();
    let mut package = Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    let wanted = format!("{}(", name.to_lowercase());
    let indexes: Vec<usize> = (package.parts().iter().enumerate())
        .filter(|(_, part)| part.name().as_str().starts_with("/xl/worksheets/sheet"))
        .map(|(index, _)| index)
        .collect();
    indexes.into_iter().any(|index| {
        let limit = xlsx_graft_diagnostics::Limit::default_of(1 << 28);
        let text = package
            .read_xml_part(index, limit, &mut diagnostics)
            .unwrap();
        text.is_some_and(|text| {
            String::from_utf8_lossy(&text)
                .to_lowercase()
                .contains(&wanted)
        })
    })
}

/// The text of the XML part `name` (`/xl/workbook.xml`) of `bytes`.
fn part_text(bytes: &[u8], name: &str) -> String {
    let mut diagnostics = Diagnostics::new();
    let mut package = Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    let index = (package.parts().iter())
        .position(|part| part.name().as_str() == name)
        .unwrap_or_else(|| panic!("no part {name}"));
    let limit = xlsx_graft_diagnostics::Limit::default_of(1 << 28);
    let text = package
        .read_xml_part(index, limit, &mut diagnostics)
        .unwrap()
        .unwrap();
    String::from_utf8(text).unwrap()
}

/// Whether the XML entry `entry` of `bytes` holds `sheet`, letter case aside.
fn mentions(bytes: &[u8], entry: &str, sheet: &str) -> bool {
    let name = format!("/{entry}");
    entry.ends_with(".xml")
        && part_text(bytes, &name)
            .to_lowercase()
            .contains(&sheet.to_lowercase())
}

/// The entries of the zip `bytes`: name to (method, CRC-32, compressed bytes).
fn entries(bytes: &[u8]) -> BTreeMap<String, (String, u32, Vec<u8>)> {
    let archive = ZipArchive::open(
        Cursor::new(bytes.to_vec()),
        &ZipLimits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    (archive.entries().iter())
        .map(|entry| {
            let range = entry.data_range();
            let data = bytes[range.start as usize..range.end as usize].to_vec();
            let name = String::from_utf8_lossy(entry.name()).into_owned();
            (name, (format!("{:?}", entry.method()), entry.crc32(), data))
        })
        .collect()
}

/// The workbook part's name without its leading `/`, and the name of its
/// Relationships part.
fn workbook_entries(bytes: &[u8]) -> (String, String) {
    let package = Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    let part = &package.parts()[book(bytes).part()];
    let entry = |name: &str| name.trim_start_matches('/').to_owned();
    (
        entry(part.name().as_str()),
        entry(part.name().relationships_part().as_str()),
    )
}

/// The entries that are removed when the sheet `gone` is deleted from the
/// workbook in `bytes`: the part of the sheet, each part only the parts that
/// are removed target (no relationship of a part that stays and none of the
/// package targets it), the calcChain part, and the Relationships part of
/// each of them. Worked out here from the package's relationships, not by the
/// engine's code.
fn removed_entries(bytes: &[u8], gone: &str) -> BTreeSet<String> {
    let package = Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut Diagnostics::new(),
    )
    .unwrap()
    .unwrap();
    let book = book(bytes);
    let sheet = book
        .sheets()
        .iter()
        .find(|sheet| sheet.name() == gone)
        .unwrap();
    let mut left: BTreeSet<usize> = BTreeSet::from([sheet.part()]);
    loop {
        let more: Vec<usize> = (package.parts().iter().enumerate())
            .filter(|(index, part)| {
                let from: Vec<_> = part
                    .incoming()
                    .iter()
                    .map(|incoming| incoming.source)
                    .collect();
                !left.contains(index)
                    && !from.is_empty()
                    && from.iter().all(|source| match source {
                        RelationshipSource::Part(from) => left.contains(from),
                        RelationshipSource::Package => false,
                    })
            })
            .map(|(index, _)| index)
            .collect();
        if more.is_empty() {
            break;
        }
        left.extend(more);
    }
    let chain = (package.parts()[book.part()].relationships().iter())
        .find(|relationship| {
            relationship
                .rel_type()
                .ends_with("/relationships/calcChain")
        })
        .and_then(|relationship| match relationship.target() {
            Target::Internal(index) => Some(*index),
            Target::External(_) => None,
        });
    let present = entries(bytes);
    left.extend(chain);
    let mut names = BTreeSet::new();
    for index in left {
        let name = package.parts()[index].name();
        names.insert(name.as_str().trim_start_matches('/').to_owned());
        names.insert(
            name.relationships_part()
                .as_str()
                .trim_start_matches('/')
                .to_owned(),
        );
    }
    names.retain(|name| present.contains_key(name));
    names
}

/// A defined name as (the name as written, the name of the sheet it is
/// scoped to, `#<id>` for a scope that is no sheet, its text).
type Defined = (String, Option<String>, String);

/// The defined names of `bytes`.
fn defined_of(bytes: &[u8]) -> Vec<Defined> {
    let names: Vec<String> = sheets(bytes).into_iter().map(|(name, _)| name).collect();
    (book(bytes).defined_names().iter())
        .map(|name| {
            let scope = name.local_sheet_id().map(|id| {
                let at = usize::try_from(id).unwrap();
                names.get(at).cloned().unwrap_or_else(|| format!("#{id}"))
            });
            (name.name().to_owned(), scope, name.text().to_owned())
        })
        .collect()
}

/// `text`, a defined name as written, with its `_xHHHH_` escapes undone.
fn unescaped(text: &str) -> String {
    let mut out = String::new();
    unescape::<std::convert::Infallible>(text, true, |piece| {
        out.push_str(piece);
        Ok(())
    })
    .unwrap();
    out
}

/// What `op` leaves of the defined names `before`: each with the text it is
/// expected to have, `None` for a text this check does not work out (a
/// formula that names a sheet in a form other than a prefix of its own).
fn defined_after(op: &Op, before: &[Defined]) -> Vec<(String, Option<String>, Option<String>)> {
    let same = |entry: &Defined| (entry.0.clone(), entry.1.clone(), Some(entry.2.clone()));
    let prefix = |name: &str| format!("{}!", sheet_prefix(name, None));
    match op {
        Op::Add(_) | Op::Move(..) => before.iter().map(same).collect(),
        Op::Rename(old, new) => before
            .iter()
            .map(|(name, scope, text)| {
                let scope = scope
                    .as_ref()
                    .map(|s| if s == old { new.clone() } else { s.clone() });
                let text = if text.contains(&prefix(old)) {
                    Some(text.replace(&prefix(old), &prefix(new)))
                } else {
                    (!text.contains(old.as_str())).then(|| text.clone())
                };
                (name.clone(), scope, text)
            })
            .collect(),
        Op::Delete(gone) => before
            .iter()
            .filter(|(_, scope, _)| scope.as_deref() != Some(gone.as_str()))
            .map(|(name, scope, text)| {
                let untouched = !text.contains(gone.as_str());
                (name.clone(), scope.clone(), untouched.then(|| text.clone()))
            })
            .collect(),
        Op::Set(name, scope, formula) => {
            let written = escaped_written(name);
            let mut list: Vec<_> = before.iter().map(same).collect();
            let found = list.iter_mut().find(|(other, other_scope, _)| {
                other.to_lowercase() == written.to_lowercase() && other_scope == scope
            });
            match found {
                Some(entry) => entry.2 = Some(formula.clone()),
                None => list.push((written, scope.clone(), Some(formula.clone()))),
            }
            list
        }
        Op::Remove(name, scope) => before
            .iter()
            .filter(|(other, other_scope, _)| {
                !(other.to_lowercase() == escaped_written(name).to_lowercase()
                    && other_scope == scope)
            })
            .map(same)
            .collect(),
    }
}

/// `name` as the engine writes a defined name: its `_xHHHH_` escapes made.
fn escaped_written(name: &str) -> String {
    let mut out = String::new();
    escape::<std::convert::Infallible>(name, |piece| {
        out.push_str(piece);
        Ok(())
    })
    .unwrap();
    out
}

/// Whether the engine may rewrite the entry `name` for `op` on the workbook
/// in `before`.
fn rewritten(name: &str, op: &Op, before: &[u8]) -> bool {
    let (workbook, workbook_rels) = workbook_entries(before);
    let adds_or_removes = matches!(op, Op::Add(_) | Op::Delete(_));
    let sheets_change = matches!(
        op,
        Op::Add(_) | Op::Rename(..) | Op::Delete(_) | Op::Move(..)
    );
    name == workbook
        || (name == "[Content_Types].xml" && adds_or_removes)
        || (name == workbook_rels && adds_or_removes)
        || (name.ends_with(".rels") && matches!(op, Op::Delete(_)))
        || (name == "docProps/app.xml" && sheets_change)
}

/// Checks `written`, the result of `op` on the workbook in `before`: the
/// sheets, the views, the defined names, the errors and the entries.
fn check(label: &str, before: &[u8], op: &Op, written: &Written) {
    let was: Vec<String> = sheets(before).into_iter().map(|(name, _)| name).collect();
    let got: Vec<String> = sheets(&written.bytes)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        got,
        op.sheets(was.clone()),
        "{label}: the sheets after {op:?}"
    );
    check_errors(label, before, op, &written.bytes);
    check_entries(label, before, op, &written.bytes);
    // Each view points at the sheet it pointed at, when that sheet is there.
    let (old, new) = (book(before), book(&written.bytes));
    assert_eq!(old.views().len(), new.views().len(), "{label}: the views");
    for (was_view, view) in old.views().iter().zip(new.views()) {
        for (then, now) in [
            (was_view.active_tab(), view.active_tab()),
            (was_view.first_sheet(), view.first_sheet()),
        ] {
            let at = usize::try_from(then.unwrap_or(0)).unwrap();
            let Some(name) = was.get(at) else { continue };
            let name = match op {
                Op::Rename(old, new) if old == name => new,
                Op::Delete(gone) if gone == name => continue,
                _ => name,
            };
            let now = usize::try_from(now.unwrap_or(0)).unwrap();
            assert_eq!(got.get(now), Some(name), "{label}: a view after {op:?}");
        }
    }
    let after = defined_of(&written.bytes);
    let expected = defined_after(op, &defined_of(before));
    assert_eq!(
        after.len(),
        expected.len(),
        "{label}: the defined names after {op:?}"
    );
    for ((name, scope, text), (want_name, want_scope, want_text)) in after.iter().zip(&expected) {
        assert_eq!(
            (name, scope),
            (want_name, want_scope),
            "{label}: after {op:?}"
        );
        if let Some(want) = want_text {
            assert_eq!(text, want, "{label}: {name} after {op:?}");
        }
    }
    if let Op::Delete(gone) = op {
        let single = format!("{}!", sheet_prefix(gone, None));
        assert!(
            after.iter().all(|(_, _, text)| !text.contains(&single)),
            "{label}: a formula still names the sheet {gone}"
        );
    }
}

/// Checks the errors `verify` reports after `op`: the ones it reported
/// before, counting each (after a delete, no more of any: the errors `verify`
/// reported may be in the cells of the deleted sheet), and after removing a
/// defined name that a cell formula calls, the `function_denied` of each call,
/// which is a problem of the formula, not of the edit.
fn check_errors(label: &str, before: &[u8], op: &Op, written: &[u8]) {
    let (had, now) = (errors(before), errors(written));
    let count =
        |codes: &[String], code: &String| codes.iter().filter(|other| *other == code).count();
    if let Op::Delete(_) = op {
        // The errors `verify` reported may be in the cells of the deleted sheet.
        let fewer = now
            .iter()
            .all(|code| count(&now, code) <= count(&had, code));
        assert!(fewer, "{label}: {op:?} added errors {now:?}, had {had:?}");
        return;
    }
    let Op::Remove(name, _) = op else {
        assert_eq!(now, had, "{label}: the errors after {op:?}");
        return;
    };
    let rest = |codes: &[String]| -> Vec<String> {
        (codes.iter())
            .filter(|code| *code != "function_denied")
            .cloned()
            .collect()
    };
    assert_eq!(rest(&now), rest(&had), "{label}: the errors after {op:?}");
    let more = now.len() - rest(&now).len() > had.len() - rest(&had).len();
    assert!(
        !more || calls(before, name),
        "{label}: function_denied after {op:?}, which no formula calls"
    );
}

/// Checks the entries of the zip after `op`: those that are removed are those
/// that should, and none but the ones the engine may rewrite changed.
fn check_entries(label: &str, before: &[u8], op: &Op, written: &[u8]) {
    let (old, new) = (entries(before), entries(written));
    let gone: BTreeSet<String> = (old.keys())
        .filter(|name| !new.contains_key(*name))
        .cloned()
        .collect();
    let expected = match op {
        Op::Delete(name) => removed_entries(before, name),
        _ => BTreeSet::new(),
    };
    assert_eq!(
        gone, expected,
        "{label}: the entries that are removed after {op:?}"
    );
    // A rename and a delete rewrite the formulas of the other parts: the
    // parts that hold the sheet's name, and no others.
    let sheet = match op {
        Op::Rename(old, _) => Some(old),
        Op::Delete(gone) => Some(gone),
        _ => None,
    };
    for (name, entry) in &old {
        let Some(other) = new.get(name) else { continue };
        if rewritten(name, op, before) || entry == other {
            continue;
        }
        let holds = sheet.is_some_and(|sheet| mentions(before, name, sheet));
        assert!(holds, "{label}: {name} changed after {op:?}");
    }
}

#[test]
fn every_workbook_of_fixtures_is_edited_and_read_back() {
    let paths = workbooks();
    assert!(!paths.is_empty());
    for path in paths {
        let label = path.display().to_string();
        let before = std::fs::read(&path).unwrap();
        let names: Vec<String> = sheets(&before).into_iter().map(|(name, _)| name).collect();
        let first = names[0].clone();
        let defined = book(&before).defined_names().first().map(|name| {
            let scope = name
                .local_sheet_id()
                .map(|id| names[usize::try_from(id).unwrap()].clone());
            (unescaped(name.name()), scope)
        });
        let formula = format!("{}!$A$1", sheet_prefix(&first, None));
        let mut ops = vec![
            Op::Add("Added".into()),
            Op::Rename(first.clone(), "Renamed 1".into()),
            Op::Set("ReadBack".into(), None, formula),
        ];
        if names.len() > 1 {
            ops.push(Op::Move(first.clone(), names.len() - 1));
        }
        ops.extend(defined.map(|(name, scope)| Op::Remove(name, scope)));
        for op in &ops {
            let written = apply(&before, op).unwrap_or_else(|| panic!("{label}: {op:?} refused"));
            check(&label, &before, op, &written);
        }
        // The formulas of the other parts name the new name.
        let rename = Op::Rename(first.clone(), "Renamed 1".into());
        let renamed = apply(&before, &rename).unwrap();
        let file = path.file_name().and_then(|name| name.to_str()).unwrap();
        let chart = "/xl/charts/chart1.xml";
        match file {
            "chart.xlsx" => {
                let text = part_text(&renamed.bytes, chart);
                assert!(text.contains("<c:f>'Renamed 1'!$A$2:$A$4</c:f>"), "{text}");
                assert!(!text.contains("Sheet1!"), "{text}");
            }
            "formulas.xlsx" => {
                let text = part_text(&renamed.bytes, "/xl/worksheets/sheet1.xml");
                assert!(text.contains("<f>SUM('Renamed 1:Sheet2'!A1)</f>"), "{text}");
            }
            _ => {}
        }
        // The first sheet the edit deletes: a workbook whose only visible
        // sheet is its only deletable one has none.
        let deleted = names.iter().find_map(|name| {
            let op = Op::Delete(name.clone());
            apply(&before, &op).map(|written| (op, written))
        });
        if let Some((op, written)) = deleted {
            check(&label, &before, &op, &written);
        } else {
            assert_eq!(names.len(), 1, "{label}: no sheet can be deleted");
        }
    }
}

/// A zip of `entries`, each deflated, written by the engine's own writer.
fn zipped(entries: &[(&str, String)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Vec::new());
    for (name, content) in entries {
        let mut spools = MemorySpools;
        let mut deflate = DeflateWriter::new(&mut spools, 1 << 20);
        deflate.write_all(content.as_bytes()).unwrap();
        zip.write_deflated(name.as_bytes(), deflate.finish().unwrap())
            .unwrap();
    }
    zip.finish().unwrap()
}

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// A workbook of the sheets Alpha to Delta, three defined names, an active
/// tab on Gamma, a `calcPr`, a calcChain part, an app part that lists the
/// sheets, a chart of Alpha's cells, a pivot cache of Beta's, and formulas of
/// Beta's that name Alpha and Gamma.
fn crafted() -> Vec<u8> {
    let sheet_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
    let overrides: String = (1..=4)
        .map(|n| {
            format!(
                r#"<Override PartName="/xl/worksheets/sheet{n}.xml" ContentType="{sheet_type}"/>"#
            )
        })
        .collect();
    let types = format!(
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/><Override PartName="/xl/calcChain.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.calcChain+xml"/><Override PartName="/xl/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/><Override PartName="/xl/pivotCache/pivotCacheDefinition1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.pivotCacheDefinition+xml"/>{overrides}</Types>"#
    );
    let root = format!(
        r#"<Relationships xmlns="{REL}"><Relationship Id="rId1" Type="{R}/officeDocument" Target="xl/workbook.xml"/><Relationship Id="rId2" Type="{R}/extended-properties" Target="docProps/app.xml"/></Relationships>"#
    );
    let sheets: String = (["Alpha", "Beta", "Gamma", "Delta"].iter().zip(1..))
        .map(|(name, n)| format!(r#"<sheet name="{name}" sheetId="{n}" r:id="rId{n}"/>"#))
        .collect();
    let workbook = format!(
        r#"<workbook xmlns="{NS}" xmlns:r="{R}"><bookViews><workbookView activeTab="2"/></bookViews><sheets>{sheets}</sheets><definedNames><definedName name="Total">Alpha!$A$1:$A$3</definedName><definedName name="OfBeta" localSheetId="1">Beta!$B$1</definedName><definedName name="Span">Alpha:Gamma!$A$1</definedName></definedNames><calcPr calcId="191029"/></workbook>"#
    );
    let links: String = (1..=4)
        .map(|n| format!(r#"<Relationship Id="rId{n}" Type="{R}/worksheet" Target="worksheets/sheet{n}.xml"/>"#))
        .collect();
    let chain = format!(r#"<Relationship Id="rId9" Type="{R}/calcChain" Target="calcChain.xml"/>"#);
    let more = format!(
        r#"<Relationship Id="rId10" Type="{R}/chart" Target="charts/chart1.xml"/><Relationship Id="rId11" Type="{R}/pivotCacheDefinition" Target="pivotCache/pivotCacheDefinition1.xml"/>"#
    );
    let rels = format!(r#"<Relationships xmlns="{REL}">{links}{chain}{more}</Relationships>"#);
    let app = r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>Test</Application><HeadingPairs><vt:vector size="2" baseType="variant"><vt:variant><vt:lpstr>Worksheets</vt:lpstr></vt:variant><vt:variant><vt:i4>4</vt:i4></vt:variant></vt:vector></HeadingPairs><TitlesOfParts><vt:vector size="4" baseType="lpstr"><vt:lpstr>Alpha</vt:lpstr><vt:lpstr>Beta</vt:lpstr><vt:lpstr>Gamma</vt:lpstr><vt:lpstr>Delta</vt:lpstr></vt:vector></TitlesOfParts></Properties>"#;
    let sheet = format!(r#"<worksheet xmlns="{NS}"><sheetData/></worksheet>"#);
    // Beta's cells name Alpha and Gamma, and a color scale names Alpha at a
    // place outside the list of those the edit rewrites.
    let beta = format!(
        r#"<worksheet xmlns="{NS}"><sheetData><row r="1"><c r="A1"><f>Alpha!A1+Gamma!B2</f></c></row></sheetData><conditionalFormatting sqref="C1:C3"><cfRule type="colorScale" priority="1"><colorScale><cfvo type="num" val="0"/><cfvo type="formula" val="Alpha!$A$1"/><color rgb="FFFF0000"/><color rgb="FF00FF00"/></colorScale></cfRule></conditionalFormatting></worksheet>"#
    );
    let chart = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:barChart><c:ser><c:val><c:numRef><c:f>Alpha!$A$1:$A$3</c:f></c:numRef></c:val></c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#;
    let cache = format!(
        r#"<pivotCacheDefinition xmlns="{NS}"><cacheSource type="worksheet"><worksheetSource ref="A1:C9" sheet="Beta"/></cacheSource></pivotCacheDefinition>"#
    );
    let mut entries: Vec<(String, String)> = vec![
        ("[Content_Types].xml".into(), types),
        ("_rels/.rels".into(), root),
        ("docProps/app.xml".into(), app.to_owned()),
        ("xl/workbook.xml".into(), workbook),
        ("xl/_rels/workbook.xml.rels".into(), rels),
        (
            "xl/calcChain.xml".into(),
            format!(r#"<calcChain xmlns="{NS}"/>"#),
        ),
    ];
    entries.extend((1..=4).map(|n| {
        let content = if n == 2 { &beta } else { &sheet };
        (format!("xl/worksheets/sheet{n}.xml"), content.clone())
    }));
    entries.push(("xl/charts/chart1.xml".into(), chart.to_owned()));
    entries.push(("xl/pivotCache/pivotCacheDefinition1.xml".into(), cache));
    let pairs: Vec<(&str, String)> = entries
        .iter()
        .map(|(n, c)| (n.as_str(), c.clone()))
        .collect();
    zipped(&pairs)
}

/// The defined names of `bytes` as (name, `localSheetId`, text).
fn defined(bytes: &[u8]) -> Vec<(String, Option<u32>, String)> {
    (book(bytes).defined_names().iter())
        .map(|name| {
            (
                name.name().to_owned(),
                name.local_sheet_id(),
                name.text().to_owned(),
            )
        })
        .collect()
}

#[test]
fn a_crafted_workbook_is_edited_and_the_names_and_views_are_read_back() {
    let before = crafted();
    let text = |s: &str| s.to_owned();
    let renamed = apply(&before, &Op::Rename("Alpha".into(), "Sales Q1".into())).unwrap();
    check(
        "crafted",
        &before,
        &Op::Rename("Alpha".into(), "Sales Q1".into()),
        &renamed,
    );
    assert_eq!(
        defined(&renamed.bytes),
        [
            (text("Total"), None, text("'Sales Q1'!$A$1:$A$3")),
            (text("OfBeta"), Some(1), text("Beta!$B$1")),
            (text("Span"), None, text("'Sales Q1:Gamma'!$A$1")),
        ]
    );
    // The formulas of the other parts follow the rename; the color scale's
    // `cfvo` is outside the list and is kept and reported.
    let sheet2 = |written: &Written| part_text(&written.bytes, "/xl/worksheets/sheet2.xml");
    assert!(sheet2(&renamed).contains("<f>'Sales Q1'!A1+Gamma!B2</f>"));
    assert!(sheet2(&renamed).contains(r#"val="Alpha!$A$1""#));
    let chart = part_text(&renamed.bytes, "/xl/charts/chart1.xml");
    assert!(chart.contains("<c:f>'Sales Q1'!$A$1:$A$3</c:f>"), "{chart}");
    let kept = |written: &Written| {
        let report = &written.report;
        (report.kept().iter())
            .map(|k| {
                (
                    k.part().to_owned(),
                    k.place().to_owned(),
                    k.table().map(str::to_owned),
                    k.reason(),
                )
            })
            .collect::<Vec<_>>()
    };
    let outside = (
        "/xl/worksheets/sheet2.xml".to_owned(),
        "cfvo@val".to_owned(),
        None,
        KeptReason::Outside,
    );
    assert_eq!(kept(&renamed), std::slice::from_ref(&outside));
    let op = Op::Delete("Alpha".into());
    let deleted = apply(&before, &op).unwrap();
    check("crafted", &before, &op, &deleted);
    assert!(sheet2(&deleted).contains("<f>#REF!+Gamma!B2</f>"));
    assert!(part_text(&deleted.bytes, "/xl/workbook.xml").contains(r#"fullCalcOnLoad="1""#));
    assert_eq!(kept(&deleted), [outside]);
    assert_eq!(
        defined(&deleted.bytes),
        [
            (text("Total"), None, text("#REF!")),
            (text("OfBeta"), Some(0), text("Beta!$B$1")),
            (text("Span"), None, text("Beta:Gamma!$A$1")),
        ]
    );
    assert_eq!(book(&deleted.bytes).views()[0].active_tab(), Some(1));
    let names = entries(&deleted.bytes);
    assert!(!names.contains_key("xl/calcChain.xml"));
    assert!(!names.contains_key("xl/worksheets/sheet1.xml"));
    let op = Op::Move("Beta".into(), 3);
    let moved = apply(&before, &op).unwrap();
    check("crafted", &before, &op, &moved);
    assert_eq!(
        defined(&moved.bytes)[1],
        (text("OfBeta"), Some(3), text("Beta!$B$1"))
    );
    assert!(entries(&moved.bytes).contains_key("xl/calcChain.xml"));
    // The pivot cache's source follows its sheet's rename, and a delete keeps
    // the source and reports it.
    let op = Op::Rename("Beta".into(), "Beta 2".into());
    let beta = apply(&before, &op).unwrap();
    check("crafted", &before, &op, &beta);
    let cache = part_text(&beta.bytes, "/xl/pivotCache/pivotCacheDefinition1.xml");
    assert!(cache.contains(r#"sheet="Beta 2""#), "{cache}");
    let op = Op::Delete("Beta".into());
    let gone_beta = apply(&before, &op).unwrap();
    check("crafted", &before, &op, &gone_beta);
    let cache = part_text(&gone_beta.bytes, "/xl/pivotCache/pivotCacheDefinition1.xml");
    assert!(cache.contains(r#"sheet="Beta""#), "{cache}");
    assert_eq!(
        kept(&gone_beta),
        [(
            "/xl/pivotCache/pivotCacheDefinition1.xml".to_owned(),
            "worksheetSource@sheet".to_owned(),
            None,
            KeptReason::DeletedSource
        )]
    );
    // The sheet the view points at is deleted: the next one is active.
    let op = Op::Delete("Gamma".into());
    let gone = apply(&before, &op).unwrap();
    check("crafted", &before, &op, &gone);
    assert_eq!(book(&gone.bytes).views()[0].active_tab(), Some(2));
    // A sheet added is empty and last, with the next sheetId.
    let op = Op::Add("Epsilon".into());
    let added = apply(&before, &op).unwrap();
    check("crafted", &before, &op, &added);
    let book = book(&added.bytes);
    assert_eq!(book.sheets().last().map(|sheet| sheet.sheet_id()), Some(5));
    assert!(entries(&added.bytes).contains_key("xl/worksheets/sheet5.xml"));
    // Names set and removed.
    let op = Op::Set("OfBeta".into(), Some("Beta".into()), "Beta!$C$3".into());
    let set = apply(&before, &op).unwrap();
    check("crafted", &before, &op, &set);
    assert_eq!(
        defined(&set.bytes)[1],
        (text("OfBeta"), Some(1), text("Beta!$C$3"))
    );
    let op = Op::Remove("Total".into(), None);
    let removed = apply(&before, &op).unwrap();
    check("crafted", &before, &op, &removed);
    assert_eq!(defined(&removed.bytes).len(), 2);
}

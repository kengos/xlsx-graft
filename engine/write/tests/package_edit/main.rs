//! `PackageEdit` over every workbook in the repository's `fixtures/` folder,
//! made in Excel: an edit that changes nothing writes the workbook's
//! entries as they are (names, order, compressed bytes, method and CRC-32)
//! and the same bytes every time, and a workbook with one part rewritten
//! (`docProps/app.xml`) has every other entry's compressed bytes as they
//! were, and both read back as the workbook did.
//!
//! `fixtures/` is outside the crate: the test reads the workbooks from disk
//! when it runs, and does not embed them in the test binary.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

use std::io::{BufRead, Cursor, Write};
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package, PartName};
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_write::{PackageEdit, PartRewriter, WriteError, WriteLimits};
use xlsx_graft_xml::events::{BytesText, Event};
use xlsx_graft_xml::{XmlLimits, XmlRewriter};

/// What the rewritten `docProps/app.xml` names as the application.
const APPLICATION: &str = "xlsx-graft";

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

/// The package in `bytes`, with the diagnostics found as `code place` lines.
fn open(bytes: &[u8]) -> (Package<Cursor<Vec<u8>>>, Vec<String>) {
    let mut diagnostics = Diagnostics::new();
    let package = Package::open(
        Cursor::new(bytes.to_vec()),
        &Limits::default(),
        &mut diagnostics,
    );
    let found = (diagnostics.records().iter())
        .map(|record| format!("{:?} {:?}", record.diagnostic, record.location))
        .collect();
    (package.unwrap().unwrap(), found)
}

/// Writes the workbook `bytes` with `edit` applied to its package, which
/// must give a zip.
fn written(bytes: &[u8], edit: impl FnOnce(&mut PackageEdit<'static, Cursor<Vec<u8>>>)) -> Vec<u8> {
    let (package, _) = open(bytes);
    let mut package_edit = PackageEdit::new(package);
    edit(&mut package_edit);
    let mut diagnostics = Diagnostics::new();
    let out = package_edit
        .finish(
            Vec::new(),
            &mut MemorySpools,
            &WriteLimits::default(),
            &mut diagnostics,
        )
        .unwrap();
    assert!(
        diagnostics.records().is_empty(),
        "{:?}",
        diagnostics.records()
    );
    out.unwrap()
}

/// For each entry of the zip `bytes`: its name, method, CRC-32 and stored
/// bytes (compressed, as they are in the file).
fn stored(bytes: &[u8]) -> Vec<(Vec<u8>, String, u32, Vec<u8>)> {
    let mut diagnostics = Diagnostics::new();
    let archive = xlsx_graft_package::ZipArchive::open(
        Cursor::new(bytes.to_vec()),
        &xlsx_graft_package::ZipLimits::default(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    (archive.entries().iter())
        .map(|entry| {
            let range = entry.data_range();
            let data = bytes[range.start as usize..range.end as usize].to_vec();
            let method = format!("{:?}", entry.method());
            (entry.name().to_vec(), method, entry.crc32(), data)
        })
        .collect()
}

/// A rewriter of `docProps/app.xml` that names [`APPLICATION`] as the
/// application, the text of its `Application` element. Every text event
/// inside the element is replaced, and an empty element, `<Application/>`,
/// is left as it is.
struct Application;

impl PartRewriter for Application {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        let mut rewriter = XmlRewriter::new(original, out, &XmlLimits::default());
        let mut buf = Vec::new();
        let mut inside = false;
        while let Some(event) = rewriter.next(&mut buf)? {
            match event {
                Event::Start(start) if start.local_name().as_ref() == "Application" => {
                    inside = true;
                    rewriter.write(Event::Start(start))?;
                }
                Event::Text(_) if inside => {
                    rewriter.write(Event::Text(BytesText::new(APPLICATION)))?;
                }
                Event::End(end) => {
                    inside = false;
                    rewriter.write(Event::End(end))?;
                }
                event => rewriter.write(event)?,
            }
        }
        Ok(())
    }
}

#[test]
fn an_edit_that_changes_nothing_writes_every_entry_as_it_was() {
    for path in workbooks() {
        let original = std::fs::read(&path).unwrap();
        let rewritten = written(&original, |_| {});
        let at = path.display();
        // The entries' names and order, and their compressed bytes, method
        // and CRC-32.
        assert_eq!(stored(&rewritten), stored(&original), "{at}");
        // The same bytes every time.
        assert_eq!(written(&original, |_| {}), rewritten, "{at}");
        // It reads as the workbook did.
        assert_eq!(open(&rewritten).1, open(&original).1, "{at}");
    }
}

#[test]
fn a_workbook_with_its_application_rewritten_has_every_other_entry_as_it_was() {
    let name = PartName::parse("/docProps/app.xml").unwrap();
    let mut rewritten_any = false;
    for path in workbooks() {
        let original = std::fs::read(&path).unwrap();
        let (package, _) = open(&original);
        let Some(app) = package.part_named(&name).map(|part| part.entry()) else {
            continue;
        };
        let bytes = written(&original, |edit| {
            edit.replace(&name, Box::new(Application)).unwrap();
        });
        let (before, after) = (stored(&original), stored(&bytes));
        let at = path.display();
        assert_eq!(before.len(), after.len(), "{at}");
        for (index, (then, now)) in before.iter().zip(&after).enumerate() {
            if index == app {
                assert_eq!(then.0, now.0, "{at}");
                assert_ne!(then.3, now.3, "{at}");
            } else {
                assert_eq!(then, now, "{at}: entry {index}");
            }
        }
        assert_eq!(open(&bytes).1, open(&original).1, "{at}");
        // The new part names the application, and is read back whole.
        let (mut reread, _) = open(&bytes);
        let index = reread.part_named(&name).unwrap().entry();
        let mut diagnostics = Diagnostics::new();
        let mut text = Vec::new();
        let mut entry = reread.archive().stream_entry(index).unwrap();
        loop {
            let mut chunk = [0; 4096];
            let read = reread.read_stream(&mut entry, &mut chunk, &mut diagnostics);
            let read = read.unwrap();
            if read == 0 {
                break;
            }
            text.extend_from_slice(&chunk[..read]);
        }
        assert!(diagnostics.records().is_empty(), "{at}");
        let text = String::from_utf8(text).unwrap();
        assert!(text.contains(&format!(">{APPLICATION}</")), "{at}: {text}");
        rewritten_any = true;
    }
    assert!(rewritten_any, "no workbook has docProps/app.xml");
}

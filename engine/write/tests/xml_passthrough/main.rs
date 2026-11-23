//! Every XML part of every workbook in the repository's `fixtures/` folder,
//! made in Excel, read by `XmlRewriter` and written as received, with no
//! change: the result equals the part's own bytes.
//!
//! The test is here and not in the `xlsx-graft-xml` crate, which it tests,
//! because `xlsx-graft-package` reads the zip and depends on
//! `xlsx-graft-xml`: a dev-dependency from xml to package cannot be resolved
//! when the publish is verified against crates.io. This crate depends on
//! package already, and xml does not depend on this one.
//!
//! `fixtures/` is outside the crate: the test reads the workbooks from disk
//! when it runs, and does not embed them in the test binary.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test binary: a panic is how a failing case reports"
)]

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{ZipArchive, ZipLimits};
use xlsx_graft_xml::{XmlLimits, XmlRewriter};

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

/// Whether the zip entry `name` is an XML part: `.xml`, `.rels` or `.vml`,
/// in any ASCII letter case.
fn is_xml(name: &[u8]) -> bool {
    [b".xml".as_slice(), b".rels", b".vml"]
        .iter()
        .any(|suffix| {
            name.len() >= suffix.len()
                && name[name.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
        })
}

/// `part` read and written back with no change.
fn rewrite(part: &[u8]) -> Vec<u8> {
    let mut rewriter = XmlRewriter::new(part, Vec::new(), &XmlLimits::default());
    let mut buf = Vec::new();
    while let Some(event) = rewriter.next(&mut buf).unwrap() {
        rewriter.write(event).unwrap();
    }
    rewriter.into_inner()
}

#[test]
fn every_xml_part_of_every_workbook_is_written_back_byte_for_byte() {
    let mut checked = 0;
    let mut differ = Vec::new();
    for path in workbooks() {
        let mut diagnostics = Diagnostics::new();
        let mut archive = ZipArchive::open(
            File::open(&path).unwrap(),
            &ZipLimits::default(),
            &mut diagnostics,
        )
        .unwrap()
        .unwrap();
        let names: Vec<(usize, String)> = archive
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| is_xml(entry.name()))
            .map(|(index, entry)| (index, String::from_utf8_lossy(entry.name()).into_owned()))
            .collect();
        assert!(
            !names.is_empty(),
            "{}: no XML part was checked",
            path.display()
        );
        for (index, name) in names {
            let mut part = Vec::new();
            let entry = archive
                .open_entry(index, &mut diagnostics)
                .unwrap()
                .unwrap();
            BufReader::new(entry).read_to_end(&mut part).unwrap();
            if rewrite(&part) != part {
                differ.push(format!("{}: {name}", path.display()));
            }
            checked += 1;
        }
    }
    assert!(differ.is_empty(), "rewritten differently: {differ:#?}");
    assert!(checked > 0, "no XML part was checked");
}

//! `repack`: the workbooks of `fixtures/` written again, each read back as
//! the original was, and the workbooks and folders it refuses.

use crate::{FixturesGenError, run};
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_spool::MemorySpools;
use xlsx_graft_write::{DeflateWriter, ZipWriter};

/// The repository's `fixtures/` folder.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn repack(from: &Path, to: &Path, out: &mut dyn Write) -> Result<(), FixturesGenError> {
    let args: Vec<OsString> = ["repack".as_ref(), from.as_os_str(), to.as_os_str()]
        .iter()
        .map(|arg: &&std::ffi::OsStr| (*arg).to_owned())
        .collect();
    run(&args, out)
}

/// The diagnostics reading the package at `path` gives, as text, and
/// whether it has `docProps/app.xml`.
fn read(path: &Path) -> (Vec<String>, bool, Vec<String>) {
    let mut diagnostics = Diagnostics::new();
    let file = std::fs::File::open(path).unwrap();
    let package = Package::open(file, &Limits::default(), &mut diagnostics);
    let package = package.unwrap().unwrap();
    let found = (diagnostics.records().iter())
        .map(|record| format!("{:?} {:?}", record.diagnostic, record.location))
        .collect();
    let names = (package.archive().entries().iter())
        .map(|entry| String::from_utf8_lossy(entry.name()).into_owned())
        .collect();
    (found, package.part("/docProps/app.xml").is_some(), names)
}

/// The text of `docProps/app.xml` of the workbook at `path`.
fn app_xml(path: &Path) -> String {
    let mut diagnostics = Diagnostics::new();
    let file = std::fs::File::open(path).unwrap();
    let mut archive = xlsx_graft_package::ZipArchive::open(
        file,
        &xlsx_graft_package::ZipLimits::default(),
        &mut diagnostics,
    )
    .unwrap()
    .unwrap();
    let index = archive.position(b"docProps/app.xml").unwrap();
    let mut entry = archive
        .open_entry(index, &mut diagnostics)
        .unwrap()
        .unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut entry, &mut text).unwrap();
    text
}

#[test]
fn each_workbook_of_fixtures_is_written_again_and_with_its_application_rewritten() {
    let to = tempfile::tempdir().unwrap();
    let mut out = Vec::new();
    repack(&fixtures(), to.path(), &mut out).unwrap();
    let lines = String::from_utf8(out).unwrap();
    let mut expected = Vec::new();
    let mut checked = 0;
    let mut paths: Vec<PathBuf> = std::fs::read_dir(fixtures())
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
    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let (stem, ext) = name.rsplit_once('.').unwrap();
        let (found, has_app, names) = read(&path);
        let copy = to.path().join(&name);
        expected.push(format!("{}\n", copy.display()));
        // Read as the original is, with the same entries.
        assert_eq!(
            read(&copy),
            (found.clone(), has_app, names.clone()),
            "{name}"
        );
        if has_app {
            let app = to.path().join(format!("{stem}-app.{ext}"));
            expected.push(format!("{}\n", app.display()));
            assert_eq!(read(&app), (found, true, names), "{name}");
            // Only the text of the `Application` element differs.
            let (before, after) = (app_xml(&path), app_xml(&app));
            assert!(
                after.contains("<Application>xlsx-graft</Application>"),
                "{name}: {after}"
            );
            let start = before.find("<Application>").unwrap() + "<Application>".len();
            let end = before.find("</Application>").unwrap();
            let expected = format!("{}xlsx-graft{}", &before[..start], &before[end..]);
            assert_eq!(after, expected, "{name}");
            checked += 1;
        } else {
            assert!(
                !to.path().join(format!("{stem}-app.{ext}")).exists(),
                "{name}"
            );
        }
    }
    assert!(checked > 0);
    assert_eq!(lines, expected.concat());
}

/// A package of three parts (the content types, the package
/// relationships and a workbook) and no `docProps/app.xml`.
fn without_app() -> Vec<u8> {
    let entries: [(&[u8], &str); 3] = [
        (
            b"[Content_Types].xml",
            concat!(
                r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
                r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
                r#"<Default Extension="xml" ContentType="application/xml"/></Types>"#,
            ),
        ),
        (
            b"_rels/.rels",
            concat!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
                r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>"#,
                "</Relationships>",
            ),
        ),
        (b"xl/workbook.xml", "<workbook/>"),
    ];
    let mut zip = ZipWriter::new(Vec::new());
    for (name, content) in entries {
        let mut spools = MemorySpools;
        let mut deflate = DeflateWriter::new(&mut spools, 1 << 20);
        deflate.write_all(content.as_bytes()).unwrap();
        zip.write_deflated(name, deflate.finish().unwrap()).unwrap();
    }
    zip.finish().unwrap()
}

#[test]
fn a_workbook_with_no_application_part_is_written_once_and_names_no_second_file() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    std::fs::write(from.path().join("small.xlsm"), without_app()).unwrap();
    std::fs::write(from.path().join("notes.txt"), b"not a workbook").unwrap();
    let mut out = Vec::new();
    repack(from.path(), &to.path().join("deeper"), &mut out).unwrap();
    let copy = to.path().join("deeper").join("small.xlsm");
    assert_eq!(
        String::from_utf8(out).unwrap(),
        format!("{}\n", copy.display())
    );
    assert!(!to.path().join("deeper").join("small-app.xlsm").exists());
    assert_eq!(std::fs::read(&copy).unwrap().len(), without_app().len());
}

#[test]
fn a_file_that_is_not_a_package_is_refused_with_the_diagnostics_that_say_why() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    let path = from.path().join("bad.xlsx");
    std::fs::write(&path, b"not a zip").unwrap();
    let err = repack(from.path(), to.path(), &mut Vec::new()).unwrap_err();
    assert!(matches!(&err, FixturesGenError::Repack { path: p, .. } if *p == path));
    assert_eq!(err.exit_status(), 1);
    assert_eq!(
        err.to_string(),
        format!("{}: the diagnostics are not_a_zip", path.display())
    );
}

#[test]
fn a_workbook_with_an_entry_that_cannot_be_read_is_refused() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    let mut bytes = std::fs::read(fixtures().join("table.xlsx")).unwrap();
    // Give the third entry a compression method no reader supports.
    let central = bytes
        .windows(4)
        .enumerate()
        .filter(|(_, w)| w == b"PK\x01\x02");
    let at = central.map(|(at, _)| at).nth(2).unwrap();
    bytes[at + 10] = 99;
    let path = from.path().join("broken.xlsx");
    std::fs::write(&path, bytes).unwrap();
    let err = repack(from.path(), to.path(), &mut Vec::new()).unwrap_err();
    assert!(matches!(&err, FixturesGenError::Repack { .. }), "{err}");
    let text = err.to_string();
    assert!(text.contains("write_entry_unreadable"), "{text}");
    assert!(text.contains("zip_unsupported_compression"), "{text}");
    // Nothing is left where the workbook was to be written.
    assert!(!to.path().join("broken.xlsx").exists());
}

#[test]
fn a_folder_that_is_not_there_or_a_destination_that_is_a_file_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let err = repack(&missing, dir.path(), &mut Vec::new()).unwrap_err();
    assert!(matches!(&err, FixturesGenError::Io { path, .. } if *path == missing));
    let file = dir.path().join("file");
    std::fs::write(&file, b"").unwrap();
    let err = repack(dir.path(), &file, &mut Vec::new()).unwrap_err();
    assert!(matches!(&err, FixturesGenError::Io { path, .. } if *path == file));
}

#[test]
fn a_folder_named_as_a_workbook_is_refused_as_a_package_that_cannot_be_read() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("folder.xlsx");
    std::fs::create_dir(&folder).unwrap();
    let to = dir.path().join("to");
    let err = repack(dir.path(), &to, &mut Vec::new()).unwrap_err();
    assert!(matches!(&err, FixturesGenError::Repack { path, .. } if *path == folder));
}

#[test]
fn a_workbook_that_cannot_be_written_to_its_place_is_refused() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    std::fs::write(from.path().join("small.xlsx"), without_app()).unwrap();
    // A folder is where the workbook is to be written.
    std::fs::create_dir(to.path().join("small.xlsx")).unwrap();
    let err = repack(from.path(), to.path(), &mut Vec::new()).unwrap_err();
    assert!(matches!(&err, FixturesGenError::Repack { .. }), "{err}");
    assert_eq!(err.exit_status(), 1);
}

#[test]
fn repack_needs_two_directories() {
    for words in [
        &["repack"][..],
        &["repack", "a"],
        &["repack", "a", "b", "c"],
    ] {
        let args: Vec<OsString> = words.iter().map(OsString::from).collect();
        let err = run(&args, &mut Vec::new()).unwrap_err();
        assert!(matches!(err, FixturesGenError::Usage), "{words:?}");
        assert_eq!(err.exit_status(), 2);
    }
}

/// An output whose every write returns an error.
struct Refuses;

impl Write for Refuses {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("refused"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn repack_returns_an_output_error_when_writing_a_path_to_the_output_fails() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    std::fs::write(from.path().join("small.xlsx"), without_app()).unwrap();
    let err = repack(from.path(), to.path(), &mut Refuses).unwrap_err();
    assert!(matches!(err, FixturesGenError::Output(_)));
    assert!(to.path().join("small.xlsx").exists());
}

#[test]
fn an_error_of_the_edit_other_than_a_missing_part_is_returned() {
    let to = tempfile::tempdir().unwrap();
    let source = fixtures().join("table.xlsx");
    // A Relationships part cannot be replaced.
    let book = |part| super::Book {
        source: &source,
        target: to.path().join("out.xlsx"),
        rewrite: Some(part),
    };
    let err = book("/_rels/.rels").write().unwrap_err();
    assert!(matches!(&err, FixturesGenError::Repack { path, .. } if *path == source));
    assert!(err.to_string().contains("Relationships part"), "{err}");
    // A part the workbook does not have is skipped.
    assert!(!book("/no/such.xml").write().unwrap());
    assert!(!to.path().join("out.xlsx").exists());
}

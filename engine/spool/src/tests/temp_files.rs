//! What only a [`FileSpool`] does: where [`TempFiles`] makes its files,
//! that nothing of them is left behind, appending to a file that holds
//! bytes already, and the failures of the file reaching the caller.

use crate::{FileSpool, Spool, SpoolFactory, TempFiles};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// The names in `dir`.
fn names(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn files_go_to_the_os_temp_dir_unless_one_is_named() {
    assert_eq!(TempFiles::new().dir(), std::env::temp_dir());
    assert_eq!(TempFiles::default().dir(), std::env::temp_dir());
    assert_eq!(TempFiles::in_dir("/scratch").dir(), Path::new("/scratch"));
}

#[test]
fn a_file_leaves_nothing_in_its_directory() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = TempFiles::in_dir(dir.path());
    let mut spool = files.create().unwrap();
    spool.append(b"abc").unwrap();
    // On Unix the file has no name while it is open: Linux makes it with
    // O_TMPFILE, the others unlink it at once. On Windows it would have
    // one until it is closed.
    #[cfg(unix)]
    assert_eq!(names(dir.path()), Vec::<String>::new());
    drop(spool);
    assert_eq!(names(dir.path()), Vec::<String>::new());
}

#[cfg(target_os = "linux")]
#[test]
fn a_file_is_deleted_while_open_on_linux() {
    use std::os::fd::AsRawFd;
    let spool = TempFiles::new().create_file().unwrap();
    let link = fs::read_link(format!("/proc/self/fd/{}", spool.file.as_raw_fd())).unwrap();
    assert!(link.to_string_lossy().ends_with(" (deleted)"), "{link:?}");
}

#[test]
fn a_missing_directory_makes_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = TempFiles::in_dir(dir.path().join("missing"));
    let error = files.create().err().unwrap();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert_eq!(
        files.create_file().unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
}

#[test]
fn a_file_holding_bytes_is_appended_to_after_them() {
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(b"abc").unwrap();
    let mut spool = FileSpool::new(file).unwrap();
    spool.append(b"de").unwrap();
    let mut buf = [0; 5];
    spool.read_exact_at(0, &mut buf).unwrap();
    assert_eq!(&buf, b"abcde");
}

#[test]
fn a_file_that_cannot_be_written_or_read_fails_the_call() {
    let named = tempfile::NamedTempFile::new().unwrap();
    fs::write(named.path(), b"abc").unwrap();

    let mut read_only = FileSpool::new(File::open(named.path()).unwrap()).unwrap();
    assert!(read_only.append(b"d").is_err());
    // The failed append is not counted: the end is still after "abc".
    let mut buf = [0; 1];
    let error = read_only.read_exact_at(3, &mut buf).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    read_only.read_exact_at(2, &mut buf).unwrap();
    assert_eq!(&buf, b"c");

    let write_only = OpenOptions::new().write(true).open(named.path()).unwrap();
    let mut write_only = FileSpool::new(write_only).unwrap();
    write_only.append(b"d").unwrap();
    let error = write_only.read_exact_at(0, &mut buf).unwrap_err();
    assert_ne!(error.kind(), io::ErrorKind::UnexpectedEof);
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_seek_is_no_spool() {
    use std::os::fd::OwnedFd;
    let (reader, _writer) = io::pipe().unwrap();
    let pipe = File::from(OwnedFd::from(reader));
    assert!(FileSpool::new(pipe).is_err());
}

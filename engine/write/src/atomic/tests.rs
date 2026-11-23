//! `write_file_atomically`: the file replaced only when the write
//! succeeds, and no temporary file left in any case.

use crate::{WriteError, write_file_atomically};
use std::path::Path;

/// The names in `folder`, sorted.
fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_successful_write_makes_the_file_and_replaces_one_there() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("book.xlsx");
    let result = write_file_atomically(&path, |out| {
        out.write_all(b"first")?;
        // The temporary file is in the folder of the file, to be renamed.
        let names = names(folder.path());
        assert!(
            names.iter().any(|name| name.starts_with(".xlsx-graft-")),
            "{names:?}"
        );
        Ok(Some(7))
    });
    assert_eq!(result.unwrap(), Some(7));
    assert_eq!(std::fs::read(&path).unwrap(), b"first");
    let result = write_file_atomically(&path, |out| {
        out.write_all(&vec![b'x'; 200_000])?;
        Ok(Some(()))
    });
    assert_eq!(result.unwrap(), Some(()));
    assert_eq!(std::fs::read(&path).unwrap(), vec![b'x'; 200_000]);
    assert_eq!(names(folder.path()), ["book.xlsx"]);
}

#[test]
fn a_write_that_gives_nothing_or_an_error_leaves_the_file_as_it_was_and_no_temporary_file() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("book.xlsx");
    std::fs::write(&path, b"before").unwrap();
    let result = write_file_atomically::<()>(&path, |out| {
        out.write_all(b"partial")?;
        Ok(None)
    });
    assert_eq!(result.unwrap(), None);
    let result = write_file_atomically::<()>(&path, |out| {
        out.write_all(b"partial")?;
        Err(WriteError::Unusable)
    });
    assert!(matches!(result, Err(WriteError::Unusable)));
    assert_eq!(std::fs::read(&path).unwrap(), b"before");
    assert_eq!(names(folder.path()), ["book.xlsx"]);
    // Nor is a file made where there was none.
    let new = folder.path().join("new.xlsx");
    let result = write_file_atomically::<()>(&new, |_| Ok(None));
    assert_eq!(result.unwrap(), None);
    assert_eq!(names(folder.path()), ["book.xlsx"]);
}

#[test]
fn a_folder_that_is_not_there_or_a_path_that_cannot_be_replaced_is_an_io_error() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("missing").join("book.xlsx");
    let result = write_file_atomically(&path, |_| Ok(Some(())));
    assert!(matches!(result, Err(WriteError::Io(_))), "{result:?}");
    // A folder cannot be replaced by a file.
    let taken = folder.path().join("taken");
    std::fs::create_dir(&taken).unwrap();
    let result = write_file_atomically(&taken, |out| {
        out.write_all(b"new")?;
        Ok(Some(()))
    });
    assert!(matches!(result, Err(WriteError::Io(_))), "{result:?}");
    assert_eq!(names(folder.path()), ["taken"]);
}

#[test]
fn a_path_with_no_folder_is_written_beside_it_in_the_current_directory() {
    // The root has no parent, so the temporary file is made in `.`; nothing
    // is renamed, and the temporary file is removed.
    let result = write_file_atomically::<()>(Path::new("/"), |_| Ok(None));
    assert_eq!(result.unwrap(), None);
    let before = names(Path::new("."));
    let result =
        write_file_atomically::<()>(Path::new("book-that-is-never-written.xlsx"), |_| Ok(None));
    assert_eq!(result.unwrap(), None);
    assert_eq!(names(Path::new(".")), before);
}

/// The permission bits of `path`.
#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[cfg(unix)]
#[test]
fn a_new_file_has_the_permissions_a_file_made_in_the_ordinary_way_has() {
    let folder = tempfile::tempdir().unwrap();
    // What the process's umask leaves of 0o666.
    let probe = folder.path().join("probe");
    std::fs::File::create(&probe).unwrap();
    let path = folder.path().join("book.xlsx");
    write_file_atomically(&path, |out| {
        out.write_all(b"x")?;
        Ok(Some(()))
    })
    .unwrap();
    assert_eq!(mode(&path), mode(&probe));
}

#[cfg(unix)]
#[test]
fn a_file_replaced_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("book.xlsx");
    std::fs::write(&path, b"before").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    write_file_atomically(&path, |out| {
        out.write_all(b"after")?;
        Ok(Some(()))
    })
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"after");
    assert_eq!(mode(&path), 0o640);
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_at_the_path_is_replaced_by_a_file_and_its_target_is_left_as_it_was() {
    let folder = tempfile::tempdir().unwrap();
    let target = folder.path().join("target.xlsx");
    let link = folder.path().join("link.xlsx");
    std::fs::write(&target, b"target").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    write_file_atomically(&link, |out| {
        out.write_all(b"new")?;
        Ok(Some(()))
    })
    .unwrap();
    assert!(
        !std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read(&link).unwrap(), b"new");
    assert_eq!(std::fs::read(&target).unwrap(), b"target");
}

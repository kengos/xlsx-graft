//! [`write_file_atomically`]: a file written whole or not at all.

use crate::error::WriteError;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// What a temporary file's name starts with.
const PREFIX: &str = ".xlsx-graft-";

/// Writes the file `path` by what `write` writes to a temporary file in the
/// same folder, and renames the temporary file to `path` only when `write`
/// returns `Ok(Some(_))`, so `path` holds either what it held before or the
/// whole new content, never a part of it. With `Ok(None)` or an error, the
/// temporary file is removed and `path` is as it was.
///
/// The new content is flushed and synced to disk before the rename, which
/// replaces a file `path` already names.
///
/// On Unix, before it writes, the function copies the permissions of the file
/// at `path`, if there is one, to the temporary file, and a new file gets the
/// permissions of a file made in the ordinary way (0o666 less the umask).
/// A symbolic link at `path` is replaced by a regular file: the rename
/// replaces the link, not the file it points to, and the permissions copied
/// are those of that file. On other platforms the permissions are those the
/// temporary file has, and nothing is copied: copying a read-only attribute
/// would leave a temporary file that cannot be renamed over a read-only file
/// or removed.
///
/// `write` gets the writer to pass to
/// [`PackageEdit::finish`](crate::PackageEdit::finish), and returns what it
/// returns, with the writer dropped from the value:
///
/// ```no_run
/// # use xlsx_graft_write::{write_file_atomically, PackageEdit, WriteError, WriteLimits};
/// # use xlsx_graft_diagnostics::Diagnostics;
/// # use xlsx_graft_spool::TempFiles;
/// # use std::path::Path;
/// # type Edit<'r> = PackageEdit<'r, std::io::Cursor<Vec<u8>>>;
/// # fn run(edit: Edit<'_>, path: &Path) -> Result<(), WriteError> {
/// let mut diagnostics = Diagnostics::new();
/// let limits = WriteLimits::default();
/// let written = write_file_atomically(path, |out| {
///     let finished = edit.finish(out, &mut TempFiles::new(), &limits, &mut diagnostics)?;
///     Ok(finished.map(|_| ()))
/// })?;
/// # let _ = written;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// [`WriteError::Io`] when the temporary file cannot be made, written,
/// synced or renamed; the error `write` returns.
pub fn write_file_atomically<T>(
    path: &Path,
    write: impl FnOnce(&mut dyn Write) -> Result<Option<T>, WriteError>,
) -> Result<Option<T>, WriteError> {
    let folder = path.parent().filter(|p| !p.as_os_str().is_empty());
    let folder = folder.unwrap_or_else(|| Path::new("."));
    let mut builder = tempfile::Builder::new();
    builder.prefix(PREFIX);
    // Not the 0o600 a temporary file has: a file made in the ordinary way,
    // under the umask.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o666));
    }
    let file = builder.tempfile_in(folder)?;
    // The file replaced, if there is one, keeps its permissions.
    #[cfg(unix)]
    if let Ok(existing) = fs::metadata(path) {
        file.as_file().set_permissions(existing.permissions())?;
    }
    let mut out = BufWriter::new(file);
    let Some(written) = write(&mut out)? else {
        return Ok(None);
    };
    let file = out.into_inner().map_err(io::Error::from)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|failed| failed.error)?;
    Ok(Some(written))
}

#[cfg(test)]
mod tests;

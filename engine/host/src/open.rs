//! Opening the file a path names, the same way for each function of the
//! host that takes a path.

use crate::report::Failure;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Opens the file at `path` for reading.
///
/// A path that is not a regular file (a directory, a named pipe, a device; a
/// link is followed) is not opened at all, so that opening it cannot block.
///
/// # Errors
///
/// An `io` [`Failure`] when `path` is not a regular file or cannot be
/// opened.
pub(crate) fn open_regular(path: &Path) -> Result<BufReader<File>, Failure> {
    let opened = match std::fs::metadata(path) {
        Ok(metadata) if !metadata.is_file() => {
            let message = format!("{} is not a regular file", path.display());
            return Err(Failure::io(message));
        }
        Ok(_) => File::open(path),
        Err(error) => Err(error),
    };
    match opened {
        Ok(file) => Ok(BufReader::new(file)),
        Err(error) => {
            let message = format!("{} could not be opened: {error}", path.display());
            Err(Failure::io(message))
        }
    }
}

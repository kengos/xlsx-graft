//! [`Spool`] and [`SpoolFactory`]: the storage a host can supply.

use std::io;

/// Append-only storage that reads back any range of what was appended.
///
/// A host implements it for storage of its own; the crate root says why it
/// takes an offset on every read and asks for no paths, no clock and no
/// `Send`. Bytes appended stay where they are: nothing overwrites or
/// truncates them.
pub trait Spool {
    /// Writes all of `bytes` after the bytes appended so far. If it fails,
    /// the bytes it wrote are not counted: the next append starts where
    /// this one did.
    ///
    /// # Errors
    ///
    /// When the storage fails.
    fn append(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Fills `buf` with the bytes appended from `offset` on. Any range
    /// within what was appended can be read, at any time.
    ///
    /// # Errors
    ///
    /// When the storage fails, and with `std::io::ErrorKind::UnexpectedEof`
    /// when the range runs past the bytes appended. An empty `buf` reads
    /// from any offset up to the end.
    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()>;
}

/// Makes a new, empty [`Spool`] each time it is asked: what the engine is
/// given where it may need scratch storage.
pub trait SpoolFactory {
    /// A new, empty spool.
    ///
    /// # Errors
    ///
    /// When the host cannot provide one.
    fn create(&mut self) -> io::Result<Box<dyn Spool>>;
}

/// The error [`Spool::read_exact_at`] returns for a range past the end.
pub(crate) fn past_end() -> io::Error {
    io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "the range runs past the end of the spool",
    )
}

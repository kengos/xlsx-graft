//! xlsx-graft spool: append-only scratch storage for what the engine
//! cannot keep in memory, such as a large shared-strings table read from a
//! workbook, or the rows of a large sheet being written.
//!
//! # The storage
//!
//! [`Spool`] is the one interface: [`append`](Spool::append) writes bytes
//! after the last ones, and [`read_exact_at`](Spool::read_exact_at) reads
//! a range back from where it starts. A host that has its own storage
//! implements it and hands it to the engine through a [`SpoolFactory`]: in
//! a browser, a file in the origin private file system, whose sync access
//! handle reads and writes at an offset in just this way. The crate holds
//! two:
//!
//! - [`MemorySpool`], a `Vec<u8>`, from [`MemorySpools`].
//! - [`FileSpool`], a file, from [`TempFiles`]: an unnamed temporary file
//!   in `std::env::temp_dir()` or a directory the caller names, which the
//!   OS removes when it is closed, however the process ends. On Linux it
//!   never has a name where the file system supports `O_TMPFILE`;
//!   elsewhere on Unix it loses its name as soon as it is made, and on
//!   Windows it is deleted on close. [`FileSpool`] and [`TempFiles`] are
//!   not built for WebAssembly, WASI included, where
//!   `std::env::temp_dir()` panics: a host there brings its own spool.
//!
//! What the interface leaves out keeps it open to such a host (the engine
//! will run as WebAssembly too):
//!
//! - **No paths**: a spool is bytes in and bytes out. Only [`TempFiles`]
//!   knows a directory, and a host need not use it.
//! - **No cursor**: an offset on every read rather than `Read + Seek`, so
//!   reading never moves where the next append goes, and the storage of a
//!   browser, which takes an offset on every call, maps one to one. A read
//!   still takes `&mut self`, so that storage with a cursor (a file, read
//!   by seeking) implements it as it is.
//! - **No clock and no threads**: nothing reads the time, and a spool need
//!   not be `Send` (a browser's handle is not). So a [`SpillBuffer`] is
//!   not `Send` either, nor what holds one.
//!
//! An `Err` is a failure of the host alone (its storage, or memory it could
//! not allocate): [`SpoolError`]. Nothing in a workbook reaches this crate.
//!
//! # Spilling
//!
//! [`SpillBuffer`] is the container callers use: an append-only run of
//! bytes held in memory until the caller calls
//! [`spill`](SpillBuffer::spill), which moves it to a spool the factory
//! makes, and leaves in memory only a write buffer of the size the caller
//! gives. When to spill is the caller's call, not the buffer's, since one
//! limit can cover several buffers (the shared-strings table spills its
//! text and the offsets into it together, once the two pass one threshold);
//! [`len`](SpillBuffer::len) and [`memory_bytes`](SpillBuffer::memory_bytes)
//! say what each holds. Each buffer spilled keeps a write buffer of its
//! own, so two spilled together hold two.
//!
//! A range can be read back at any time, also between appends and before
//! the write buffer reaches the spool. Reading the shared-strings table
//! needs that only once it is read to the end, but a writer looks back
//! while still writing: a table of the strings written so far is checked
//! for each new one. Allowing it costs nothing here, since a read takes its
//! offset with it and is served from the write buffer for the bytes not yet
//! in the spool.

mod error;
// No temporary directory on WebAssembly: the host brings a spool.
#[cfg(not(target_family = "wasm"))]
mod file;
mod memory;
mod spill;
mod spool;

pub use error::SpoolError;
#[cfg(not(target_family = "wasm"))]
pub use file::{FileSpool, TempFiles};
pub use memory::{MemorySpool, MemorySpools};
pub use spill::{Fill, SpillBuffer};
pub use spool::{Spool, SpoolFactory};

#[cfg(test)]
mod tests;

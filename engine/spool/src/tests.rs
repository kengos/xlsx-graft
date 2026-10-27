//! Tests for the spools and [`SpillBuffer`](crate::SpillBuffer), driven
//! through the public API. `spools` holds what every [`Spool`] does, run on
//! both of the crate's; `temp_files` what only a temporary file does;
//! `spill` the buffer, over both and over a spool that fails.

mod spill;
mod spools;
mod temp_files;

use crate::{MemorySpool, Spool, SpoolFactory};
use std::cell::Cell;
use std::io;
use std::rc::Rc;

/// A spool in memory that fails as a host's storage would, once `budget`
/// runs out: `Some(n)` answers `n` more calls and fails the rest, `None`
/// never fails.
struct Flaky {
    inner: MemorySpool,
    budget: Rc<Cell<Option<usize>>>,
}

impl Flaky {
    fn check(&self) -> io::Result<()> {
        match self.budget.get() {
            None => Ok(()),
            Some(0) => Err(io::Error::other("the disk went away")),
            Some(left) => {
                self.budget.set(Some(left - 1));
                Ok(())
            }
        }
    }
}

impl Spool for Flaky {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.check()?;
        self.inner.append(bytes)
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        self.check()?;
        self.inner.read_exact_at(offset, buf)
    }
}

/// Makes [`Flaky`] spools that share one budget, counting them, and fails
/// to make one while `refusing` is set.
#[derive(Default)]
struct FlakySpools {
    budget: Rc<Cell<Option<usize>>>,
    refusing: bool,
    created: usize,
}

impl SpoolFactory for FlakySpools {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        if self.refusing {
            return Err(io::Error::other("no room for a spool"));
        }
        self.created += 1;
        Ok(Box::new(Flaky {
            inner: MemorySpool::new(),
            budget: Rc::clone(&self.budget),
        }))
    }
}

#[test]
fn an_error_says_the_storage_failed() {
    let error = crate::SpoolError::from(io::Error::other("the disk went away"));
    assert_eq!(
        error.to_string(),
        "the spool's storage failed: the disk went away"
    );
}

//! The sheet protection a `SheetEdit` sets or removes, and whether the part
//! has a `sheetProtection`. A `sheetProtection` of the part is replaced
//! (written at its `Due`), never edited.

use crate::sheet::SheetProtection;
use std::io::{self, Write};

/// The `protect` or `unprotect` call and what the part had.
#[derive(Debug)]
pub(super) struct ProtectionEdit {
    /// Whether the part has a `sheetProtection`.
    has: bool,
    /// The last call: `Some(Some(_))` for `protect`, `Some(None)` for
    /// `unprotect`.
    request: Option<Option<SheetProtection>>,
}

impl ProtectionEdit {
    pub(super) fn new(has: bool) -> Self {
        Self { has, request: None }
    }

    pub(super) fn protect(&mut self, protection: SheetProtection) {
        self.request = Some(Some(protection));
    }

    pub(super) fn unprotect(&mut self) {
        self.request = Some(None);
    }

    /// Whether the call changes the part: `protect` does, `unprotect` does
    /// when the part has a `sheetProtection`.
    pub(super) fn changed(&self) -> bool {
        match &self.request {
            None => false,
            Some(request) => request.is_some() || self.has,
        }
    }

    /// Writes the `sheetProtection` of `protect`, nothing for `unprotect`.
    pub(super) fn write_new(&self, out: &mut dyn Write, prefix: &str) -> io::Result<()> {
        (self.request.iter().flatten()).try_for_each(|protection| protection.write(out, prefix))
    }
}

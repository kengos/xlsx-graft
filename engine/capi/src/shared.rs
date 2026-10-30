//! [`Shared`]: what a book and every rows handle opened from it share, so
//! that a binding has nothing of it to keep: the diagnostics, the first
//! failure, whether a panic poisoned them, and the rows still open, which
//! the book's close closes.

use crate::book::Reader;
use crate::message::failed;
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_read::SheetRows;

/// A sheet's rows, which the book's close takes away from its handle.
pub(crate) type Rows = RefCell<Option<SheetRows<Reader>>>;

/// A rows handle's hold on its rows.
pub(crate) type Live = Rc<Rows>;

/// Why a book or its rows stopped short, as `read`'s end line says it:
/// `{"code": ..., "message": ...}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Failure {
    /// A refusal's code (`sheet_not_found`, ...) or `io`.
    pub(crate) code: &'static str,
    /// English for a person.
    pub(crate) message: String,
}

/// The state of one book and its rows handles.
pub(crate) struct Shared {
    pub(crate) diagnostics: RefCell<Diagnostics>,
    /// The first failure; later ones keep it.
    failure: RefCell<Option<Failure>>,
    /// Set when a call on the book or its rows panicked.
    poisoned: Cell<bool>,
    /// The rows handles' rows, while their handles live.
    rows: RefCell<Vec<Weak<Rows>>>,
}

impl Shared {
    pub(crate) fn new(diagnostics: Diagnostics) -> Self {
        Self {
            diagnostics: RefCell::new(diagnostics),
            failure: RefCell::new(None),
            poisoned: Cell::new(false),
            rows: RefCell::new(Vec::new()),
        }
    }

    /// Keeps `failure` unless one is kept already.
    pub(crate) fn fail(&self, code: &'static str, message: String) {
        if let Ok(mut kept) = self.failure.try_borrow_mut() {
            kept.get_or_insert(Failure { code, message });
        }
    }

    /// The first failure, if any.
    pub(crate) fn failure(&self) -> Option<Failure> {
        self.failure.try_borrow().ok().and_then(|kept| kept.clone())
    }

    /// Marks the book and its rows as left half-changed by a panic.
    pub(crate) fn poison(&self) {
        self.poisoned.set(true);
    }

    pub(crate) fn is_poisoned(&self) -> bool {
        self.poisoned.get()
    }

    /// Keeps a rows handle's rows for the book's close to take, and forgets
    /// those whose handles are gone.
    ///
    /// # Errors
    ///
    /// [`XLSX_GRAFT_ERR_IO`](crate::XLSX_GRAFT_ERR_IO) were the list
    /// borrowed, which no call re-entering another can make it.
    pub(crate) fn track(&self, rows: &Live) -> Result<(), i32> {
        let mut tracked = self.rows.try_borrow_mut().map_err(failed)?;
        tracked.retain(|weak| weak.strong_count() > 0);
        tracked.push(Rc::downgrade(rows));
        Ok(())
    }

    /// Takes every open rows handle's rows, closing the part each reads and
    /// so the file: what is left of each handle reads no more.
    pub(crate) fn close_rows(&self) {
        let tracked = (self.rows.try_borrow_mut())
            .map(|mut tracked| std::mem::take(&mut *tracked))
            .unwrap_or_default();
        for live in tracked.iter().filter_map(Weak::upgrade) {
            if let Ok(mut rows) = live.try_borrow_mut() {
                rows.take();
            }
        }
    }
}

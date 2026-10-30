//! [`BookHandle`]: a C ABI book handle that a [`Book`](super::Book) and
//! every [`Rows`](super::Rows) opened from it share.
//!
//! Closing a book closes its rows, so rows hold their book: the handle is
//! closed by the book's `close`, or when the last of them is dropped, as
//! the garbage collector finalizes their objects, never while rows read on.

use std::cell::Cell;
use std::rc::Rc;
use xlsx_graft_capi::{XlsxGraftBook, xlsx_graft_book_close};

/// A book handle, null once closed.
#[derive(Debug)]
pub(super) struct BookHandle(Cell<*mut XlsxGraftBook>);

impl BookHandle {
    /// Takes `handle`, live or null, to close once.
    pub(super) fn new(handle: *mut XlsxGraftBook) -> Rc<Self> {
        Rc::new(Self(Cell::new(handle)))
    }

    /// The handle, null once closed.
    pub(super) fn get(&self) -> *mut XlsxGraftBook {
        self.0.get()
    }

    /// Closes the handle; every call after passes a null one, which the C
    /// ABI refuses.
    pub(super) fn close(&self) {
        let handle = self.0.replace(std::ptr::null_mut());
        // SAFETY: the handle is live or null, and nulled above so it is
        // closed once. A close cannot fail but by a panic, after which the
        // book is closed as far as it will be.
        unsafe { xlsx_graft_book_close(handle) };
    }
}

impl Drop for BookHandle {
    fn drop(&mut self) {
        self.close();
    }
}

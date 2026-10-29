//! [`Rows`]: a C ABI rows handle, owned, and each row it lends copied out
//! as [`Cell`]s.
//!
//! A cell is flat: its type, the JSON Lines name, and the fields that type
//! fills, the rest `None`. A date or a time stays in parts; which
//! JavaScript type it becomes, and in which time zone, is the JS SDK's to
//! decide.

use super::{message, owned};
use xlsx_graft_capi::{
    XLSX_GRAFT_OK, XLSX_GRAFT_VALUE_BOOLEAN, XLSX_GRAFT_VALUE_DATE, XLSX_GRAFT_VALUE_DATETIME,
    XLSX_GRAFT_VALUE_ELAPSED, XLSX_GRAFT_VALUE_EMPTY, XLSX_GRAFT_VALUE_ERROR,
    XLSX_GRAFT_VALUE_NUMBER, XLSX_GRAFT_VALUE_STRING, XLSX_GRAFT_VALUE_TIME, XlsxGraftBook,
    XlsxGraftRows, XlsxGraftValue, xlsx_graft_rows_close, xlsx_graft_rows_next,
    xlsx_graft_rows_open, xlsx_graft_rows_open_name,
};

/// The rows of one sheet, opened through the C ABI, closed when dropped.
/// Not [`Send`], as [`Book`](super::Book).
#[derive(Debug)]
pub struct Rows {
    handle: *mut XlsxGraftRows,
    /// The open's status.
    pub status: i32,
}

/// What one read of a row came to.
#[derive(Debug, Clone, PartialEq)]
pub struct Next {
    /// The C ABI's status.
    pub status: i32,
    /// Why the host failed, when `status` is `XLSX_GRAFT_ERR_IO`.
    pub message: Option<String>,
    /// The row, `None` at the end of the sheet or on a status other than
    /// `XLSX_GRAFT_OK`.
    pub row: Option<Row>,
}

/// One row the sheet writes.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Its number, from 1.
    pub number: u32,
    /// Its values, from column A to its last cell.
    pub cells: Vec<Cell>,
}

/// One cell's value. `kind` is the type `read --format json` names it,
/// and only the fields that type fills are set.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cell {
    /// `empty`, `string`, `number`, `boolean`, `date`, `time`,
    /// `datetime`, `elapsed`, `error`, or `unknown` for a kind newer than
    /// this addon.
    pub kind: &'static str,
    /// A string's or an error value's text.
    pub text: Option<String>,
    /// A number's value.
    pub number: Option<f64>,
    /// A boolean's value.
    pub boolean: Option<bool>,
    /// A date's (or a date-time's) year, month and day.
    pub date: Option<[u32; 3]>,
    /// A time's (or a date-time's) hour, minute, second and millisecond.
    pub time: Option<[u32; 4]>,
    /// An elapsed time, in milliseconds: a whole number at most about
    /// 3.2e14, exact in a double.
    pub milliseconds: Option<f64>,
}

impl Rows {
    /// Opens the rows of the sheet at `index` of `book`.
    pub(super) fn open_index(book: *mut XlsxGraftBook, index: usize) -> Self {
        let mut handle = std::ptr::null_mut();
        // SAFETY: `book` is a live handle or null; `handle` a writable slot.
        let status = unsafe { xlsx_graft_rows_open(book, index, &raw mut handle) };
        Self {
            handle: owned(status, handle),
            status,
        }
    }

    /// Opens the rows of the sheet of `book` named `name`.
    pub(super) fn open_name(book: *mut XlsxGraftBook, name: &str) -> Self {
        let mut handle = std::ptr::null_mut();
        // SAFETY: `book` is a live handle or null, `name` alive for the
        // call, `handle` a writable slot.
        let status =
            unsafe { xlsx_graft_rows_open_name(book, name.as_ptr(), name.len(), &raw mut handle) };
        Self {
            handle: owned(status, handle),
            status,
        }
    }

    /// Reads the next row, copied out.
    #[must_use]
    pub fn next_row(&mut self) -> Next {
        let (mut number, mut values, mut len) = (0, std::ptr::null(), 0);
        // SAFETY: the handle is live or null; the out pointers are
        // writable slots.
        let status = unsafe {
            xlsx_graft_rows_next(self.handle, &raw mut number, &raw mut values, &raw mut len)
        };
        let message = message(status);
        if status != XLSX_GRAFT_OK || number == 0 {
            return Next {
                status,
                message,
                row: None,
            };
        }
        // An empty row's values are not to be read (the header).
        let cells = if len == 0 {
            Vec::new()
        } else {
            // SAFETY: on OK with a row, the library lent `len` values alive
            // until the next call on the handle.
            let lent = unsafe { std::slice::from_raw_parts(values, len) };
            // SAFETY: each value's text is alive as long as the values.
            lent.iter()
                .map(|value| unsafe { Cell::of(value) })
                .collect()
        };
        Next {
            status,
            message,
            row: Some(Row { number, cells }),
        }
    }

    /// Closes the rows; every call after is refused as a null handle's.
    pub fn close(&mut self) {
        let handle = std::mem::replace(&mut self.handle, std::ptr::null_mut());
        // SAFETY: the handle is live or null, and nulled above so it is
        // closed once.
        unsafe { xlsx_graft_rows_close(handle) };
    }
}

impl Drop for Rows {
    fn drop(&mut self) {
        self.close();
    }
}

impl Cell {
    /// `value`, copied out.
    ///
    /// # Safety
    ///
    /// A non-null `value.text` must point at `value.text_len` bytes alive
    /// for the call.
    unsafe fn of(value: &XlsxGraftValue) -> Self {
        let date = [value.year.into(), value.month.into(), value.day.into()];
        let time = [
            value.hour.into(),
            value.minute.into(),
            value.second.into(),
            value.millisecond.into(),
        ];
        let text = (!value.text.is_null()).then(|| {
            // SAFETY: the caller's contract.
            let bytes = unsafe { std::slice::from_raw_parts(value.text, value.text_len) };
            String::from_utf8_lossy(bytes).into_owned()
        });
        let cell = Self::default();
        match value.kind {
            XLSX_GRAFT_VALUE_EMPTY => Self {
                kind: "empty",
                ..cell
            },
            XLSX_GRAFT_VALUE_STRING => Self {
                kind: "string",
                text,
                ..cell
            },
            XLSX_GRAFT_VALUE_NUMBER => Self {
                kind: "number",
                number: Some(value.number),
                ..cell
            },
            XLSX_GRAFT_VALUE_BOOLEAN => Self {
                kind: "boolean",
                boolean: Some(value.boolean != 0),
                ..cell
            },
            XLSX_GRAFT_VALUE_DATE => Self {
                kind: "date",
                date: Some(date),
                ..cell
            },
            XLSX_GRAFT_VALUE_TIME => Self {
                kind: "time",
                time: Some(time),
                ..cell
            },
            XLSX_GRAFT_VALUE_DATETIME => Self {
                kind: "datetime",
                date: Some(date),
                time: Some(time),
                ..cell
            },
            // At most about 3.2e14: exact in a double.
            XLSX_GRAFT_VALUE_ELAPSED => Self {
                kind: "elapsed",
                milliseconds: Some(value.elapsed as f64),
                ..cell
            },
            XLSX_GRAFT_VALUE_ERROR => Self {
                kind: "error",
                text,
                ..cell
            },
            _ => Self {
                kind: "unknown",
                ..cell
            },
        }
    }
}

#[cfg(test)]
mod tests;

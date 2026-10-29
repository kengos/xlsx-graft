//! [`XlsxGraftValue`]: one cell's value as it crosses the C ABI, and the
//! kinds it can be.
//!
//! The kinds are the nine types of `read --format json`, in the order its
//! help lists them, and each carries what that type's JSON value says: an
//! elapsed time is whole milliseconds, not wrapped at a day; a date, a time
//! or a date-time is the workbook's wall-clock time, in parts. So every SDK
//! reads a cell the same way, whether it reads the C ABI or the CLI.
//!
//! The layout is part of the ABI and has no padding the compiler chose: the
//! header gives each field's offset, and `tests/capi/layout.rs` checks them.

use xlsx_graft_read::{Date, Time, Value};

/// An empty cell, or a column with no cell between two that have one.
pub const XLSX_GRAFT_VALUE_EMPTY: u32 = 0;
/// A string: `text`.
pub const XLSX_GRAFT_VALUE_STRING: u32 = 1;
/// A number: `number`.
pub const XLSX_GRAFT_VALUE_NUMBER: u32 = 2;
/// A boolean: `boolean`, 0 or 1.
pub const XLSX_GRAFT_VALUE_BOOLEAN: u32 = 3;
/// A date: `year`, `month`, `day`.
pub const XLSX_GRAFT_VALUE_DATE: u32 = 4;
/// A time of day: `hour`, `minute`, `second`, `millisecond`.
pub const XLSX_GRAFT_VALUE_TIME: u32 = 5;
/// A date and a time of day: the fields of both.
pub const XLSX_GRAFT_VALUE_DATETIME: u32 = 6;
/// A span of time: `elapsed`, in milliseconds.
pub const XLSX_GRAFT_VALUE_ELAPSED: u32 = 7;
/// An error value as written, such as `#N/A`: `text`.
pub const XLSX_GRAFT_VALUE_ERROR: u32 = 8;

/// One cell's value. The fields its `kind` does not name are zero, and
/// `text` is null.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XlsxGraftValue {
    /// One of the `XLSX_GRAFT_VALUE_*` kinds.
    pub kind: u32,
    /// The year, 1900 to 9999 (1904 to 9999 in a workbook counting from
    /// 1904).
    pub year: u16,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month, 1 to 31.
    pub day: u8,
    /// The hour, 0 to 23.
    pub hour: u8,
    /// The minute, 0 to 59.
    pub minute: u8,
    /// The second, 0 to 59.
    pub second: u8,
    /// A boolean's value, 0 or 1.
    pub boolean: u8,
    /// The millisecond, 0 to 999.
    pub millisecond: u16,
    /// Zero; it keeps `number` at its alignment without a gap the compiler
    /// would choose.
    pub reserved: u16,
    /// A number's value, always finite.
    pub number: f64,
    /// An elapsed time, in milliseconds.
    pub elapsed: u64,
    /// The UTF-8 bytes of a string or an error value, not NUL-terminated.
    pub text: *const u8,
    /// How many bytes `text` points at.
    pub text_len: usize,
}

impl XlsxGraftValue {
    /// `value`, its text lent from where `value` borrows it.
    pub(crate) fn of(value: Value<'_>) -> Self {
        let mut out = Self {
            kind: XLSX_GRAFT_VALUE_EMPTY,
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            boolean: 0,
            millisecond: 0,
            reserved: 0,
            number: 0.0,
            elapsed: 0,
            text: std::ptr::null(),
            text_len: 0,
        };
        match value {
            Value::Empty => {}
            Value::String(text) => out.text(XLSX_GRAFT_VALUE_STRING, text),
            Value::Number(number) => {
                out.kind = XLSX_GRAFT_VALUE_NUMBER;
                out.number = number;
            }
            Value::Bool(boolean) => {
                out.kind = XLSX_GRAFT_VALUE_BOOLEAN;
                out.boolean = u8::from(boolean);
            }
            Value::Date(date) => {
                out.kind = XLSX_GRAFT_VALUE_DATE;
                out.date(date);
            }
            Value::Time(time) => {
                out.kind = XLSX_GRAFT_VALUE_TIME;
                out.time(time);
            }
            Value::DateTime(at) => {
                out.kind = XLSX_GRAFT_VALUE_DATETIME;
                out.date(at.date());
                out.time(at.time());
            }
            Value::Elapsed(elapsed) => {
                out.kind = XLSX_GRAFT_VALUE_ELAPSED;
                out.elapsed = elapsed.milliseconds();
            }
            Value::Error(text) => out.text(XLSX_GRAFT_VALUE_ERROR, text),
        }
        out
    }

    fn text(&mut self, kind: u32, text: &str) {
        self.kind = kind;
        self.text = text.as_ptr();
        self.text_len = text.len();
    }

    fn date(&mut self, date: Date) {
        self.year = date.year();
        self.month = date.month();
        self.day = date.day();
    }

    fn time(&mut self, time: Time) {
        self.hour = time.hour();
        self.minute = time.minute();
        self.second = time.second();
        self.millisecond = time.millisecond();
    }
}

#[cfg(test)]
mod tests;

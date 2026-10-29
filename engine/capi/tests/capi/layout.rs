//! The structs a binding declares on its side have the size and the field
//! offsets `include/xlsx_graft.h` gives, on a 64-bit target as on a 32-bit
//! one.

use std::mem::{offset_of, size_of};
use xlsx_graft_capi::{XlsxGraftSheet, XlsxGraftValue};

/// The width of a pointer and of a `size_t`.
const WORD: usize = size_of::<usize>();

#[test]
fn a_value_is_laid_out_as_the_header_says() {
    let offsets = [
        offset_of!(XlsxGraftValue, kind),
        offset_of!(XlsxGraftValue, year),
        offset_of!(XlsxGraftValue, month),
        offset_of!(XlsxGraftValue, day),
        offset_of!(XlsxGraftValue, hour),
        offset_of!(XlsxGraftValue, minute),
        offset_of!(XlsxGraftValue, second),
        offset_of!(XlsxGraftValue, boolean),
        offset_of!(XlsxGraftValue, millisecond),
        offset_of!(XlsxGraftValue, reserved),
        offset_of!(XlsxGraftValue, number),
        offset_of!(XlsxGraftValue, elapsed),
        offset_of!(XlsxGraftValue, text),
        offset_of!(XlsxGraftValue, text_len),
    ];
    assert_eq!(
        offsets,
        [0, 4, 6, 7, 8, 9, 10, 11, 12, 14, 16, 24, 32, 32 + WORD]
    );
    assert_eq!(size_of::<XlsxGraftValue>(), 32 + 2 * WORD);
}

#[test]
fn a_sheet_is_laid_out_as_the_header_says() {
    let offsets = [
        offset_of!(XlsxGraftSheet, name),
        offset_of!(XlsxGraftSheet, name_len),
        offset_of!(XlsxGraftSheet, kind),
        offset_of!(XlsxGraftSheet, state),
    ];
    assert_eq!(offsets, [0, WORD, 2 * WORD, 2 * WORD + 4]);
    assert_eq!(size_of::<XlsxGraftSheet>(), 2 * WORD + 8);
}

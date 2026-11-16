//! The short forms the text writes: a relationship type's last segment, a
//! media type without its common start, and a size with commas.

/// What the media types of a workbook start with, longest first, so that
/// the text can leave them out.
const CONTENT_TYPE_PREFIXES: [&str; 4] = [
    "application/vnd.openxmlformats-officedocument.spreadsheetml.",
    "application/vnd.openxmlformats-officedocument.",
    "application/vnd.openxmlformats-package.",
    "application/vnd.ms-",
];

/// The last segment of a relationship type's URI; the whole type when that
/// segment is empty.
pub(super) fn short_type(rel_type: &str) -> &str {
    match rel_type.rsplit('/').next() {
        Some(last) if !last.is_empty() => last,
        _ => rel_type,
    }
}

/// `content_type` without the start its workbook media types share.
pub(super) fn short_content_type(content_type: &str) -> &str {
    (CONTENT_TYPE_PREFIXES.iter())
        .find_map(|prefix| content_type.strip_prefix(prefix))
        .unwrap_or(content_type)
}

/// `number` with a comma between each three digits.
pub(super) fn grouped(number: u64) -> String {
    let digits = number.to_string();
    let count = digits.len();
    let mut out = String::new();
    for (position, digit) in digits.chars().enumerate() {
        if position > 0 && count.saturating_sub(position) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

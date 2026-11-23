//! Start tags the rewriter writes again: the name of one, and one with the
//! value of its `count` attribute changed and every other byte as it was.

use xlsx_graft_xml::events::BytesStart;

/// The name of the element `start`, as written.
pub(super) fn name_of(start: &BytesStart<'_>) -> String {
    start.name().as_ref().to_owned()
}

/// `start` with the value of its `count` attribute written as `total`: the
/// quotes, the white space and the other attributes are the bytes they were.
/// A tag with no `count` is as it was.
pub(in crate::edit) fn counted(start: &BytesStart<'_>, total: usize) -> BytesStart<'static> {
    let content: &str = start;
    let name_len = start.name().as_ref().len();
    let counted = count_value(content, name_len).map_or_else(
        || content.to_owned(),
        |value| {
            format!(
                "{}{total}{}",
                &content[..value.start],
                &content[value.end..]
            )
        },
    );
    BytesStart::from_content(counted, name_len)
}

/// Where the value of the attribute `count` is in `content`, the text of a
/// start tag between `<` and `>`, whose name is `name_len` bytes long: the
/// bytes between its quotes.
fn count_value(content: &str, name_len: usize) -> Option<std::ops::Range<usize>> {
    let mut offset = name_len;
    loop {
        let rest = &content[offset..];
        let key_at = offset + rest.len() - rest.trim_start().len();
        let rest = &content[key_at..];
        let equals = rest.find('=')?;
        let key = rest[..equals].trim_end();
        let after = rest[equals + 1..].trim_start();
        let quote = after.chars().next()?;
        let value_at = content.len() - after.len() + quote.len_utf8();
        let value_len = content[value_at..].find(quote)?;
        if key == "count" {
            return Some(value_at..value_at + value_len);
        }
        offset = value_at + value_len + quote.len_utf8();
    }
}

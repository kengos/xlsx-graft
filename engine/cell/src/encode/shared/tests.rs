//! Tests for `SharedStringsWriter`: the part's frame, the numbers, and the
//! failures of the writer.

use super::SharedStringsWriter;
use crate::encode::tests::Short;
use crate::{CellError, Written};

const HEAD: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
);

#[test]
fn an_empty_part_is_the_frame_alone() {
    let part = SharedStringsWriter::new(Vec::new())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(String::from_utf8(part).unwrap(), format!("{HEAD}</sst>"));
}

#[test]
fn strings_are_numbered_from_0_and_duplicates_are_not_merged() {
    let mut strings = SharedStringsWriter::new(Vec::new()).unwrap();
    assert_eq!(strings.push("a").unwrap(), (0, Written::Whole));
    assert_eq!(strings.push(" b&").unwrap(), (1, Written::Whole));
    assert_eq!(strings.push("a").unwrap(), (2, Written::Whole));
    let part = String::from_utf8(strings.finish().unwrap()).unwrap();
    assert_eq!(
        part,
        format!(
            "{HEAD}<si><t>a</t></si><si><t xml:space=\"preserve\"> b&amp;</t></si><si><t>a</t></si></sst>"
        )
    );
    assert!(!part.contains("count") && !part.contains("uniqueCount"));
}

#[test]
fn a_string_past_the_limit_is_cut_and_returns_truncated() {
    let mut strings = SharedStringsWriter::new(Vec::new()).unwrap();
    let (number, written) = strings.push(&"a".repeat(32_768)).unwrap();
    assert_eq!((number, written), (0, Written::Truncated));
    let part = String::from_utf8(strings.finish().unwrap()).unwrap();
    // Without the `a` of the XML declaration and the namespace in HEAD, the
    // part holds the first 32,767 `a` of the string.
    assert_eq!(
        part.matches('a').count() - HEAD.matches('a').count(),
        32_767
    );
}

#[test]
fn a_failing_writer_is_an_io_error_at_each_step() {
    let head = HEAD.len();
    assert!(matches!(
        SharedStringsWriter::new(Short { room: 0 }),
        Err(CellError::Io(_))
    ));
    assert!(matches!(
        SharedStringsWriter::new(Short { room: 10 }),
        Err(CellError::Io(_))
    ));
    // The declaration and its line feed fit exactly; the `sst` start tag
    // does not.
    let declaration = HEAD.find("<sst").unwrap();
    for room in [declaration, declaration + 12] {
        let result = SharedStringsWriter::new(Short { room });
        assert!(matches!(result, Err(CellError::Io(_))), "{room}");
    }
    // The head is written; `<si>`, `<t>`, `a`, `</t>` and `</si>` each
    // fail in turn.
    for room in [head, head + 4, head + 7, head + 8, head + 12] {
        let mut strings = SharedStringsWriter::new(Short { room }).unwrap();
        assert!(matches!(strings.push("a"), Err(CellError::Io(_))), "{room}");
    }
    let strings = SharedStringsWriter::new(Short { room: head }).unwrap();
    assert!(matches!(strings.finish(), Err(CellError::Io(_))));
}

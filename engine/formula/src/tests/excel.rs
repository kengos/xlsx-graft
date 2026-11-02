//! Formulas as the specifications show them stored: the examples of
//! ECMA-376 Part 1 §18.17 and [MS-XLSX] §2.2.2, read whole.

use super::calls;
use crate::{ReferenceKind, analyze};

#[test]
fn the_specifications_examples_read_as_they_say() {
    // §18.17.2.3: an external reference as stored in `<f>`.
    let read = analyze("SUM([1]Sheet1!$A$1:$A$3)");
    assert_eq!(calls("SUM([1]Sheet1!$A$1:$A$3)"), ["SUM"]);
    let at: Vec<_> = read
        .facts
        .references
        .iter()
        .map(|r| {
            (
                r.location.book.as_deref(),
                r.location.sheet.as_deref(),
                r.target.as_str(),
            )
        })
        .collect();
    assert_eq!(
        at,
        [
            (Some("1"), Some("Sheet1"), "$A$1"),
            (Some("1"), Some("Sheet1"), "$A$3")
        ]
    );
    // §18.17.2.3: a 3-D reference.
    assert_eq!(calls("SUM(Sheet2:Sheet13!B5)"), ["SUM"]);
    // §18.17.2.2: the union of two areas, in grouping parentheses.
    let read = analyze("SUM((B5:B15,D5:D15))");
    assert_eq!(read.facts.references.len(), 4);
    // [MS-XLSX] §2.2.2: a function only a worksheet allows, and a
    // structured reference from one column to another.
    assert_eq!(
        calls("_xlfn._xlws.SORT(myData[[Gender]:[Age]])"),
        ["_xlfn._xlws.SORT"]
    );
    let read = analyze("_xlfn._xlws.SORT(myData[[Gender]:[Age]])");
    let kinds: Vec<_> = read.facts.references.iter().map(|r| r.kind).collect();
    assert_eq!(kinds, [ReferenceKind::Table]);
}

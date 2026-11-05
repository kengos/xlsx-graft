//! Formulas as stored: the examples of ECMA-376-1:2016 §18.17 and
//! [MS-XLSX] §2.2.2, and formulas Excel for Mac saved, read whole.

use super::{calls, unseen};
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

/// A formula Excel saved, with the facts it states; the facts are
/// lists of what `calls`, `references` and `unseen` write, and of the
/// names and parameters.
struct Saved {
    formula: &'static str,
    calls: &'static [&'static str],
    /// Each as its kind, then its book in `[]` and its sheet and `!`
    /// when it has them, then its target.
    references: &'static [&'static str],
    names: &'static [&'static str],
    params: &'static [&'static str],
    unseen: &'static [&'static str],
}

/// A formula that states nothing, to fill the facts a case leaves out.
const NONE: Saved = Saved {
    formula: "",
    calls: &[],
    references: &[],
    names: &[],
    params: &[],
    unseen: &[],
};

/// Formulas Excel for Mac (`AppVersion` 16.0300 in `docProps/app.xml`)
/// wrote back after opening and saving them: `<f>` of a cell unless the
/// comment names another place.
const SAVED: &[Saved] = &[
    // A function only a worksheet allows: `_xlfn.` first, `_xlws.` after.
    Saved {
        formula: "_xlfn._xlws.FILTER(A1:A3,B1:B3>1)",
        calls: &["_xlfn._xlws.FILTER"],
        references: &["Cell A1", "Cell A3", "Cell B1", "Cell B3"],
        ..NONE
    },
    Saved {
        formula: "_xlfn._xlws.SORT(A1:A3)",
        calls: &["_xlfn._xlws.SORT"],
        references: &["Cell A1", "Cell A3"],
        ..NONE
    },
    Saved {
        formula: "_xlfn.SEQUENCE(3)",
        calls: &["_xlfn.SEQUENCE"],
        ..NONE
    },
    // A lambda called where it is written, and LET's names.
    Saved {
        formula: "_xlfn.LAMBDA(_xlpm.x,_xlpm.x+1)(A1)",
        calls: &["_xlfn.LAMBDA"],
        references: &["Cell A1"],
        params: &["X", "X"],
        ..NONE
    },
    Saved {
        formula: "_xlfn.LET(_xlpm.v,A1,_xlpm.v*2)",
        calls: &["_xlfn.LET"],
        references: &["Cell A1"],
        params: &["V", "V"],
        ..NONE
    },
    // A function passed as a value, and a lambda passed to MAP.
    Saved {
        formula: "_xlfn.BYROW(A1:B3,_xleta.SUM)",
        calls: &["_xlfn.BYROW", "_xleta.SUM*"],
        references: &["Cell A1", "Cell B3"],
        ..NONE
    },
    Saved {
        formula: "_xlfn.MAP(A1:A3,_xlfn.LAMBDA(_xlpm.x,_xlpm.x*2))",
        calls: &["_xlfn.MAP", "_xlfn.LAMBDA"],
        references: &["Cell A1", "Cell A3"],
        params: &["X", "X"],
        ..NONE
    },
    // `definedName` MyLambda, and the cell that calls it by name.
    Saved {
        formula: "_xlfn.LAMBDA(_xlpm.x,_xlpm.x*2)",
        calls: &["_xlfn.LAMBDA"],
        params: &["X", "X"],
        ..NONE
    },
    Saved {
        formula: "MyLambda(A1)",
        calls: &["MYLAMBDA"],
        references: &["Cell A1"],
        ..NONE
    },
    // `definedName` Rate, and the cell that reads it.
    Saved {
        formula: "Sheet1!$A$1",
        references: &["Cell Sheet1!$A$1"],
        ..NONE
    },
    Saved {
        formula: "LOG10(100)+Rate",
        calls: &["LOG10"],
        names: &["RATE"],
        ..NONE
    },
    // Column names with `"` and `[`; a `'` before `[`, `]` or `'` escapes it.
    Saved {
        formula: "SUM(T[a\"b])",
        calls: &["SUM"],
        references: &["Table T"],
        ..NONE
    },
    Saved {
        formula: "SUM(T[x'[y']])",
        calls: &["SUM"],
        references: &["Table T"],
        ..NONE
    },
    // The table's calculated column, in its cells: `[@a"b]` is saved as
    // `[#This Row]`, and `#REF!` stands for a column name written wrong.
    Saved {
        formula: "#REF!&T[[#This Row],[a\"b]]",
        references: &["Table T"],
        ..NONE
    },
    Saved {
        formula: "SUM(#REF!)",
        calls: &["SUM"],
        ..NONE
    },
    // Quoted sheet names, and a 3-D reference.
    Saved {
        formula: "'it''s'!A1+'a b'!A1",
        references: &["Cell it's!A1", "Cell a b!A1"],
        ..NONE
    },
    Saved {
        formula: "SUM(Sheet1:Sheet2!A1)",
        calls: &["SUM"],
        references: &["Cell Sheet1:Sheet2!A1"],
        ..NONE
    },
    // Excel wrote it with `ca="1"` on the `<f>`; the formula alone is read here.
    Saved {
        formula: "_xlfn.WEBSERVICE(\"https://example.com\")",
        calls: &["_xlfn.WEBSERVICE"],
        ..NONE
    },
    // `J1#` is saved as ANCHORARRAY, and `@A1:A3` as `A1:A3`, with no
    // `_xlfn.SINGLE`.
    Saved {
        formula: "SUM(_xlfn.ANCHORARRAY(J1))",
        calls: &["SUM", "_xlfn.ANCHORARRAY"],
        references: &["Cell J1"],
        ..NONE
    },
    Saved {
        formula: "A1:A3",
        references: &["Cell A1", "Cell A3"],
        ..NONE
    },
    // The `SUM(` in the string is no call; the scan blind to quotes
    // reports it unseen.
    Saved {
        formula: "IF(TRUE,\"SUM(\"\"x\"\")\",0)",
        calls: &["IF"],
        unseen: &["SUM"],
        ..NONE
    },
    // An array constant, its rows split by `;`.
    Saved {
        formula: "SUM({1,2;3,4})",
        calls: &["SUM"],
        ..NONE
    },
    // `<formula>` of a conditional format, `<formula1>` of a validation.
    Saved {
        formula: "A1>1",
        references: &["Cell A1"],
        ..NONE
    },
    Saved {
        formula: "ISNUMBER(K1)",
        calls: &["ISNUMBER"],
        references: &["Cell K1"],
        ..NONE
    },
];

/// The references in `formula`, each as `Saved` writes one.
fn references(formula: &str) -> Vec<String> {
    analyze(formula)
        .facts
        .references
        .iter()
        .map(|r| {
            let book = r
                .location
                .book
                .as_deref()
                .map_or(String::new(), |book| format!("[{book}]"));
            let sheet = r
                .location
                .sheet
                .as_deref()
                .map_or(String::new(), |sheet| format!("{sheet}!"));
            format!("{:?} {book}{sheet}{}", r.kind, r.target)
        })
        .collect()
}

#[test]
fn formulas_excel_for_mac_saved_read_whole() {
    for case in SAVED {
        let formula = case.formula;
        let read = analyze(formula);
        assert!(read.facts.malformed.is_empty(), "{formula}");
        assert_eq!(calls(formula), case.calls, "{formula}");
        assert_eq!(references(formula), case.references, "{formula}");
        let names: Vec<_> = read.facts.names.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, case.names, "{formula}");
        let params: Vec<_> = read.facts.params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(params, case.params, "{formula}");
        assert_eq!(unseen(formula), case.unseen, "{formula}");
        assert!(read.facts.dde_links.is_empty(), "{formula}");
        assert!(read.facts.external_books.is_empty(), "{formula}");
    }
}

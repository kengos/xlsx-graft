//! The ways a pattern scan is led past a call, one case a row: what the
//! lexer calls, and what the safety net adds when the lexer is right.

use super::{calls, unseen};
use crate::{Location, MalformedReason, ReferenceKind, analyze};

#[test]
fn quotes_that_hide_a_call_from_a_pattern_hide_none_from_the_lexer() {
    let cases: [(&str, &[&str]); 16] = [
        // `'` escapes the next character in a column name.
        (r#"T[a'#]+WEBSERVICE("x")"#, &["WEBSERVICE"]),
        (r#"T[a']]+WEBSERVICE("x")"#, &["WEBSERVICE"]),
        (r#"T[a'']+'x'!A1+WEBSERVICE("x")"#, &["WEBSERVICE"]),
        (r#"T[[#This Row],[a'[b]]+CALL("k","f","A")"#, &["CALL"]),
        // `"` in a column name opens no string.
        (r#"T[a"b]+CALL("x")"#, &["CALL"]),
        // A string's `""` and a sheet name's `''` close nothing.
        (r#""a""b"&WEBSERVICE(A1)"#, &["WEBSERVICE"]),
        (r"'it''s'!A1+WEBSERVICE(A1)", &["WEBSERVICE"]),
        // `_xHHHH_` hides a letter.
        (r#"_x0057_EBSERVICE("x")"#, &["WEBSERVICE"]),
        (r#"_x0057__x0045_BSERVICE("x")"#, &["WEBSERVICE"]),
        // Prefixes are taken off the name and kept apart.
        ("_xlfn._xlws.FILTER(A1:A3,B1:B3)", &["_xlfn._xlws.FILTER"]),
        (
            "_xlfn.BYROW(A1:A3,_xleta.SUM)",
            &["_xlfn.BYROW", "_xleta.SUM*"],
        ),
        // `_xlpm.` with a function's prefix makes no parameter.
        (
            r#"_xlfn._xlpm.WEBSERVICE("x")"#,
            &["_xlfn._xlpm.WEBSERVICE"],
        ),
        // A name before `(` is no column and no DDE topic.
        ("A:T(B1)", &["T"]),
        ("cmd|N(1)", &["N"]),
        // A prefixed word before brackets is a function, not a table.
        ("_xleta.WEBSERVICE[x]", &["_xleta.WEBSERVICE*"]),
        // A space before `(` still makes a call.
        ("WEBSERVICE (A1)", &["WEBSERVICE"]),
    ];
    for (formula, expected) in cases {
        assert_eq!(calls(formula), expected, "{formula}");
        assert_eq!(unseen(formula), Vec::<String>::new(), "{formula}");
    }
}

#[test]
fn an_escape_with_an_upper_case_x_is_kept_and_malformed() {
    // Decoded, this would be `_xludf.F[(1)+]`: a table, and no call.
    let formula = "_xludf.F_X005B_(1)+_X005D_";
    let read = analyze(formula);
    assert_eq!(read.text, formula);
    assert_eq!(calls(formula), ["_xludf.F_X005B_"]);
    let reasons: Vec<_> = read.facts.malformed.iter().map(|m| m.reason).collect();
    assert_eq!(reasons, [MalformedReason::UpperCaseEscape; 2]);
}

#[test]
fn a_call_carries_the_sheet_and_book_before_it() {
    let read = analyze("Sheet2!MyFn(1)+[1]!F(2)");
    let at: Vec<_> = read
        .facts
        .calls
        .iter()
        .map(|c| {
            (
                c.name.as_str(),
                c.location.book.as_deref(),
                c.location.sheet.as_deref(),
            )
        })
        .collect();
    assert_eq!(at, [("MYFN", None, Some("Sheet2")), ("F", Some("1"), None)]);
    // After a range's `:` the sheet is the cell's alone.
    let read = analyze("Sheet2!A1:F(1)+Sheet2!A1:Foo+S!A1:1:3+S!A1:[c]+S!A1:B2[c]");
    let after: Vec<_> = read
        .facts
        .references
        .iter()
        .filter(|r| r.kind != ReferenceKind::Cell)
        .map(|r| r.location.clone())
        .collect();
    assert_eq!(after, vec![Location::default(); 3]);
    let call = read.facts.calls.first().map(|c| c.location.clone());
    assert_eq!(call, Some(Location::default()));
    let name = read.facts.names.first().map(|n| n.location.clone());
    assert_eq!(name, Some(Location::default()));
    // A word shaped like a cell that is called is a call, with no sheet.
    let read = analyze("Sheet2!A1:LOG10(1)");
    let call = read.facts.calls.first().map(|c| c.location.clone());
    assert_eq!(call, Some(Location::default()));
}

#[test]
fn names_are_upper_cased_in_ascii_only() {
    // `ſ` upper-cases to `S` in Unicode, which would make an unknown name
    // read as SUM.
    assert_eq!(calls("ſum(1)"), ["ſUM"]);
    assert_eq!(unseen(r#""ſum(""#), ["ſUM"]);
}

#[test]
fn an_escaped_underscore_is_decoded_once() {
    let formula = analyze("_x005F_x0041_(1)");
    assert_eq!(formula.text, "_x0041_(1)");
    assert_eq!(calls("_x005F_x0041_(1)"), ["_X0041_"]);
}

#[test]
fn the_net_reports_a_call_the_lexer_reads_as_text() {
    // Correct reports: the lexer reads these as a string, a sheet name and
    // a column name, and the net, ignoring quotes, sees a call in each.
    assert_eq!(unseen(r#""=SUM(1)""#), ["SUM"]);
    assert_eq!(unseen("'a WEBSERVICE(b'!A1"), ["WEBSERVICE"]);
    assert_eq!(unseen("T[CALL(x)]"), ["CALL"]);
    // A prefix is taken off, as from a call.
    assert_eq!(unseen(r#""_xlfn.FOO (""#), ["FOO"]);
}

#[test]
fn a_quote_left_open_hides_the_rest_from_the_lexer_but_not_from_the_net() {
    let cases = [
        (r#"1&"x&WEBSERVICE(A1)"#, MalformedReason::UnclosedString),
        ("'x!A1+WEBSERVICE(A1)", MalformedReason::UnclosedSheetName),
        ("T[a+WEBSERVICE(A1)", MalformedReason::UnclosedBracket),
    ];
    for (formula, reason) in cases {
        let read = analyze(formula);
        assert_eq!(calls(formula), Vec::<String>::new(), "{formula}");
        assert_eq!(unseen(formula), ["WEBSERVICE"], "{formula}");
        let malformed = read.facts.malformed.first().map(|m| m.reason);
        assert_eq!(malformed, Some(reason), "{formula}");
    }
}

#[test]
fn a_name_is_a_possible_function_and_a_cell_is_not() {
    // `LOG10` is column LOG, row 10; with `(` it is the function.
    let read = analyze("LOG10+LOG10(1)");
    assert_eq!(calls("LOG10+LOG10(1)"), ["LOG10"]);
    let targets: Vec<_> = read
        .facts
        .references
        .iter()
        .map(|r| r.target.as_str())
        .collect();
    assert_eq!(targets, ["LOG10"]);
    // Past the last row or column a word is a name, which may hold a LAMBDA.
    let read = analyze("ABC1234567+XFE1+MyLambda+TRUE+false");
    let names: Vec<_> = read.facts.names.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["ABC1234567", "XFE1", "MYLAMBDA"]);
}

#[test]
fn a_lambda_parameter_is_no_function_called_or_not() {
    let formula = "_xlfn.LAMBDA(_xlpm.x,_xlpm.f,_xlpm.f(_xlpm.x))";
    let read = analyze(formula);
    assert_eq!(calls(formula), ["_xlfn.LAMBDA"]);
    let params: Vec<_> = read.facts.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(params, ["X", "F", "F", "X"]);
    assert!(read.facts.unseen_calls.is_empty());
}

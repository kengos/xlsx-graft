//! The bounds of what `apply` holds while it reads the formulas of the other
//! parts, tested at the largest value each admits and one past it: the bytes
//! of one formula, of a reported attribute and of a reported place, the
//! number of references one report holds, and how a formula is written when
//! it is past the byte limit.

use super::parts::{CHART, CHART_NS, WORKSHEET, beta_with, book, formula_of, kept, reported};
use super::{applied_with_report, defined, zip};
use crate::structure::KeptReason;
use std::fmt::Write;

/// The part name of the worksheet the tests put the formulas in.
const SHEET2: &str = "/xl/worksheets/sheet2.xml";

#[test]
fn a_formula_at_the_byte_limit_is_rewritten_and_one_byte_more_is_reported_and_kept() {
    let sheet = |formula: &str| beta_with(&format!(r#"<c r="A1"><f>{formula}</f></c>"#));
    let exact = sheet(&formula_of(65_536));
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &exact[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    let expected = sheet(&formula_of(65_536).replacen("Alpha", "Delta", 1));
    assert_eq!(output.part(SHEET2), Some(expected));
    assert_eq!(kept(&report), []);
    let long = sheet(&format!("{} ", formula_of(65_536)));
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &long[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert!(output.holds(&long));
    assert_eq!(
        kept(&report),
        [(SHEET2, "f", Some("A1"), None, KeptReason::TooLong)]
    );
}

#[test]
fn the_comments_inside_a_formula_count_toward_its_byte_limit() {
    let sheet = |comment: usize| {
        let content = format!(
            r#"<c r="A1"><f>Alpha!A1<!--{}--></f></c>"#,
            "x".repeat(comment)
        );
        beta_with(&content)
    };
    // The text is 8 bytes and the comment's content is the rest of 65,536.
    let exact = sheet(65_528);
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &exact[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert!(!output.holds(&exact));
    assert_eq!(kept(&report), []);
    let long = sheet(65_529);
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &long[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert!(output.holds(&long));
    assert_eq!(report.kept().len(), 1);
    assert!(matches!(report.kept()[0].reason(), KeptReason::TooLong));
}

#[test]
fn a_reported_attribute_at_the_byte_limit_is_read_and_one_byte_more_is_reported_too_long() {
    let sheet = |val: &str| {
        beta_with(&format!(
            r#"<conditionalFormatting sqref="F1"><cfRule><dataBar><cfvo val="{val}"/></dataBar></cfRule></conditionalFormatting>"#
        ))
    };
    for (len, reason, name) in [
        (65_536, KeptReason::Outside, Some("Alpha")),
        (65_537, KeptReason::TooLong, None),
    ] {
        let content = sheet(&format!(
            "{}{}",
            formula_of(65_536),
            " ".repeat(len - 65_536)
        ));
        let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &content[..])];
        let (_, report) = applied_with_report(book("", &extra), |edit| {
            edit.rename_sheet("Alpha", "Delta").unwrap();
        });
        assert_eq!(
            kept(&report),
            [(SHEET2, "cfvo@val", Some("F1"), name, reason)],
            "{len}"
        );
    }
}

/// A worksheet of `count` cells whose formulas are malformed and name
/// Alpha.
fn malformed_cells(count: usize) -> String {
    let mut out = String::new();
    for n in 0..count {
        write!(out, r#"<c r="A{n}"><f>Alpha!"x</f></c>"#).unwrap();
    }
    beta_with(&out)
}

#[test]
fn the_cell_the_range_and_the_defined_name_of_a_report_are_cut_at_1024_bytes() {
    let long = |n: usize| "a".repeat(n);
    let multibyte = format!("{}é", long(1_023));
    let sheet = beta_with(&format!(
        r#"<c r="{c}"><f>Alpha!"x</f></c><conditionalFormatting sqref="{r}"><f>Alpha!"y</f></conditionalFormatting><conditionalFormatting sqref="{m}"><f>Alpha!"w</f></conditionalFormatting><conditionalFormatting sqref="{x}"><f>Alpha!"v</f></conditionalFormatting><definedName name="{n}">Alpha!"z</definedName>"#,
        c = long(1_025),
        r = long(1_024),
        m = multibyte,
        x = long(1_025),
        n = long(1_100),
    ));
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (_, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    let ats: Vec<&str> = report.kept().iter().map(|k| k.at().unwrap()).collect();
    assert_eq!(ats[0], long(1_024));
    assert_eq!(ats[1], long(1_024));
    assert_eq!(ats[2], long(1_023));
    assert_eq!(ats[3], long(1_024));
    // The 1,024 bytes include the `definedName ` the place starts with.
    assert_eq!(ats[4], format!("definedName {}", long(1_012)));
}

#[test]
fn the_place_of_a_defined_name_is_1024_bytes_with_its_prefix_in_a_sheet_and_in_the_workbook() {
    let prefix = "definedName ";
    let sheet = |name: &str| {
        beta_with(&format!(
            r#"<definedName name="{name}">Alpha!"x</definedName>"#
        ))
    };
    for (len, expected) in [(1_012, 1_012), (1_013, 1_012), (1_011, 1_011)] {
        let name = "n".repeat(len);
        let content = sheet(&name);
        let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &content[..])];
        let in_book = defined(&[(name.as_str(), "", "Alpha!\"x")]);
        for (names, extra) in [("", &extra[..]), (in_book.as_str(), &[][..])] {
            let (_, report) = applied_with_report(book(names, extra), |edit| {
                edit.rename_sheet("Alpha", "Delta").unwrap();
            });
            let at = report.kept()[0].at().unwrap();
            assert_eq!(at, format!("{prefix}{}", "n".repeat(expected)), "{len}");
            assert!(at.len() <= 1_024, "{len}");
        }
    }
    // A character of two bytes that does not fit is dropped, not split.
    let name = format!("{}é", "n".repeat(1_011));
    let content = sheet(&name);
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &content[..])];
    let (_, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert_eq!(
        report.kept()[0].at(),
        Some(format!("{prefix}{}", "n".repeat(1_011)).as_str())
    );
}

#[test]
fn a_report_holds_at_most_16384_references_and_counts_the_others() {
    for (count, held, omitted) in [(16_384, 16_384, 0), (16_385, 16_384, 1)] {
        let sheet = malformed_cells(count);
        let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
        let bytes = zip("", &[("Alpha", ""), ("Beta", "")], "", &extra);
        let report = reported(bytes, |edit| edit.rename_sheet("Alpha", "Delta").unwrap());
        assert_eq!((report.kept().len(), report.omitted()), (held, omitted));
    }
}

#[test]
fn the_limit_of_references_is_for_the_whole_report_not_for_each_part() {
    let (sheet, graph) = (malformed_cells(10_000), chart_cells(10_000));
    let extra = [
        ("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..]),
        ("xl/charts/chart1.xml", CHART, &graph[..]),
    ];
    let bytes = zip("", &[("Alpha", ""), ("Beta", "")], "", &extra);
    let report = reported(bytes, |edit| edit.rename_sheet("Alpha", "Delta").unwrap());
    assert_eq!((report.kept().len(), report.omitted()), (16_384, 3_616));
    // The first part's references come first, then the second part's.
    let parts: Vec<&str> = report.kept().iter().map(|k| k.part()).collect();
    assert_eq!(parts.iter().filter(|part| **part == SHEET2).count(), 10_000);
}

#[test]
fn the_defined_names_of_the_workbook_are_reported_up_to_what_the_parts_left_of_the_limit() {
    let names: Vec<(String, &str, &str)> = (0..16_385)
        .map(|n| (format!("n{n}"), "", "Alpha!\"x"))
        .collect();
    let borrowed: Vec<(&str, &str, &str)> = (names.iter())
        .map(|(name, scope, text)| (name.as_str(), *scope, *text))
        .collect();
    let bytes = zip("", &[("Alpha", ""), ("Beta", "")], &defined(&borrowed), &[]);
    let report = reported(bytes, |edit| edit.rename_sheet("Alpha", "Delta").unwrap());
    assert_eq!((report.kept().len(), report.omitted()), (16_384, 1));
    // With 10 references in a part before them, 10 of the names do not fit.
    let sheet = malformed_cells(10);
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let bytes = zip(
        "",
        &[("Alpha", ""), ("Beta", "")],
        &defined(&borrowed),
        &extra,
    );
    let report = reported(bytes, |edit| edit.rename_sheet("Alpha", "Delta").unwrap());
    assert_eq!((report.kept().len(), report.omitted()), (16_384, 11));
    assert_eq!(report.kept()[0].part(), SHEET2);
}

/// A chart of `count` series whose formulas are malformed and name Alpha.
fn chart_cells(count: usize) -> String {
    format!(
        r#"<c:chartSpace xmlns:c="{CHART_NS}">{}</c:chartSpace>"#,
        r#"<c:f>Alpha!"x</c:f>"#.repeat(count)
    )
}

#[test]
fn empty_comments_and_processing_instructions_inside_a_formula_count_toward_its_byte_limit() {
    let sheet = |inside: &str| beta_with(&format!(r#"<c r="A1"><f>Alpha!A1{inside}</f></c>"#));
    // The text is 8 bytes; each empty comment counts as one.
    for (count, over) in [(65_528, false), (65_529, true)] {
        let content = sheet(&"<!---->".repeat(count));
        let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &content[..])];
        let (output, report) = applied_with_report(book("", &extra), |edit| {
            edit.rename_sheet("Alpha", "Delta").unwrap();
        });
        assert_eq!(output.holds(&content), over, "{count}");
        assert_eq!(report.kept().len(), usize::from(over), "{count}");
    }
    // The text is 8 bytes; the instruction's target and space are 2 and its
    // data (`data` bytes) are the rest of 65,536.
    for (data, over) in [(65_526, false), (65_527, true)] {
        let content = sheet(&format!("<?p {}?>", "y".repeat(data)));
        let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &content[..])];
        let (output, report) = applied_with_report(book("", &extra), |edit| {
            edit.rename_sheet("Alpha", "Delta").unwrap();
        });
        assert_eq!(output.holds(&content), over, "{data}");
        assert_eq!(report.kept().len(), usize::from(over), "{data}");
    }
    let content = sheet("<?pi data?>");
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &content[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert_eq!(
        output.part(SHEET2),
        Some(sheet("").replace("Alpha!", "Delta!"))
    );
    assert_eq!(kept(&report), []);
}

#[test]
fn a_formula_too_long_is_written_as_it_was_beside_a_formula_that_is_rewritten() {
    let long = format!("{} ", formula_of(65_536));
    let sheet = |first: &str| {
        beta_with(&format!(
            r#"<c r="A1"><f>{first}A1</f></c><c r="A2"><f>{long}</f></c><c r="A3"><f>{long}<!--c--></f></c>"#
        ))
    };
    let input = sheet("Alpha!");
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &input[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert_eq!(output.part(SHEET2), Some(sheet("Delta!")));
    let too_long = |at| (SHEET2, "f", Some(at), None, KeptReason::TooLong);
    assert_eq!(kept(&report), [too_long("A2"), too_long("A3")]);
}

#[test]
fn a_formula_past_the_byte_limit_after_an_element_inside_it_is_written_as_it_was_and_too_long() {
    // The element `x` is inside the formula, then the text passes 64 KiB.
    let after_child = format!("Alpha!A1<x/>{}", "y".repeat(65_536));
    // The text passes 64 KiB, then the element `x` is inside the formula.
    let before_child = format!("{}<x/>Alpha!A1", "y".repeat(65_537));
    for formula in [after_child, before_child] {
        let sheet = beta_with(&format!(r#"<c r="A1"><f>{formula}</f></c>"#));
        let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
        let (output, report) = applied_with_report(book("", &extra), |edit| {
            edit.rename_sheet("Alpha", "Delta").unwrap();
        });
        assert_eq!(output.part(SHEET2), Some(sheet.clone()));
        assert_eq!(
            kept(&report),
            [(SHEET2, "f", Some("A1"), None, KeptReason::TooLong)]
        );
    }
}

#[test]
fn a_formula_at_the_byte_limit_with_an_element_inside_it_is_reported_as_markup_not_too_long() {
    // 8 bytes of text, the element, then text up to 65,536 bytes in all.
    let formula = format!("Alpha!A1<x/>{}", "y".repeat(65_528));
    let sheet = beta_with(&format!(r#"<c r="A1"><f>{formula}</f></c>"#));
    let extra = [("xl/worksheets/sheet2.xml", WORKSHEET, &sheet[..])];
    let (output, report) = applied_with_report(book("", &extra), |edit| {
        edit.rename_sheet("Alpha", "Delta").unwrap();
    });
    assert_eq!(output.part(SHEET2), Some(sheet.clone()));
    assert_eq!(
        kept(&report),
        [(SHEET2, "f", Some("A1"), Some("Alpha"), KeptReason::Markup)]
    );
}

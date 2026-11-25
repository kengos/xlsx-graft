//! `Context::judge`: a formula is tried only against the changes about the
//! sheets its prefixes name, and the result is what applying every change
//! in order gives.

use super::{Context, Judged};
use crate::structure::edit::change::Change;
use crate::structure::edit::formula::rewrite_text;
use crate::structure::edit::state::escaped;
use crate::structure::edit::tests::open;
use std::collections::BTreeMap;
use std::sync::Arc;
use xlsx_graft_formula::analyze;

/// What `Context::judge` gives for `raw`, as the text, whether a `#REF!` was
/// written and the sheet of the first change that touched it.
fn judged(context: &Context, raw: &str) -> Option<(String, bool, String)> {
    let Judged { rewritten, .. } = context.judge(raw);
    rewritten.map(|done| (done.text, done.ref_written, done.sheet.to_string()))
}

/// The same, by applying each change of `context`, in order, to the formula.
fn applied_in_order(context: &Context, raw: &str) -> Option<(String, bool, String)> {
    let mut current = analyze(raw);
    let mut result: Option<(String, bool, String)> = None;
    for change in &context.changes {
        let Some((text, ref_written)) = rewrite_text(&current, change, &context.ids).written else {
            continue;
        };
        current = analyze(&escaped(&text));
        result = Some(match result {
            Some((_, before, sheet)) => (text, before || ref_written, sheet),
            None => (text, ref_written, change.sheet().to_string()),
        });
    }
    result
}

/// A context of the changes of a workbook of 120 sheets `S0` to `S119` and
/// `A` to `E`, after calls that chain: rename, delete, rename, delete,
/// with 100 renames of sheets the formulas do not name between them.
fn chained() -> Context {
    let mut names: Vec<String> = (0..120).map(|n| format!("S{n}")).collect();
    names.extend(["A", "B", "C", "D", "E"].map(str::to_owned));
    let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut edit = open(&borrowed, "");
    edit.rename_sheet("B", "Bee").unwrap();
    for n in 0..50 {
        edit.rename_sheet(&format!("S{n}"), &format!("Renamed {n}"))
            .unwrap();
    }
    edit.delete_sheet("bee").unwrap();
    edit.rename_sheet("C", "Cee").unwrap();
    for n in 50..100 {
        edit.rename_sheet(&format!("S{n}"), &format!("Renamed {n}"))
            .unwrap();
    }
    edit.delete_sheet("A").unwrap();
    edit.rename_sheet("E", "Eee").unwrap();
    edit.rename_sheet("Cee", "C").unwrap();
    let changes = std::mem::take(&mut edit.state.changes);
    let ids = std::mem::take(&mut edit.state.sheet_ids);
    Context::new(changes, ids, BTreeMap::new())
}

#[test]
fn a_formula_gets_the_changes_that_name_its_sheets_in_order_and_each_once() {
    let context = chained();
    assert_eq!(context.changes.len(), 106);
    let formulas = [
        // Renamed, then deleted: `B` is `Bee`, then #REF!.
        "B!X1+D!Y1",
        "A:E!X1",
        "E:A!X1",
        "B:C!X1",
        "'B':C!X1",
        "C!X1+Bee!X1+Eee!X1+E!X1",
        "SUM(A!X1,B!X1,C!X1,D!X1,E!X1)",
        "A:B!X1+B:C!X1+C:D!X1+D:E!X1",
        // A sheet that no change names, and a book.
        "D!X1+S119!X1",
        "[1]A!X1",
        "S3!X1+S77!X1",
        "no sheet at all",
        "\"A!X1",
    ];
    for formula in formulas {
        assert_eq!(
            judged(&context, formula),
            applied_in_order(&context, formula),
            "{formula}"
        );
    }
    let (text, ref_written, sheet) = judged(&context, "B!X1+D!Y1").unwrap();
    assert_eq!(
        (text.as_str(), ref_written, sheet.as_str()),
        ("#REF!+D!Y1", true, "B")
    );
}

#[test]
fn a_formula_that_no_change_names_is_not_touched_and_a_malformed_one_is_not_rewritten() {
    let context = chained();
    assert!(judged(&context, "D!X1+S119!X1").is_none());
    assert!(judged(&context, "A!\"x").is_none());
    let rename = Change::Rename {
        old: Arc::from("Aaa"),
        new: "Fresh".to_owned(),
    };
    let context = Context::new(vec![rename], Default::default(), BTreeMap::new());
    assert_eq!(
        judged(&context, "aaa!X1").map(|done| done.0),
        Some("Fresh!X1".to_owned())
    );
}

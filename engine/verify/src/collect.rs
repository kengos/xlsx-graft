//! [`FormulaSink`]: where the formulas the parts hold go as the parts are
//! read, with the defined names and tables they declare
//! (`collector.rs` finds them).
//!
//! Nothing is kept here: each formula is handed over as it is read, so a
//! large worksheet costs no more memory than a small one. Nothing
//! is judged here either: [`verify`](crate::verify) hands them to
//! [`Discard`] until the judge ([`FormulaJudge`](crate::FormulaJudge))
//! takes them.

mod collector;

pub(crate) use collector::Collector;

/// What takes the formulas of the parts, and the names they may use, as
/// they are read. Every method is told the part it was found in, by its
/// name; `at` is where in the part, as `collector.rs` says.
///
/// Each is found by its element's local name in whatever part holds it,
/// as the part kinds are: a defined name or table a part of another kind
/// holds is handed over the same way, and which parts to believe is the
/// sink's to decide. Which sheet a `localSheetId` counts to is not found
/// here: the sheets a workbook lists are its workbook part's, to be read
/// by that part's schema, in their namespace and order.
pub(crate) trait FormulaSink {
    /// A formula, its text whole, `_xHHHH_` escapes and all.
    fn formula(&mut self, part: &str, at: &str, text: &str);

    /// A formula longer than the limit, whose text was not kept: `size`
    /// bytes, as UTF-8.
    fn too_long(&mut self, part: &str, at: &str, size: u64);

    /// A formula with an element inside it, whose text was not put
    /// together: what Excel reads of it is not known.
    fn markup(&mut self, part: &str, at: &str);

    /// A defined name, as written, the `localSheetId` it is scoped to as
    /// written (`None` for the whole workbook), and its formula, whole. A
    /// defined name whose formula is too long is
    /// [`too_long`](Self::too_long) instead, one with markup inside it
    /// [`markup`](Self::markup), and one whose formula is empty or white
    /// space is not handed over: a check that lists the defined names from
    /// this alone does not know those.
    fn defined_name(&mut self, part: &str, at: &str, name: &str, sheet: Option<&str>, text: &str);

    /// A table's name or display name, as written.
    fn table(&mut self, part: &str, name: &str);
}

/// A sink that drops whatever it is handed.
#[derive(Debug, Default)]
pub(crate) struct Discard;

impl FormulaSink for Discard {
    fn formula(&mut self, _part: &str, _at: &str, _text: &str) {}

    fn too_long(&mut self, _part: &str, _at: &str, _size: u64) {}

    fn markup(&mut self, _part: &str, _at: &str) {}

    fn defined_name(&mut self, _: &str, _: &str, _: &str, _: Option<&str>, _: &str) {}

    fn table(&mut self, _part: &str, _name: &str) {}
}

//! The formula check's judge: what a formula calls, names and links to,
//! with its names resolved against the workbook's, judged by the
//! [`Rules`] and the [`FunctionTable`].
//!
//! [`FormulaJudge`] reads no XML: its caller finds each formula, says
//! where it stands ([`Place`]) and hands over the names the workbook
//! defines ([`Names`]). The judge reads each formula with
//! [`xlsx_graft_formula::analyze`] and refuses:
//!
//! - a formula with any malformed place, whole: past it, what the formula
//!   states is a guess (`formula_malformed`);
//! - a DDE link as [`PartKind::Dde`](crate::PartKind::Dde) content and
//!   a reference to another workbook as
//!   [`PartKind::ExternalLink`](crate::PartKind::ExternalLink) content,
//!   unless the template allows the kind (`formula_denied`, or
//!   `formula_forbidden` when the policy forbids it);
//! - a function the template does not allow and the table does not mark
//!   reviewed safe (`function_dangerous`, `function_denied`), and one the
//!   policy forbids whatever the template says (`function_forbidden`).
//!
//! It also checks each function a formula calls against the versions the
//! template's [`Targets`] hold, by the compatibility table: one some of
//! them lack, or are not confirmed to have, is an error for `targets:`
//! (`target_function_unsupported`) and a warning for `bestEffort:`
//! (`best_effort_function_unsupported`).
//!
//! How it tells a function from a defined name or a table, and what its
//! caller does, is for [`FormulaJudge`] to say.

mod body;
mod found;
mod graph;
mod names;

pub use names::Names;

use crate::{FunctionTable, Rules};
use found::Found;
use graph::Graph;
use names::Scope;
use xlsx_graft_compat::{Feature, Targets};
use xlsx_graft_diagnostics::Diagnostics;

/// Where a formula stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place<'a> {
    /// The part it is in.
    pub part: &'a str,
    /// Where in the part, as a reader would look for it: a cell (`B2`), or
    /// what holds the formula (`definedName MyFn`).
    pub at: &'a str,
    /// The sheet whose names the formula sees first, by its index among the
    /// workbook's sheets as a defined name's `localSheetId` counts it; for
    /// a defined name's own formula, its `localSheetId`. `None` outside any
    /// sheet.
    pub sheet: Option<u32>,
}

/// Judges formulas one at a time, then reports what it found.
///
/// # What a name is
///
/// A function called, one passed as a value, a `NAME(` only the formula
/// reader's safety net saw, and a name with no `(` are each a word that
/// may be a function. A word with a prefix (`_xlfn.`, `_xleta.` and the
/// like) or after another workbook (`[1]!MyFn`) is looked up in the table
/// under its name, a user-defined one (`_xludf.`) under `_XLUDF.` and its
/// name, so only the template can allow it (and the policy forbids it
/// under its name too). A word only the safety net saw counts as after
/// another workbook when `!` comes before it, a sheet's or a workbook's.
/// Any other is first looked up in the defined names:
///
/// - after a sheet (`Sheet2!MyFn`), or in the formula of a defined name
///   reached from another formula, every defined name of that name, in
///   any scope, since which one Excel would take is not worth guessing;
/// - otherwise, a defined name's own formula judged at its own place
///   included, those scoped to the formula's sheet ([`Place::sheet`]);
///   the workbook's when the sheet has none; and, when neither has one,
///   those of every scope rather than none: which one the word may mean
///   is not guessed, though Excel gives a cell `#NAME?` there. Such a
///   word is judged as a function too, and denied when the table does not
///   know it: a name out of the word's scope does not make it safe.
///
/// A word that names a defined name, or has no `(` and names a table, is
/// judged as a function as well, when the rules or the table know a
/// function of its name: a defined name or a table called `RTD` does not
/// make `RTD(…)` safe. Any other word is a function, and one the table
/// does not know is denied. A word only the safety net saw is judged
/// like any other, so a string holding `Total (net)` is denied as the
/// function `TOTAL`: the net errs towards reporting, as the formula reader
/// says it does.
///
/// A defined name's formula is judged by the same rules, read once
/// through other formulas, at the first formula that reaches it: each problem it holds is reported at
/// that formula's place, with the defined name, as many times as its
/// formula shows it. A name naming another is followed, breadth first and
/// no name twice, however deep, without recursing: the work is linear in
/// the names and their formulas, so no depth limit is needed to bound it,
/// and every name reached is judged by its formula. A defined name that
/// names itself and is judged at its own place shows each problem twice:
/// at that place, and through its name.
///
/// # Targets
///
/// Each function a formula calls, or names with no `(`, that the
/// compatibility table has a row for is checked against every version the
/// [`Targets`] hold, the features it needs included; a function the table
/// has no row for (one Excel 2007 already had) is not. Through a defined
/// name, the functions of its formula are checked: a `LAMBDA` named `MyFn`
/// is checked by the functions inside it. A call is checked as the
/// function of its name whatever names share it, but a word with no `(`
/// that names a defined name in scope or a table calls nothing, and is
/// not: unlike the judging above, guessing here would refuse a workbook
/// that works. A word only the safety net saw is not checked either, nor
/// a user-defined function or one after another workbook. Each part gets
/// one diagnostic per defined name reached through, function, application
/// and list, as for the other findings. [`verify`](crate::verify) leaves
/// out of what a function needs the features it found the workbook to use,
/// which it checks on their own.
///
/// # One diagnostic per finding
///
/// Each part gets one diagnostic per defined name reached through (or
/// none) and finding, however many times it shows it, located at the
/// first place and counting every time the part's formulas show it; the
/// defined names' own formulas, judged at their own places, show theirs
/// as the workbook part's, with none reached through: a hundred thousand cells
/// calling `WEBSERVICE` are one diagnostic, at the first of them. One
/// found through a defined name counts the times that name's formula shows
/// it. Work is linear in the formulas and the defined names' formulas:
/// each bucket of defined names is looked through once, and each defined
/// name read once through other formulas.
///
/// # What the caller does
///
/// The caller judges every formula of the workbook, each defined name's own
/// formula among them, at its own place (`definedName MyFn`, its
/// `localSheetId` as the sheet). A defined name is then judged at its own
/// place, and, when a formula reaches it, through the first one that does
/// as well. Without that, the judge is not closed: a sheet's own defined
/// name hides the workbook's of the same name from that sheet's formulas,
/// so a workbook name no formula reaches would be judged by nothing.
///
/// # Examples
///
/// ```
/// use xlsx_graft_diagnostics::Diagnostics;
/// use xlsx_graft_verify::{
///     FormulaJudge, FunctionClass, FunctionTable, Names, Place, Rules, Targets,
/// };
///
/// struct Table;
/// impl FunctionTable for Table {
///     fn classify(&self, name: &str) -> Option<FunctionClass<'_>> {
///         (name == "SUM").then_some(FunctionClass::ReviewedSafe)
///     }
/// }
///
/// let rules = Rules::default();
/// let names = Names::default().defined_name("Fetch", None, "WEBSERVICE(\"https://x\")");
/// let targets = Targets::default();
/// let mut judge = FormulaJudge::new(&rules, &Table, &targets, &names);
/// let place = Place { part: "/xl/worksheets/sheet1.xml", at: "A1", sheet: Some(0) };
/// judge.judge(&place, "SUM(1,2)+Fetch");
/// let mut diagnostics = Diagnostics::new();
/// judge.finish(&mut diagnostics);
/// // `WEBSERVICE` is dangerous, and Excel for Mac, which the default
/// // targets hold, lacks it.
/// assert_eq!(diagnostics.records().len(), 2);
/// ```
pub struct FormulaJudge<'a> {
    context: Context<'a>,
    graph: Graph,
    found: Found,
}

/// What the judge judges by.
#[derive(Clone, Copy)]
struct Context<'a> {
    rules: &'a Rules,
    functions: &'a dyn FunctionTable,
    targets: &'a Targets,
    names: &'a Names,
}

impl<'a> FormulaJudge<'a> {
    /// A judge of formulas by `rules` and `functions`, checking the
    /// functions against `targets`, whose names are looked up in `names`.
    #[must_use]
    pub fn new(
        rules: &'a Rules,
        functions: &'a dyn FunctionTable,
        targets: &'a Targets,
        names: &'a Names,
    ) -> Self {
        Self {
            context: Context {
                rules,
                functions,
                targets,
                names,
            },
            graph: Graph::new(names),
            found: Found::default(),
        }
    }

    /// Judges `formula`, the text of a formula with no leading `=`, at
    /// `place`.
    pub fn judge(&mut self, place: &Place<'_>, formula: &str) {
        let body = body::read(formula, Scope::From(place.sheet), self.context);
        for finding in &body.findings {
            self.found.add(place, None, finding);
        }
        for (holder, finding) in self.graph.reach(&body.names, self.context) {
            let name = self.context.names.name(holder);
            self.found.add(place, Some((holder, name)), &finding);
        }
    }

    /// Pushes a diagnostic for each finding into `diagnostics`, in the
    /// order they were first found.
    pub fn finish(self, diagnostics: &mut Diagnostics) {
        self.finish_besides(&[], diagnostics);
    }

    /// What [`FormulaJudge::finish`] does, the `features` the workbook was
    /// found to use left out of what each function needs: each is checked
    /// on its own ([`Targets::check_besides`]).
    pub(crate) fn finish_besides(self, features: &[Feature], diagnostics: &mut Diagnostics) {
        self.found
            .finish(self.context.targets, features, diagnostics);
    }
}

#[cfg(test)]
mod tests;

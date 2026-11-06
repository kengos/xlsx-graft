//! How a word taken for a function is judged: by the rules, then by the
//! table, with the notices a function that is accepted gets.

use super::{Body, Finding, Scheduled, Verdict};
use crate::formulas::Context;
use crate::{FunctionClass, Ruling};
use xlsx_graft_diagnostics::FunctionDenial;

/// The functions whose result is a reference the formula computes as the
/// workbook is calculated, so verify does not see which cells that
/// reference points to. INDIRECT returns the reference a text value names
/// (ECMA-376-1:2016 §18.17.7.167), and OFFSET a reference moved from its
/// argument by computed numbers of rows and columns (ECMA-376-1:2016
/// §18.17.7.245). The text INDIRECT reads may name a cell in another
/// workbook. Verify refuses a reference to another workbook only when the
/// formula writes that reference out, and the template's `parts.allow`
/// does not admit it.
const UNCHECKED_REFERENCES: [&str; 2] = ["INDIRECT", "OFFSET"];

impl Body {
    /// The function `name`, judged by the rules, then by the table; one
    /// the table does not know is denied when `unknown` says so. A
    /// function the table schedules a later version of xlsx-graft to deny
    /// is accepted with a notice, or denied as dangerous when the rules
    /// deny the scheduled functions now.
    pub(super) fn function(&mut self, context: Context<'_>, name: &str, unknown: bool) {
        let verdict = match context.rules.function(name) {
            Ruling::Forbidden { reason } => Verdict::Forbidden(reason.to_owned()),
            Ruling::Allowed => return self.accepted(name),
            Ruling::Unruled => match context.functions.classify(name) {
                Some(FunctionClass::ReviewedSafe) => return self.accepted(name),
                Some(FunctionClass::Scheduled { from, reason }) => {
                    if context.rules.denies_scheduled() {
                        Verdict::Dangerous(format!(
                            "{reason}; xlsx-graft {from} is to deny it, and the host's policy denies it now"
                        ))
                    } else {
                        self.findings.push(Finding::Scheduled(Scheduled {
                            name: name.to_owned(),
                            from: from.to_owned(),
                            reason: reason.to_owned(),
                        }));
                        return self.accepted(name);
                    }
                }
                Some(FunctionClass::Dangerous { reason }) => Verdict::Dangerous(reason.to_owned()),
                Some(FunctionClass::Unreviewed) => Verdict::Denied(FunctionDenial::Unreviewed),
                None if unknown => Verdict::Denied(FunctionDenial::Unknown),
                None => return,
            },
        };
        self.findings
            .push(Finding::Function(name.to_owned(), verdict));
    }

    /// The function `name`, accepted: a notice when it returns a reference
    /// whose cells verify does not see.
    fn accepted(&mut self, name: &str) {
        if UNCHECKED_REFERENCES.contains(&name) {
            self.findings.push(Finding::Unchecked(name.to_owned()));
        }
    }
}

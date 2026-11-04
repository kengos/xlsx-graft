//! What one formula holds: the problems it shows itself ([`Finding`]),
//! and the defined names it names, whose formulas `graph` follows.

use super::Context;
use super::names::{Bucket, Scope};
use crate::{FunctionClass, Ruling};
use std::hash::{Hash, Hasher};
use xlsx_graft_compat::Row;
use xlsx_graft_diagnostics::{FormulaMalformedReason, FunctionDenial, PartKind};
use xlsx_graft_formula::Location;
use xlsx_graft_formula::{MalformedReason, analyze};

/// What one formula holds.
#[derive(Debug, Default)]
pub(super) struct Body {
    /// The problems it shows, once for each time it shows them.
    pub(super) findings: Vec<Finding>,
    /// The defined names it names, once for each time.
    pub(super) names: Vec<Bucket>,
}

/// A problem a formula shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum Finding {
    /// It could not be read, first for this reason.
    Malformed(FormulaMalformedReason),
    /// It holds content of a kind the template does not allow.
    Denied(PartKind),
    /// It holds content of a kind the policy forbids, for this reason.
    Forbidden(PartKind, String),
    /// It may call this function, which is denied.
    Function(String, Verdict),
    /// It calls this function, which the compatibility table has a row
    /// for, to check against the targets.
    Target(Target),
}

/// A function's row in the compatibility table, compared by name so a
/// finding counts once per function.
#[derive(Debug, Clone, Copy)]
pub(super) struct Target(pub(super) &'static Row);

impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        self.0.name == other.0.name
    }
}

impl Eq for Target {}

impl Hash for Target {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.name.hash(state);
    }
}

/// Why a function is denied.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum Verdict {
    /// The policy forbids it, for this reason.
    Forbidden(String),
    /// The table finds it dangerous, for this reason.
    Dangerous(String),
    /// The table has not reviewed it, or does not know it.
    Denied(FunctionDenial),
}

/// A word in a formula that may be a function.
struct Word<'a> {
    /// The name, upper-cased, its prefixes taken off.
    name: &'a str,
    /// The prefixes, as written.
    prefix: &'a str,
    /// Whether another workbook is named before it.
    book: bool,
    /// Which defined names it may mean, when it is one.
    scope: Scope,
    /// Whether it may be a table's name: a name with no `(`.
    table: bool,
    /// Whether the formula reader saw it, rather than only its safety net.
    seen: bool,
}

/// What `formula` holds, its names looked up in `scope` unless a sheet
/// is named before them.
pub(super) fn read(formula: &str, scope: Scope, context: Context<'_>) -> Body {
    let formula = analyze(formula);
    let facts = &formula.facts;
    let mut body = Body::default();
    if let Some(first) = facts.malformed.first() {
        body.findings
            .push(Finding::Malformed(malformed(first.reason)));
        return body;
    }
    for _ in &facts.dde_links {
        body.kind(context, PartKind::Dde);
    }
    for _ in &facts.external_books {
        body.kind(context, PartKind::ExternalLink);
    }
    for call in &facts.calls {
        body.word(
            context,
            &Word {
                name: &call.name,
                prefix: &call.prefix,
                book: call.location.book.is_some(),
                scope: within(&call.location, scope),
                table: false,
                seen: true,
            },
        );
    }
    for unseen in &facts.unseen_calls {
        let written = formula.slice(unseen.span);
        let prefix = written.len().saturating_sub(unseen.name.len());
        body.word(
            context,
            &Word {
                name: &unseen.name,
                prefix: written.get(..prefix).unwrap_or(written),
                book: formula
                    .text
                    .get(..unseen.span.start)
                    .is_some_and(|before| before.ends_with('!')),
                scope,
                table: false,
                seen: false,
            },
        );
    }
    for name in &facts.names {
        body.word(
            context,
            &Word {
                name: &name.name,
                prefix: "",
                book: name.location.book.is_some(),
                scope: within(&name.location, scope),
                table: true,
                seen: true,
            },
        );
    }
    body
}

impl Body {
    /// Content of `kind`, judged by the rules.
    fn kind(&mut self, context: Context<'_>, kind: PartKind) {
        match context.rules.part(kind) {
            Ruling::Forbidden { reason } => {
                self.findings
                    .push(Finding::Forbidden(kind, reason.to_owned()));
            }
            Ruling::Allowed => {}
            Ruling::Unruled => self.findings.push(Finding::Denied(kind)),
        }
    }

    /// `word`: a defined name, a table or a function. A defined name or a
    /// table of a function's name is both: which one Excel would take is
    /// not worth guessing.
    fn word(&mut self, context: Context<'_>, word: &Word<'_>) {
        if word.prefix.is_empty() && !word.book {
            if let Some((bucket, in_scope)) = context.names.resolve(word.name, word.scope) {
                self.names.push(bucket);
                let named = in_scope || context.names.is_table(word.name);
                self.target(context, word, word.table && named);
                self.function(context, word.name, !in_scope);
            } else {
                let table = word.table && context.names.is_table(word.name);
                self.target(context, word, table);
                self.function(context, word.name, !table);
            }
        } else if is_user_defined(word.prefix) {
            if let Ruling::Forbidden { reason } = context.rules.function(word.name) {
                let verdict = Verdict::Forbidden(reason.to_owned());
                self.findings
                    .push(Finding::Function(word.name.to_owned(), verdict));
            }
            self.function(context, &format!("_XLUDF.{}", word.name), true);
        } else {
            self.target(context, word, false);
            self.function(context, word.name, true);
        }
    }

    /// `word` as a function, to check against the targets when the table
    /// of compatibility has a row for it: unless only the safety net saw
    /// it, another workbook is named before it, or it is a `named` defined
    /// name in scope or table with no `(`, which then calls no function.
    /// A call stays a call whatever names share its name.
    fn target(&mut self, context: Context<'_>, word: &Word<'_>, named: bool) {
        if !word.seen || word.book || named || !context.targeted {
            return;
        }
        if let Some(row) = xlsx_graft_compat::function(word.name) {
            self.findings.push(Finding::Target(Target(row)));
        }
    }

    /// The function `name`, judged by the rules, then by the table; one
    /// the table does not know is denied when `unknown` says so.
    fn function(&mut self, context: Context<'_>, name: &str, unknown: bool) {
        let verdict = match context.rules.function(name) {
            Ruling::Forbidden { reason } => Verdict::Forbidden(reason.to_owned()),
            Ruling::Allowed => return,
            Ruling::Unruled => match context.functions.classify(name) {
                Some(FunctionClass::ReviewedSafe) => return,
                Some(FunctionClass::Dangerous { reason }) => Verdict::Dangerous(reason.to_owned()),
                Some(FunctionClass::Unreviewed) => Verdict::Denied(FunctionDenial::Unreviewed),
                None if unknown => Verdict::Denied(FunctionDenial::Unknown),
                None => return,
            },
        };
        self.findings
            .push(Finding::Function(name.to_owned(), verdict));
    }
}

/// Which defined names a word at `location` may mean, `scope` when no
/// sheet is named before it.
fn within(location: &Location, scope: Scope) -> Scope {
    if location.sheet.is_some() {
        Scope::Any
    } else {
        scope
    }
}

/// Whether `prefix` makes a user-defined function: it holds `_xludf.` in
/// any letter case.
fn is_user_defined(prefix: &str) -> bool {
    prefix
        .as_bytes()
        .windows(7)
        .any(|window| window.eq_ignore_ascii_case(b"_xludf."))
}

/// The diagnostic's reason for the formula reader's `reason`.
fn malformed(reason: MalformedReason) -> FormulaMalformedReason {
    match reason {
        MalformedReason::UndecodableEscape => FormulaMalformedReason::UndecodableEscape,
        MalformedReason::UpperCaseEscape => FormulaMalformedReason::UpperCaseEscape,
        MalformedReason::UnclosedString => FormulaMalformedReason::UnclosedString,
        MalformedReason::UnclosedSheetName => FormulaMalformedReason::UnclosedSheetName,
        MalformedReason::UnclosedBracket => FormulaMalformedReason::UnclosedBracket,
        MalformedReason::BracketsTooDeep => FormulaMalformedReason::BracketsTooDeep,
        MalformedReason::UnknownCharacter => FormulaMalformedReason::UnknownCharacter,
        MalformedReason::StraySheetName => FormulaMalformedReason::StraySheetName,
    }
}

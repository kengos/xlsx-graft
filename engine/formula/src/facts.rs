//! The facts a formula states, read from its tokens by `walk`, with the
//! words told apart by `word`.

mod walk;
pub(crate) mod word;

use crate::{Span, Token};

/// What a formula states. Function and defined names are upper-cased in
/// ASCII (Excel compares them without regard to letter case); sheet and
/// book names are as written, unquoted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// The functions it calls or names as a value.
    pub calls: Vec<Call>,
    /// The LAMBDA parameters it names.
    pub params: Vec<Param>,
    /// Its references to cells, rows, columns and tables.
    pub references: Vec<Reference>,
    /// The names that are none of the above: defined names, or functions
    /// named without a call, such as a defined name holding a LAMBDA.
    pub names: Vec<Name>,
    /// Its DDE links.
    pub dde_links: Vec<DdeLink>,
    /// The other workbooks it refers to.
    pub external_books: Vec<ExternalBook>,
    /// Where it could not be read.
    pub malformed: Vec<Malformed>,
    /// Each `NAME(` the quote-blind scan found where the lexer saw no
    /// call: a possible function the lexer missed.
    pub unseen_calls: Vec<UnseenCall>,
}

/// A function a formula calls, or names as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// The name, upper-cased in ASCII, with its prefixes taken off:
    /// `FILTER` for `_xlfn._xlws.FILTER`.
    pub name: String,
    /// The prefixes as written (`_xlfn._xlws.`), or `""`.
    pub prefix: String,
    /// The book and sheet before it: `Sheet2` for `Sheet2!MyFn(1)`, a
    /// defined name scoped to that sheet.
    pub location: Location,
    /// Whether it is named without a call: a function passed as a value
    /// (`_xleta.SUM` in `BYROW(A1:A3,_xleta.SUM)`), or a prefixed name with
    /// no `(` after it.
    pub passed: bool,
    /// Where the name is.
    pub span: Span,
}

/// A LAMBDA parameter (`_xlpm.x`), called or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The name after `_xlpm.`, upper-cased in ASCII.
    pub name: String,
    /// Where it is, prefix included.
    pub span: Span,
}

/// A reference to cells, rows, columns or a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// What it refers to.
    pub kind: ReferenceKind,
    /// The book and sheet before it.
    pub location: Location,
    /// The reference as written: `$A$1`, `A:C`, or a table's name (`""`
    /// for a structured reference with none, inside its own table).
    pub target: String,
    /// Where it is: for a table, its name and its brackets.
    pub span: Span,
}

/// What a [`Reference`] refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// One cell: `A1`, `$XFD$1048576`.
    Cell,
    /// Whole columns: `A:C`.
    Columns,
    /// Whole rows: `1:3`.
    Rows,
    /// A structured reference: `T[a]`, `[@a]`.
    Table,
}

/// The book and sheet a reference or name is prefixed with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Location {
    /// The external book: its index (`1` for `[1]`), or the path a quoted
    /// sheet name holds (`C:\d\b.xlsx`).
    pub book: Option<String>,
    /// The sheet, or two joined by `:` for a 3-D reference.
    pub sheet: Option<String>,
}

/// A name that is not a call, a parameter, a reference or `TRUE`/`FALSE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The name, upper-cased in ASCII.
    pub name: String,
    /// The book and sheet before it.
    pub location: Location,
    /// Where it is.
    pub span: Span,
}

/// A DDE link: `app|topic!item`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdeLink {
    /// The application before `|`, unquoted; `""` when there is none.
    pub app: String,
    /// The topic after `|`, unquoted; `""` when there is none.
    pub topic: String,
    /// The item after `!`, unquoted.
    pub item: Option<String>,
    /// Where the link is.
    pub span: Span,
}

/// Another workbook a formula refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalBook {
    /// Its index (`1` for `[1]`) or the path a quoted sheet name holds.
    pub book: String,
    /// Where it is named.
    pub span: Span,
}

/// A place a formula could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed {
    /// Why.
    pub reason: MalformedReason,
    /// Where.
    pub span: Span,
}

/// Why a formula could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalformedReason {
    /// An `_xHHHH_` naming a surrogate, kept as written.
    UndecodableEscape,
    /// An escape written `_XHHHH_`, kept as written: ECMA-376 does not say
    /// whether it is one.
    UpperCaseEscape,
    /// A string with no closing `"`: the rest of the formula.
    UnclosedString,
    /// A quoted sheet name with no closing `'`: the rest of the formula.
    UnclosedSheetName,
    /// Brackets with no closing `]`: the rest of the formula.
    UnclosedBracket,
    /// Brackets nested deeper than [`crate::MAX_BRACKET_DEPTH`]: the rest
    /// of the formula.
    BracketsTooDeep,
    /// A character no token starts with.
    UnknownCharacter,
    /// A quoted sheet name with no `!` or `|` after it.
    StraySheetName,
}

/// A `NAME(` the lexer did not see as a call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnseenCall {
    /// The name, upper-cased in ASCII, with its prefixes taken off.
    pub name: String,
    /// Where the name is.
    pub span: Span,
}

/// The facts `tokens` of `text` state, with the `malformed` places found
/// so far; [`Facts::unseen_calls`] is left for the safety net.
pub(crate) fn read(text: &str, tokens: &[Token], malformed: Vec<Malformed>) -> Facts {
    let mut facts = Facts {
        malformed,
        ..Facts::default()
    };
    walk::walk(text, tokens, &mut facts);
    facts
}

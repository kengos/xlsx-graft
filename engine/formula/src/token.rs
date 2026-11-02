//! A formula's tokens, and the characters a word is made of, which the
//! lexer and the safety net share so that they cut words alike.

/// A byte range of a formula's decoded text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// The first byte.
    pub start: usize,
    /// One past the last byte.
    pub end: usize,
}

/// One token of a formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// What it is.
    pub kind: TokenKind,
    /// Where it is.
    pub span: Span,
}

/// What a token is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// Spaces, tabs and line breaks. A space between two references is the
    /// intersection operator.
    Whitespace,
    /// A run of word characters: a function or defined name, a cell or
    /// column reference, a sheet name, `TRUE`.
    Word,
    /// A number such as `1`, `1.5` or `1E-3`.
    Number,
    /// A string constant, quotes included.
    String,
    /// A quoted sheet name, quotes included: `'a b'`, `'C:\d\[b.xlsx]S'`.
    SheetName,
    /// Brackets and what is in them: a structured reference's `[…]` or an
    /// external book's `[1]`.
    Bracket,
    /// An error constant such as `#REF!`.
    ErrorValue,
    /// An operator other than the ones below: `+ - * / ^ & = <> < <= > >=
    /// % @`, and `#` after a spilling reference.
    Operator,
    /// `:`, the range operator, or between two sheet names.
    Colon,
    /// `!`, after a sheet or book.
    Bang,
    /// `|`, between a DDE link's application and topic.
    Pipe,
    /// `,`.
    Comma,
    /// `;`, between the rows of an array constant.
    Semicolon,
    /// `(`.
    OpenParen,
    /// `)`.
    CloseParen,
    /// `{`.
    OpenBrace,
    /// `}`.
    CloseBrace,
    /// Text the lexer could not read; [`crate::Facts::malformed`] says why.
    Malformed,
}

/// Whether a word starts with `c`: a letter, `_`, `\`, `$`, or any
/// character past ASCII (a name may use any of them, [MS-XLSX] §2.2.2).
pub(crate) fn is_word_start(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '_' | '\\' | '$') || !c.is_ascii()
}

/// Whether a word goes on with `c`: what starts one, a digit, `.` or `?`.
pub(crate) fn is_word_char(c: char) -> bool {
    is_word_start(c) || c.is_ascii_digit() || matches!(c, '.' | '?')
}

/// Whether `c` is whitespace in a formula.
pub(crate) fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

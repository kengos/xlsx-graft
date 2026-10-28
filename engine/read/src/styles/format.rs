//! [`classify`]: the [`FormatKind`] of a number format code
//! (ECMA-376-1:2016 §18.8.31), read token by token, as the crate root says.

use super::FormatKind;

/// A date or time token of a section, consecutive letters of one kind
/// taken as one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    /// A year: `y`, an era (`g`), an era year (`e`), a Buddhist year (`b`).
    Year,
    /// `m`: a month, or minutes after hours or before seconds.
    M,
    /// A day: `d`, or the day of the week, `aaa`.
    Day,
    /// Hours: `h`.
    Hour,
    /// Seconds: `s`.
    Second,
    /// Elapsed hours, minutes or seconds: `[h]`, `[mm]`, `[ss]`, its letter
    /// lowercased.
    Elapsed(char),
}

/// What one section holds outside its literals, read as it goes: no more
/// than a few flags, however long the section.
#[derive(Debug, Default, Clone, Copy)]
struct Section {
    /// The date and time token read last.
    last: Option<Token>,
    /// An `m` read last that is not minutes by what came before it, so is
    /// minutes only if seconds come next.
    open_m: bool,
    /// A year, a month or a day.
    date: bool,
    /// Hours, minutes or seconds.
    time: bool,
    /// Elapsed time.
    elapsed: bool,
    /// A digit placeholder (`0`, `#`, `?`) or an exponent.
    digits: bool,
    /// `General`.
    general: bool,
    /// `@`: the section is for text.
    text: bool,
    /// Anything at all, literals included.
    written: bool,
}

/// The kind of the format `code`: the kind its number sections that show
/// the number agree on (General and Number agree as Number),
/// [`FormatKind::Unknown`] when they do not or the code does not read,
/// [`FormatKind::Text`] when it has text sections alone, and
/// [`FormatKind::General`] when it shows no number.
#[must_use]
pub(super) fn classify(code: &str) -> FormatKind {
    let Some(sections) = sections(code) else {
        return FormatKind::Unknown;
    };
    // The fourth section is for text, as is any with `@`.
    let text = |at: usize, section: &Section| at == 3 || section.text;
    let mut kinds = (sections.iter().enumerate())
        .filter(|&(at, section)| section.counts() && !text(at, section))
        .map(|(_, section)| section.kind());
    let Some(first) = kinds.next() else {
        let any_text = (sections.iter().enumerate()).any(|(at, s)| s.written && text(at, s));
        return if any_text {
            FormatKind::Text
        } else {
            FormatKind::General
        };
    };
    kinds.fold(first, |agreed, kind| match (agreed, kind) {
        _ if agreed == kind => agreed,
        (FormatKind::General | FormatKind::Number, FormatKind::General | FormatKind::Number) => {
            FormatKind::Number
        }
        _ => FormatKind::Unknown,
    })
}

/// The sections of `code`, split at each `;` outside a literal or
/// brackets; `None` when a literal, an escape or brackets are left open,
/// or there are more than four sections, which Excel does not take.
fn sections(code: &str) -> Option<Vec<Section>> {
    let mut sections = Vec::new();
    let mut section = Section::default();
    let mut rest = code;
    while let Some(c) = rest.chars().next() {
        rest = rest.get(c.len_utf8()..).unwrap_or_default();
        if c == ';' {
            if sections.len() == 3 {
                return None;
            }
            sections.push(section.finish());
            section = Section::default();
            continue;
        }
        section.written = true;
        let lower = c.to_ascii_lowercase();
        let token = match lower {
            // A literal: quoted text, or the character after `\`, `_`, `*`.
            '"' => {
                let (_, after) = rest.split_once('"')?;
                rest = after;
                None
            }
            '\\' | '_' | '*' => {
                let next = rest.chars().next()?;
                rest = rest.get(next.len_utf8()..).unwrap_or_default();
                None
            }
            // A color, a condition, a locale, or elapsed time.
            '[' => {
                let (inside, after) = rest.split_once(']')?;
                rest = after;
                elapsed(inside)
            }
            '@' => {
                section.text = true;
                None
            }
            '0' | '#' | '?' => {
                section.digits = true;
                None
            }
            // Office takes only `E+` and `E-` as exponents (MS-OI29500 on
            // §18.8.31, note e): an `e` is an era year.
            'e' if c == 'E' && (rest.starts_with('+') || rest.starts_with('-')) => {
                section.digits = true;
                None
            }
            'g' if starts_with(rest, "eneral") => {
                section.general = true;
                rest = rest.get("eneral".len()..).unwrap_or_default();
                None
            }
            // `G/` and a localized name (`G/通用格式`, `G/標準`) is General as
            // other writers spell it; the name, in letters other than ASCII,
            // is passed over.
            'g' if rest.starts_with('/') => {
                section.general = true;
                let name = rest.get(1..).unwrap_or_default();
                rest = name.trim_start_matches(|c: char| !c.is_ascii());
                None
            }
            'a' if starts_with(rest, "m/pm") || starts_with(rest, "/p") => {
                let marker = if starts_with(rest, "m/pm") { 4 } else { 2 };
                rest = rest.get(marker..).unwrap_or_default();
                None
            }
            // Any other `a` is the Japanese day of the week (`aaa`).
            'a' => Some(Token::Day),
            // `b1` and `b2` name a calendar; `b` alone is a Buddhist year.
            'b' if rest.starts_with(['1', '2']) => {
                rest = rest.get(1..).unwrap_or_default();
                None
            }
            'y' | 'g' | 'e' | 'b' => Some(Token::Year),
            'm' => Some(Token::M),
            'd' => Some(Token::Day),
            'h' => Some(Token::Hour),
            's' => Some(Token::Second),
            _ => None,
        };
        if let Some(token) = token {
            section.read(token);
        }
    }
    sections.push(section.finish());
    Some(sections)
}

/// Whether `rest` starts with `word`, in any letter case.
fn starts_with(rest: &str, word: &str) -> bool {
    (rest.get(..word.len())).is_some_and(|head| head.eq_ignore_ascii_case(word))
}

/// The elapsed time token brackets holding `inside` stand for: one letter
/// of `h`, `m` or `s`, repeated; `None` for anything else.
fn elapsed(inside: &str) -> Option<Token> {
    let first = inside.chars().next()?.to_ascii_lowercase();
    let same = inside.chars().all(|c| c.to_ascii_lowercase() == first);
    (same && matches!(first, 'h' | 'm' | 's')).then_some(Token::Elapsed(first))
}

impl Section {
    /// Takes in `token`, consecutive tokens of one kind as one. An `m` is
    /// minutes right after hours or right before seconds, among the date
    /// and time tokens, and a month otherwise (§18.8.31, "Month versus
    /// minutes").
    fn read(&mut self, token: Token) {
        if self.last == Some(token) {
            return;
        }
        if self.open_m {
            self.open_m = false;
            let minutes = matches!(token, Token::Second | Token::Elapsed('s'));
            self.time |= minutes;
            self.date |= !minutes;
        }
        match token {
            Token::M if matches!(self.last, Some(Token::Hour | Token::Elapsed('h'))) => {
                self.time = true;
            }
            Token::M => self.open_m = true,
            Token::Year | Token::Day => self.date = true,
            Token::Hour | Token::Second => self.time = true,
            Token::Elapsed(_) => self.elapsed = true,
        }
        self.last = Some(token);
    }

    /// The section once read to its end: an `m` still open is a month.
    fn finish(mut self) -> Self {
        self.date |= self.open_m;
        self.open_m = false;
        self
    }

    /// Whether the section shows the number: it holds a date or time
    /// token, a digit placeholder or `General`. One of literals alone, such
    /// as `"-"` for zero, shows none.
    fn counts(&self) -> bool {
        self.last.is_some() || self.digits || self.general
    }

    /// What the section makes of a number.
    fn kind(&self) -> FormatKind {
        match (self.date, self.time, self.elapsed) {
            (true, _, true) => FormatKind::Unknown,
            (false, _, true) => FormatKind::Duration,
            (true, true, false) => FormatKind::DateTime,
            (true, false, false) => FormatKind::Date,
            (false, true, false) => FormatKind::Time,
            (false, false, false) if self.general && !self.digits => FormatKind::General,
            (false, false, false) => FormatKind::Number,
        }
    }
}

#[cfg(test)]
mod tests;

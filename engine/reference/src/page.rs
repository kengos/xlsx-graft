//! One reference page: the front matter (`title`, `summary`, `tools`) read
//! off the top of the embedded file, and the markdown body after it.
//!
//! A page whose front matter does not have the form [`Page`] describes is
//! left out of [`crate::pages`]; `crate::tests` checks that every embedded
//! page is in it, so such a page fails the tests instead.

use crate::{MAX_PAGE_BYTES, URI_PREFIX};

/// The line that opens and closes the front matter.
const FENCE: &str = "---";

/// One reference page, as [`crate::pages`] and [`crate::page`] return it.
///
/// A page file opens with front matter: exactly three `key: value` lines,
/// `title`, `summary` and `tools` in any order, between two `---` lines.
/// It is not YAML: no other key, no repeated key, no empty `title` or
/// `summary`, and `tools` is a bracketed list of distinct tool names made
/// of `[a-z0-9_]` (`tools: [version]`, or `tools: []`).
///
/// ```text
/// ---
/// title: version
/// summary: The version of the xlsx-graft engine the server was built from.
/// tools: [version]
/// ---
///
/// # version
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Page {
    name: &'static str,
    title: &'static str,
    summary: &'static str,
    tools: Vec<&'static str>,
    body: &'static str,
}

impl Page {
    /// The page's name: its file name in `pages/` without `.md`, and the
    /// `<page>` segment of its URI.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The page's URI, `xlsx-graft://reference/<page>`, where `<page>` is
    /// [`Page::name`].
    #[must_use]
    pub fn uri(&self) -> String {
        format!("{URI_PREFIX}{}", self.name)
    }

    /// The front matter's `title`.
    #[must_use]
    pub fn title(&self) -> &'static str {
        self.title
    }

    /// The front matter's `summary`: one line on what the page covers.
    #[must_use]
    pub fn summary(&self) -> &'static str {
        self.summary
    }

    /// The front matter's `tools`: the names of the MCP tools the page
    /// documents, in the order the page lists them. Empty for a page that
    /// documents no tool.
    #[must_use]
    pub fn tools(&self) -> &[&'static str] {
        &self.tools
    }

    /// The markdown after the front matter, without the blank lines that
    /// separate the two. It starts with the page's `# ` heading.
    #[must_use]
    pub fn body(&self) -> &'static str {
        self.body
    }
}

/// Reads one embedded page. `None` when the source is longer than
/// [`MAX_PAGE_BYTES`], or its front matter is missing, not closed, has a
/// line other than the three keys, repeats a key, leaves a key out or
/// leaves `title` or `summary` empty, or `tools` is not a bracketed list of
/// distinct tool names.
pub(crate) fn parse(name: &'static str, source: &'static str) -> Option<Page> {
    if source.len() > MAX_PAGE_BYTES {
        return None;
    }
    let (front, body) = split(source)?;
    let mut title = None;
    let mut summary = None;
    let mut tools = None;
    for line in front {
        let (key, value) = line.split_once(':')?;
        let value = value.trim();
        match key {
            "title" => set_once(&mut title, text(value)?)?,
            "summary" => set_once(&mut summary, text(value)?)?,
            "tools" => set_once(&mut tools, tool_list(value)?)?,
            _ => return None,
        }
    }
    Some(Page {
        name,
        title: title?,
        summary: summary?,
        tools: tools?,
        body,
    })
}

/// Splits the front matter's lines off the body. A line's `\r\n` ending
/// reads as `\n` too, though `.gitattributes` keeps `pages/*.md` at `\n`
/// in every checkout.
fn split(source: &'static str) -> Option<(Vec<&'static str>, &'static str)> {
    let (first, mut rest) = next_line(source)?;
    if first != FENCE {
        return None;
    }
    let mut front = Vec::new();
    loop {
        let (line, after) = next_line(rest)?;
        rest = after;
        if line == FENCE {
            return Some((front, rest.trim_start_matches(['\r', '\n'])));
        }
        front.push(line);
    }
}

/// The first line of `s` without its line ending, and the text after it.
/// `None` when `s` is empty.
fn next_line(s: &'static str) -> Option<(&'static str, &'static str)> {
    if s.is_empty() {
        return None;
    }
    let (line, rest) = s.split_once('\n').unwrap_or((s, ""));
    Some((line.strip_suffix('\r').unwrap_or(line), rest))
}

/// Stores the first value of a front-matter key. `None` when the key
/// already has a value.
fn set_once<T>(slot: &mut Option<T>, value: T) -> Option<()> {
    if slot.is_some() {
        return None;
    }
    *slot = Some(value);
    Some(())
}

/// A `title` or `summary` value. `None` when it is empty.
fn text(value: &'static str) -> Option<&'static str> {
    (!value.is_empty()).then_some(value)
}

/// A `tools` value, `[name, name]` or `[]`. `None` when the brackets are
/// missing, an entry is not a tool name (`[a-z0-9_]`, not empty) or a tool
/// name is listed twice.
fn tool_list(value: &'static str) -> Option<Vec<&'static str>> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?.trim();
    if inner.is_empty() {
        return Some(Vec::new());
    }
    inner
        .split(',')
        .map(str::trim)
        .try_fold(Vec::new(), |mut tools, tool| {
            if !is_tool_name(tool) || tools.contains(&tool) {
                return None;
            }
            tools.push(tool);
            Some(tools)
        })
}

/// Whether `s` is a tool name: lowercase ASCII letters, digits and `_`,
/// at least one character.
fn is_tool_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

#[cfg(test)]
mod tests;

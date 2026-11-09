//! xlsx-graft reference: the reference pages compiled into the binary, so
//! that a host serves them without a path to configure and without
//! reading a file at run time.
//!
//! A page is a markdown file in this crate's `pages/` directory. Its front
//! matter names its `title`, a one-line `summary` and the `tools` it
//! documents (see [`Page`]); the markdown body after it says how to call
//! each tool and what the call returns. The pages are inside the crate
//! because `include_str!` embeds them when the crate is compiled, and
//! `cargo package` leaves out every file outside the crate's directory.
//!
//! [`pages`] lists every page; [`page`] looks one up by its URI,
//! `xlsx-graft://reference/<page>`. A URI is matched against the embedded
//! list only, so a URI from a client never becomes a file path.
//!
//! The MCP server is the first host: its `list_reference` tool lists the
//! pages with [`HOW_TO_READ`], and its `get_reference` tool returns one
//! page's markdown.

mod embed;
mod error;
mod page;

pub use error::ReferenceError;
pub use page::Page;
use std::sync::OnceLock;

/// The part of every page's URI before the page's name.
pub const URI_PREFIX: &str = "xlsx-graft://reference/";

/// The largest page file, in bytes, front matter included.
///
/// A client reads a page whole, in one response, into its context, so the
/// author of a page that grows past this splits it into several pages. The
/// tests check every embedded page against it, and [`pages`] leaves out a
/// page larger than it.
pub const MAX_PAGE_BYTES: usize = 64 * 1024;

/// How to use the pages: the `howToRead` the MCP `list_reference` tool
/// returns beside the page list. It names the MCP tool `get_reference`.
/// `pages/reference.md` says the same in other words; the two are kept in
/// step by hand.
pub const HOW_TO_READ: &str = "Each page is markdown about the tools named in that page's \
     `tools` in this list: how to call each one, with an example call, and what the call \
     returns. Read the page of a tool before calling it. Fetch a page with get_reference, \
     passing its `uri`.";

/// Every embedded page whose front matter has the form [`Page`] describes
/// and whose file is within [`MAX_PAGE_BYTES`], ordered by name.
#[must_use]
pub fn pages() -> &'static [Page] {
    static PAGES: OnceLock<Vec<Page>> = OnceLock::new();
    PAGES.get_or_init(|| {
        embed::PAGES
            .iter()
            .filter_map(|p| page::parse(p.name, p.source))
            .collect()
    })
}

/// The page whose URI is `uri`, `xlsx-graft://reference/<page>`.
///
/// # Errors
///
/// [`ReferenceError::UnknownUri`] when `uri` does not start with
/// [`URI_PREFIX`] or no page has the name after it. The name after the
/// prefix is compared byte for byte with each page's name: no case
/// folding, no percent-decoding, and a `/` in it is not read as a path
/// separator, so a name with `/` matches no page.
pub fn page(uri: &str) -> Result<&'static Page, ReferenceError> {
    let name = uri
        .strip_prefix(URI_PREFIX)
        .ok_or(ReferenceError::UnknownUri)?;
    pages()
        .iter()
        .find(|p| p.name() == name)
        .ok_or(ReferenceError::UnknownUri)
}

#[cfg(test)]
mod tests;

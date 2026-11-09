//! Tests for the page list and the lookup by URI: the embedded table
//! against the `pages/` directory, every embedded page served, and
//! [`page`] for the URIs it accepts and refuses.

use super::*;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// The crate's `pages/` directory.
fn pages_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pages")
}

/// The name of every entry in `pages/` except `.DS_Store`, the file the
/// macOS Finder writes into a folder it shows (git does not track it, as
/// for `fixtures/`).
fn entries_in_pages_dir() -> BTreeSet<String> {
    std::fs::read_dir(pages_dir())
        .expect("pages/ is readable")
        .map(|entry| {
            let entry = entry.expect("a pages/ entry");
            entry.file_name().to_string_lossy().into_owned()
        })
        .filter(|name| name != ".DS_Store")
        .collect()
}

#[test]
fn the_embedded_table_and_the_pages_directory_list_the_same_pages() {
    let entries = entries_in_pages_dir();
    let on_disk: BTreeSet<String> = entries
        .iter()
        .filter_map(|entry| entry.strip_suffix(".md"))
        .map(str::to_string)
        .collect();
    for name in &on_disk {
        assert!(
            name.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
            "pages/{name}.md: a page name, the last segment of its URI, is [a-z0-9-] only"
        );
    }
    assert!(
        !on_disk.is_empty(),
        "pages/ holds no .md file: the test reads the wrong directory"
    );
    assert_eq!(
        on_disk.len(),
        entries.len(),
        "pages/ holds entries other than .md files: {entries:?}"
    );
    let embedded: BTreeSet<String> = embed::PAGES.iter().map(|p| p.name.to_string()).collect();
    assert_eq!(
        embedded,
        on_disk,
        "missing from embed::PAGES: {:?}; in embed::PAGES but not in pages/: {:?}",
        on_disk.difference(&embedded).collect::<Vec<_>>(),
        embedded.difference(&on_disk).collect::<Vec<_>>()
    );
}

#[test]
fn every_embedded_page_is_served_in_name_order() {
    let served: Vec<&str> = pages().iter().map(Page::name).collect();
    let embedded: Vec<&str> = embed::PAGES.iter().map(|p| p.name).collect();
    assert_eq!(
        served, embedded,
        "pages() leaves out an embedded page: parse() refuses its front matter, or its file is over MAX_PAGE_BYTES"
    );
    let mut sorted = served.clone();
    sorted.sort_unstable();
    assert_eq!(served, sorted, "embed::PAGES is not ordered by name");
}

#[test]
fn every_page_is_within_the_size_limit_and_opens_with_its_title() {
    for source in embed::PAGES {
        assert!(
            source.source.len() <= MAX_PAGE_BYTES,
            "pages/{}.md is {} bytes, over MAX_PAGE_BYTES ({MAX_PAGE_BYTES})",
            source.name,
            source.source.len()
        );
    }
    for page in pages() {
        let heading = format!("# {}\n", page.title());
        assert!(
            page.body().starts_with(&heading),
            "the body of {} does not open with {heading:?}",
            page.name()
        );
        assert!(
            embed::PAGES
                .iter()
                .any(|p| p.name == page.name() && p.source.ends_with(page.body())),
            "the body of {} is not the end of its file",
            page.name()
        );
    }
}

#[test]
fn the_version_page_and_the_reference_page_name_the_tools_they_document() {
    let version = page("xlsx-graft://reference/version").expect("the version page");
    assert_eq!(version.tools(), ["version"]);
    let reference = page("xlsx-graft://reference/reference").expect("the reference page");
    assert_eq!(reference.tools(), ["list_reference", "get_reference"]);
    assert_eq!(reference.title(), "Reading the reference");
    assert!(!reference.summary().is_empty());
}

#[test]
fn page_finds_every_page_by_the_uri_the_page_gives() {
    for listed in pages() {
        let uri = listed.uri();
        assert!(uri.starts_with(URI_PREFIX), "{uri}");
        assert_eq!(page(&uri), Ok(listed));
    }
}

#[test]
fn page_refuses_a_uri_that_names_no_page() {
    for uri in [
        "",
        "version",
        "xlsx-graft://reference/",
        "xlsx-graft://reference/missing",
        "xlsx-graft://reference/VERSION",
        "xlsx-graft://reference/version.md",
        "xlsx-graft://reference/version/",
        "xlsx-graft://reference/../version",
        "xlsx-graft://reference/%76ersion",
        "XLSX-GRAFT://reference/version",
        "other://reference/version",
    ] {
        assert_eq!(page(uri), Err(ReferenceError::UnknownUri), "{uri:?}");
    }
    assert_eq!(
        ReferenceError::UnknownUri.to_string(),
        "no reference page has this URI; the page URIs are xlsx-graft://reference/<page>"
    );
}

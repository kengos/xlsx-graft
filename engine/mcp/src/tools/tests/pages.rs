//! The check that every tool has a reference page: the page each row of
//! `TOOLS` names is embedded and names the tool in its `tools`, and every
//! tool a page names is a row of `TOOLS` that names that page, so each tool
//! is documented by exactly one page. No two rows have the same name. The checks that the `pages/`
//! directory and the embedded page list agree, and that each page file is
//! within `MAX_PAGE_BYTES`, are in the tests of xlsx-graft-reference.

use crate::tools::TOOLS;
use std::collections::BTreeSet;
use xlsx_graft_reference::{URI_PREFIX, page, pages};

#[test]
fn every_tool_is_named_by_the_page_its_row_names() {
    for tool in TOOLS {
        let page = page(&format!("{URI_PREFIX}{}", tool.page))
            .expect("the page a row of TOOLS names is an embedded page");
        assert!(
            page.tools().contains(&tool.name),
            "{} does not name the tool {} in its tools",
            page.uri(),
            tool.name
        );
    }
}

#[test]
fn every_tool_a_page_names_is_a_row_of_tools_that_names_the_page() {
    for page in pages() {
        for name in page.tools() {
            assert!(
                TOOLS
                    .iter()
                    .any(|tool| tool.name == *name && tool.page == page.name()),
                "{} names {name} in its tools, and no row of TOOLS with that name names this page",
                page.uri()
            );
        }
    }
}

#[test]
fn no_two_rows_of_tools_have_the_same_name() {
    let names: BTreeSet<&str> = TOOLS.iter().map(|tool| tool.name).collect();
    assert_eq!(names.len(), TOOLS.len());
}

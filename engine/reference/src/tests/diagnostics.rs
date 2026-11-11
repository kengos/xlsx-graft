//! Tests for the diagnostics pages (`pages/diagnostics-*.md`): every
//! diagnostic code has exactly one `## <code>` section in them, each section
//! states the code's severity, a `Workbook` line names a workbook that is in
//! `fixtures/`, and each page's `summary` names the codes of its sections.

use crate::embed;
use std::path::PathBuf;
use xlsx_graft_diagnostics::Code;

/// One `## <heading>` section of a page: the heading and the lines under it
/// up to the next `## ` heading.
struct Section {
    /// The page's name, the file name without `.md`.
    page: &'static str,
    /// The text after `## `.
    heading: String,
    /// The lines under the heading, outside code fences.
    lines: Vec<&'static str>,
}

/// Whether the page `name` is a diagnostics page.
fn is_diagnostics_page(name: &str) -> bool {
    name.starts_with("diagnostics-")
}

/// The sections of every embedded page for which `wanted(name)` is true,
/// in page-name order. A line between two ```` ``` ```` fences (a fence may
/// be indented) is not read, so a `## ` line inside an example is not a
/// heading. A page that ends inside a fence fails.
fn sections(wanted: impl Fn(&str) -> bool) -> Vec<Section> {
    let mut found: Vec<Section> = Vec::new();
    for page in embed::PAGES.iter().filter(|p| wanted(p.name)) {
        let mut in_fence = false;
        for line in page.source.lines() {
            if line.trim().starts_with("```") {
                in_fence = !in_fence;
            } else if !in_fence && let Some(heading) = line.strip_prefix("## ") {
                found.push(Section {
                    page: page.name,
                    heading: heading.to_string(),
                    lines: Vec::new(),
                });
            } else if !in_fence
                && let Some(section) = found.last_mut().filter(|s| s.page == page.name)
            {
                section.lines.push(line);
            }
        }
        assert!(
            !in_fence,
            "pages/{}.md ends inside a code fence: close the fence that opens a ``` block, \
             or the sections after it are not read",
            page.name
        );
    }
    found
}

/// The sections of the diagnostics pages. Fails when there is none, so a
/// test that loops over them cannot pass by reading nothing.
fn diagnostics_sections() -> Vec<Section> {
    let found = sections(is_diagnostics_page);
    assert!(
        !found.is_empty(),
        "no `## ` section was found in any pages/diagnostics-*.md page; \
         check the page names (they start with \"diagnostics-\") and the heading level (`## `)"
    );
    found
}

#[test]
fn every_code_has_exactly_one_section_in_the_diagnostics_pages() {
    let found = diagnostics_sections();
    for code in Code::ALL {
        let name = code.as_str();
        let pages: Vec<&str> = found
            .iter()
            .filter(|s| s.heading == name)
            .map(|s| s.page)
            .collect();
        assert!(
            !pages.is_empty(),
            "the code {name} has no \"## {name}\" section in any pages/diagnostics-*.md page; \
             add one to the page for its area"
        );
        assert!(
            pages.len() == 1,
            "the code {name} has {} \"## {name}\" sections ({pages:?}); \
             delete the duplicates so that exactly one is left",
            pages.len()
        );
    }
}

#[test]
fn every_heading_of_a_diagnostics_page_is_a_code() {
    let codes: Vec<&str> = Code::ALL.iter().copied().map(Code::as_str).collect();
    for section in diagnostics_sections() {
        assert!(
            codes.contains(&section.heading.as_str()),
            "pages/{}.md has the heading \"## {}\", which is no diagnostic code; \
             a diagnostics page has `## ` headings for codes only (use `###` for a subheading)",
            section.page,
            section.heading
        );
    }
}

#[test]
fn the_sections_of_a_page_follow_the_order_of_the_code_table() {
    let position = |heading: &str| Code::ALL.iter().position(|c| c.as_str() == heading);
    let found = diagnostics_sections();
    for pair in found.windows(2) {
        let (before, after) = (&pair[0], &pair[1]);
        if before.page != after.page {
            continue;
        }
        let (Some(at), Some(next)) = (position(&before.heading), position(&after.heading)) else {
            continue;
        };
        assert!(
            at < next,
            "pages/{}.md has \"## {}\" before \"## {}\", but the code table lists them in a different order; \
             order the sections as Code::ALL does (engine/diagnostics/src/code.rs)",
            before.page,
            before.heading,
            after.heading
        );
    }
}

#[test]
fn every_section_states_the_severity_of_its_code_once() {
    for section in diagnostics_sections() {
        let Some(code) = Code::ALL.iter().find(|c| c.as_str() == section.heading) else {
            continue;
        };
        let line = format!("- Severity: `{}`", code.severity().as_str());
        let severity_lines: Vec<&str> = section
            .lines
            .iter()
            .copied()
            .filter(|l| l.starts_with("- Severity:"))
            .collect();
        assert_eq!(
            severity_lines,
            [line.as_str()],
            "pages/{}.md, section \"## {}\": it needs exactly one line \"{line}\"",
            section.page,
            section.heading
        );
    }
}

#[test]
fn a_workbook_line_names_a_workbook_in_fixtures() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let mut named = 0;
    for section in diagnostics_sections() {
        for line in section
            .lines
            .iter()
            .filter(|l| l.starts_with("- Workbook:"))
        {
            let file = line
                .strip_prefix("- Workbook: `fixtures/")
                .and_then(|rest| rest.split_once('`'))
                .map(|(file, _)| file);
            let Some(file) = file else {
                panic!(
                    "pages/{}.md, section \"## {}\": the line \"{line}\" does not start with \
                     \"- Workbook: `fixtures/<file>`\"",
                    section.page, section.heading
                );
            };
            assert!(
                !file.contains('/') && !file.contains(".."),
                "pages/{}.md, section \"## {}\": the Workbook file \"{file}\" must be a file name alone, \
                 with no `/` and no `..`",
                section.page,
                section.heading
            );
            assert!(
                fixtures.join(file).is_file(),
                "pages/{}.md, section \"## {}\": fixtures/{file} is not in the repository's fixtures/ directory",
                section.page,
                section.heading
            );
            named += 1;
        }
    }
    assert!(
        named > 0,
        "no `- Workbook:` line was found in any pages/diagnostics-*.md page; \
         at least one `- Workbook: `fixtures/<file>`` line is expected"
    );
}

#[test]
fn only_a_diagnostics_page_has_a_heading_that_is_a_code() {
    let codes: Vec<&str> = Code::ALL.iter().copied().map(Code::as_str).collect();
    let others = sections(|name| !is_diagnostics_page(name));
    assert!(
        !others.is_empty(),
        "no `## ` section was found in any page whose name does not start with \"diagnostics-\"; \
         check the heading level (`## `)"
    );
    for section in others {
        assert!(
            !codes.contains(&section.heading.as_str()),
            "pages/{}.md has the heading \"## {}\", a diagnostic code; \
             move the section to a page whose name starts with \"diagnostics-\"",
            section.page,
            section.heading
        );
    }
}

#[test]
fn a_diagnostics_page_documents_no_tool() {
    let listed: Vec<&str> = crate::pages()
        .iter()
        .filter(|p| is_diagnostics_page(p.name()))
        .map(|p| p.name())
        .collect();
    let embedded: Vec<&str> = embed::PAGES
        .iter()
        .filter(|p| is_diagnostics_page(p.name))
        .map(|p| p.name)
        .collect();
    assert!(
        !embedded.is_empty(),
        "no page name starts with \"diagnostics-\": the diagnostics pages are missing"
    );
    assert_eq!(
        listed, embedded,
        "pages() leaves out a diagnostics page: parse() refuses its front matter, or its file is over MAX_PAGE_BYTES"
    );
    for page in crate::pages()
        .iter()
        .filter(|p| is_diagnostics_page(p.name()))
    {
        assert!(
            page.tools().is_empty(),
            "pages/{}.md has `tools: {:?}`; a diagnostics page explains codes, so its `tools` is []",
            page.name(),
            page.tools()
        );
    }
}

#[test]
fn a_page_whose_tools_is_empty_is_a_diagnostics_page() {
    for page in crate::pages().iter().filter(|p| p.tools().is_empty()) {
        assert!(
            is_diagnostics_page(page.name()),
            "pages/{}.md has `tools: []` but its name does not start with \"diagnostics-\"; \
             HOW_TO_READ and pages/reference.md tell a client that a page whose `tools` is empty explains diagnostic codes",
            page.name()
        );
    }
}

/// The codes a summary names: each backticked word, which is a code, or a
/// prefix of codes when it ends with `*` (`zip_*`).
fn summary_forms(summary: &str) -> Vec<&str> {
    summary.split('`').skip(1).step_by(2).collect()
}

/// Whether `code` is one of the codes the `form` of a summary names.
fn matches_form(form: &str, code: &str) -> bool {
    match form.strip_suffix('*') {
        Some(prefix) => code.starts_with(prefix),
        None => form == code,
    }
}

#[test]
fn the_summary_of_a_diagnostics_page_names_the_codes_of_that_page_alone() {
    let pages: Vec<&crate::Page> = crate::pages()
        .iter()
        .filter(|p| is_diagnostics_page(p.name()))
        .collect();
    assert!(
        !pages.is_empty(),
        "no diagnostics page is served by pages(): the diagnostics pages are missing"
    );
    for section in &diagnostics_sections() {
        let naming: Vec<&str> = pages
            .iter()
            .filter(|p| {
                summary_forms(p.summary())
                    .iter()
                    .any(|form| matches_form(form, &section.heading))
            })
            .map(|p| p.name())
            .collect();
        assert_eq!(
            naming,
            [section.page],
            "the code {} is in pages/{}.md, but the summaries that name it, as `{}` or as a prefix such as `zip_*`, \
             are in {naming:?}; write the code in the summary of its own page, and no other page's summary may name it",
            section.heading,
            section.page,
            section.heading
        );
    }
}

#[test]
fn every_form_in_the_summary_of_a_diagnostics_page_names_a_code_of_that_page() {
    let found = diagnostics_sections();
    for page in crate::pages()
        .iter()
        .filter(|p| is_diagnostics_page(p.name()))
    {
        let forms = summary_forms(page.summary());
        assert!(
            !forms.is_empty(),
            "the summary of pages/{}.md names no code in backticks; list its codes, such as `zip_*`",
            page.name()
        );
        for form in forms {
            assert!(
                found
                    .iter()
                    .any(|s| s.page == page.name() && matches_form(form, &s.heading)),
                "the summary of pages/{}.md names `{form}`, which no section of that page has; \
                 remove it from the summary, or add the section",
                page.name()
            );
        }
    }
}

//! Unit tests for the text of a tree: the workbooks Excel saved in
//! `fixtures/`, and packages the tests build to get a part written
//! twice, a cycle, a part nothing references and each line that is not a
//! relationship.

use super::{MAX_DEPTH, TreeText, grouped, short_content_type, short_type};
use crate::policy::limits::HostLimits;
use crate::support::{
    CONTENT_TYPES, FIXTURES, main_rels, package, rel, rels, zip, zip_with_methods,
};
use std::path::Path;

/// The text of the tree of the package `bytes` holds.
fn of_bytes(bytes: &[u8]) -> String {
    TreeText(&crate::tree_bytes(bytes, &HostLimits::default())).to_string()
}

/// The text of a package of the main document `xl/workbook.xml` (named by
/// the package's relationships), each part `xl/<name>.xml` of `others`, and
/// for each `(source, body)` of `links` the Relationships part of
/// `xl/<source>.xml` holding `body`; every part but the Relationships parts
/// is `<a/>`.
fn linked(links: &[(&str, &str)], others: &[&str]) -> String {
    let mut files: Vec<(String, String)> = vec![
        ("_rels/.rels".to_owned(), main_rels()),
        ("xl/workbook.xml".to_owned(), "<a/>".to_owned()),
    ];
    for name in others {
        files.push((format!("xl/{name}.xml"), "<a/>".to_owned()));
    }
    for (source, body) in links {
        files.push((format!("xl/_rels/{source}.xml.rels"), rels(body)));
    }
    let entries: Vec<(&str, &str)> = (files.iter())
        .map(|(name, data)| (name.as_str(), data.as_str()))
        .collect();
    of_bytes(&package(&entries))
}

#[test]
fn the_tree_of_a_workbook_is_written_in_full() {
    let view = crate::tree_path(
        &Path::new(FIXTURES).join("table.xlsx"),
        &HostLimits::default(),
    );
    assert_eq!(
        TreeText(&view).to_string(),
        concat!(
            "/ (package)\n",
            "├── extended-properties rId3 → /docProps/app.xml  [extended-properties+xml, 801 B]\n",
            "├── core-properties rId2 → /docProps/core.xml  [core-properties+xml, 593 B]\n",
            "└── officeDocument rId1 → /xl/workbook.xml  [sheet.main+xml, 2,145 B]\n",
            "    ├── styles rId3 → /xl/styles.xml  [styles+xml, 1,866 B]\n",
            "    ├── theme rId2 → /xl/theme/theme1.xml  [theme+xml, 8,725 B]\n",
            "    ├── worksheet rId1 → /xl/worksheets/sheet1.xml  [worksheet+xml, 1,877 B]\n",
            "    │   └── table rId1 → /xl/tables/table1.xml  [table+xml, 1,239 B]\n",
            "    ├── calcChain rId5 → /xl/calcChain.xml  [calcChain+xml, 237 B]\n",
            "    └── sharedStrings rId4 → /xl/sharedStrings.xml  [sharedStrings+xml, 335 B]\n",
            "9 parts, 0 errors, 0 warnings, 0 notices — ok\n",
        )
    );
}

#[test]
fn a_part_found_a_second_time_is_above_and_its_relationships_are_not_written_again() {
    let workbook = format!(
        "{}{}",
        rel("rId1", "worksheet", "s1.xml"),
        rel("rId2", "styles", "styles.xml")
    );
    let text = linked(
        &[
            ("workbook", &workbook),
            ("s1", &rel("rId1", "styles", "styles.xml")),
        ],
        &["s1", "styles"],
    );
    assert_eq!(
        text,
        concat!(
            "/ (package)\n",
            "└── officeDocument rId1 → /xl/workbook.xml  [application/xml, 4 B]\n",
            "    ├── worksheet rId1 → /xl/s1.xml  [application/xml, 4 B]\n",
            "    │   └── styles rId1 → /xl/styles.xml  [application/xml, 4 B]\n",
            "    └── styles rId2 → /xl/styles.xml  (above)\n",
            "3 parts, 0 errors, 0 warnings, 0 notices — ok\n",
        )
    );
}

#[test]
fn a_cycle_back_to_a_part_written_earlier_ends_with_above() {
    let text = linked(
        &[
            ("workbook", &rel("rId1", "link", "b.xml")),
            ("b", &rel("rId1", "link", "workbook.xml")),
        ],
        &["b"],
    );
    assert_eq!(
        text,
        concat!(
            "/ (package)\n",
            "└── officeDocument rId1 → /xl/workbook.xml  [application/xml, 4 B]\n",
            "    └── link rId1 → /xl/b.xml  [application/xml, 4 B]\n",
            "        └── link rId1 → /xl/workbook.xml  (above)\n",
            "2 parts, 0 errors, 0 warnings, 0 notices — ok\n",
        )
    );
}

#[test]
fn parts_not_yet_written_while_following_the_relationships_from_the_package_follow_the_tree_the_unreferenced_ones_first()
 {
    let text = linked(
        &[
            ("orphan", &rel("rId1", "link", "child.xml")),
            ("x", &rel("rId1", "link", "y.xml")),
            ("y", &rel("rId1", "link", "x.xml")),
        ],
        &["orphan", "child", "x", "y"],
    );
    let (tree, rest) = text.split_once("\n\n").unwrap();
    assert_eq!(
        tree,
        concat!(
            "/ (package)\n",
            "└── officeDocument rId1 → /xl/workbook.xml  [application/xml, 4 B]\n",
            "/xl/orphan.xml  [application/xml, 4 B]  — not referenced\n",
            "└── link rId1 → /xl/child.xml  [application/xml, 4 B]\n",
            "/xl/x.xml  [application/xml, 4 B]\n",
            "└── link rId1 → /xl/y.xml  [application/xml, 4 B]\n",
            "    └── link rId1 → /xl/x.xml  (above)",
        )
    );
    assert!(
        rest.starts_with("notice\n  part_unreferenced /xl/orphan.xml — "),
        "{rest}"
    );
    assert!(
        rest.ends_with("\n5 parts, 0 errors, 0 warnings, 1 notice — ok\n"),
        "{rest}"
    );
}

#[test]
fn when_the_relationships_were_not_all_read_a_line_says_so() {
    let main = main_rels();
    let bytes = zip_with_methods(&[
        ("[Content_Types].xml", CONTENT_TYPES, 0),
        ("_rels/.rels", &main, 99),
        ("xl/workbook.xml", "<a/>", 0),
    ]);
    let text = of_bytes(&bytes);
    assert!(
        text.starts_with(concat!(
            "/ (package)  (some relationships not listed)\n",
            "/xl/workbook.xml  [application/xml, 4 B]\n",
            "the relationships were not all read, so whether a part is referenced is not known\n",
        )),
        "{text}"
    );
    assert!(
        text.contains("\nerror\n  zip_unsupported_compression"),
        "{text}"
    );
}

#[test]
fn a_part_says_when_it_has_no_media_type_or_an_unknown_one_or_cannot_be_read() {
    let none = of_bytes(&package(&[("xl/data.bin", "abc")]));
    assert!(
        none.contains("/xl/data.bin  [no content type, 3 B]"),
        "{none}"
    );
    let unknown = of_bytes(&zip(&[("xl/data.xml", "abc")]));
    assert!(
        unknown.contains("/xl/data.xml  [content type unknown, 3 B]"),
        "{unknown}"
    );
    let refused = of_bytes(&zip_with_methods(&[("xl/data.xml", "abc", 99)]));
    assert!(
        refused.contains("/xl/data.xml  [content type unknown, 3 B, not readable]"),
        "{refused}"
    );
}

#[test]
fn an_external_target_is_marked_and_a_relationship_that_is_not_the_last_has_a_branch_below_it() {
    let body = concat!(
        r#"<Relationship Id="e1" Type="urn:x/hyperlink" Target="https://example.com/" TargetMode="External"/>"#,
        r#"<Relationship Id="e2" Type="urn:x/" Target="file.xlsx" TargetMode="External"/>"#,
    );
    let text = of_bytes(&package(&[("_rels/.rels", &rels(body))]));
    assert!(
        text.starts_with(concat!(
            "/ (package)\n",
            "├── hyperlink e1 → https://example.com/  (external)\n",
            "└── urn:x/ e2 → file.xlsx  (external)\n",
        )),
        "{text}"
    );
}

#[test]
fn a_view_that_could_not_be_read_is_one_line_instead_of_a_tree() {
    let view = crate::tree_path(Path::new("no/such.xlsx"), &HostLimits::default());
    let text = TreeText(&view).to_string();
    assert!(
        text.starts_with("could not read: no/such.xlsx could not be opened: "),
        "{text}"
    );
    assert!(
        text.ends_with("\n0 parts, 0 errors, 0 warnings, 0 notices — not ok\n"),
        "{text}"
    );
    assert_eq!(text.lines().count(), 2, "{text}");
}

#[test]
fn the_diagnostics_follow_the_tree_after_an_empty_line_and_one_part_is_not_plural() {
    let not_a_zip = of_bytes(b"nope");
    assert!(
        not_a_zip.starts_with("/ (package)  (some relationships not listed)\n\nerror\n  not_a_zip"),
        "{not_a_zip}"
    );
    assert!(
        not_a_zip.ends_with("\n0 parts, 1 error, 0 warnings, 0 notices — not ok\n"),
        "{not_a_zip}"
    );
    let one = of_bytes(&zip(&[("xl/a.xml", "<a/>")]));
    let last = one.lines().last().unwrap();
    assert!(last.starts_with("1 part, "), "{one}");
}

#[test]
fn a_size_has_a_comma_between_each_three_digits() {
    let sizes = [0, 7, 999, 1_000, 12_345, 123_456, 1_234_567];
    let written: Vec<String> = sizes.into_iter().map(grouped).collect();
    assert_eq!(
        written,
        ["0", "7", "999", "1,000", "12,345", "123,456", "1,234,567"]
    );
}

#[test]
fn a_relationship_type_is_its_last_segment_and_a_media_type_loses_its_common_start() {
    assert_eq!(short_type("http://x.example/rel/styles"), "styles");
    assert_eq!(short_type("urn:x/"), "urn:x/");
    assert_eq!(short_type("plain"), "plain");
    let start = "application/vnd.openxmlformats-";
    assert_eq!(
        short_content_type(&format!(
            "{start}officedocument.spreadsheetml.worksheet+xml"
        )),
        "worksheet+xml"
    );
    assert_eq!(
        short_content_type(&format!("{start}officedocument.theme+xml")),
        "theme+xml"
    );
    assert_eq!(
        short_content_type(&format!("{start}package.core-properties+xml")),
        "core-properties+xml"
    );
    assert_eq!(short_content_type("application/vnd.ms-excel"), "excel");
    assert_eq!(short_content_type("application/xml"), "application/xml");
}

/// The text of a package of the main document `xl/workbook.xml` whose
/// relationships are `workbook`, and the parts `c1` to `c<count>`, each
/// targeting the next; the relationships of the last are `tail`, and the
/// package also holds the parts `extra`.
fn chained(workbook: &str, count: usize, tail: &str, extra: &[&str]) -> String {
    let names: Vec<String> = (1..=count).map(|n| format!("c{n}")).collect();
    let mut links = vec![("workbook".to_owned(), workbook.to_owned())];
    for (n, name) in names.iter().enumerate() {
        let body = match names.get(n + 1) {
            Some(next) => rel("rId1", "link", &format!("{next}.xml")),
            None => tail.to_owned(),
        };
        links.push((name.clone(), body));
    }
    let links: Vec<(&str, &str)> = (links.iter())
        .map(|(source, body)| (source.as_str(), body.as_str()))
        .collect();
    let mut others: Vec<&str> = names.iter().map(String::as_str).collect();
    others.extend(extra);
    linked(&links, &others)
}

/// How many characters a line has before its branch (`├── ` or `└── `).
fn indent(line: &str) -> usize {
    line.chars().take_while(|c| matches!(c, ' ' | '│')).count()
}

/// The longest indent among the lines of `text` from the one that starts
/// with `from` up to the next one that starts with `/` (a root).
fn deepest_indent_from(text: &str, from: &str) -> usize {
    let mut lines = text.lines().skip_while(|line| !line.starts_with(from));
    lines.next();
    let under: Vec<&str> = lines.take_while(|line| !line.starts_with('/')).collect();
    under.iter().map(|line| indent(line)).max().unwrap()
}

#[test]
fn a_chain_deeper_than_the_limit_is_continued_after_the_tree_as_a_root() {
    // The workbook is level 1, so c31 is level 32: it ends there, and is
    // written again as a root. c32 is level 1 under it, so c63 is level 32:
    // it ends there too, and is a root with c64 to c70 under it.
    let text = chained(&rel("rId1", "link", "c1.xml"), 70, "", &["extra"]);
    assert_eq!(MAX_DEPTH, 32);
    assert_eq!(text.matches("(continued below)").count(), 2);
    assert_eq!(text.matches("(above)").count(), 0);
    for n in [31, 63] {
        assert_eq!(text.matches(&format!("/xl/c{n}.xml  [")).count(), 2, "c{n}");
        let root = format!("\n/xl/c{n}.xml  [application/xml, 4 B]  (continued)\n");
        assert!(text.contains(&root), "{text}");
    }
    for n in (1..=70).filter(|n| *n != 31 && *n != 63) {
        let described = format!("/xl/c{n}.xml  [");
        assert_eq!(text.matches(&described).count(), 1, "c{n}");
    }
    // The first level has no indent, so the last has 4 characters fewer
    // than 4 per level, under the package and under a root.
    let longest = 4 * (MAX_DEPTH - 1);
    assert_eq!(deepest_indent_from(&text, "/ (package)"), longest);
    assert_eq!(deepest_indent_from(&text, "/xl/c31.xml"), longest);
    assert!(
        text.contains("\n72 parts, 0 errors, 0 warnings, 1 notice — ok\n"),
        "{text}"
    );
}

#[test]
fn a_part_not_yet_written_that_is_targeted_twice_at_the_last_level_is_continued_below_twice() {
    // c30 is level 31, so what it targets is level 32.
    let twice = format!(
        "{}{}",
        rel("rId1", "link", "z.xml"),
        rel("rId2", "link", "z.xml")
    );
    let text = chained(&rel("rId1", "link", "c1.xml"), 30, &twice, &["z"]);
    assert_eq!(text.matches("(continued below)").count(), 2);
    assert_eq!(text.matches("(above)").count(), 0);
    assert_eq!(
        text.matches("\n/xl/z.xml  [application/xml, 4 B]  (continued)\n")
            .count(),
        1
    );
}

#[test]
fn a_part_already_written_that_is_targeted_at_the_last_level_is_above() {
    // The workbook targets p, then c1; c30 (level 31) targets p again.
    let head = format!(
        "{}{}",
        rel("rId1", "link", "p.xml"),
        rel("rId2", "link", "c1.xml")
    );
    let text = chained(&head, 30, &rel("rId1", "link", "p.xml"), &["p"]);
    assert_eq!(text.matches("(above)").count(), 1);
    assert_eq!(text.matches("(continued below)").count(), 0);
}

#[test]
fn a_part_continued_below_is_not_a_root_when_a_later_line_wrote_it() {
    // c30 is level 31, so z is level 32 under it and ends there. The
    // workbook then targets z at level 2, which writes it.
    let head = format!(
        "{}{}",
        rel("rId1", "link", "c1.xml"),
        rel("rId2", "link", "z.xml")
    );
    let text = chained(&head, 30, &rel("rId1", "link", "z.xml"), &["z"]);
    assert_eq!(text.matches("(continued below)").count(), 1);
    assert_eq!(text.matches("/xl/z.xml  [").count(), 2);
    assert!(!text.contains("(continued)\n"), "{text}");
    assert!(
        !text.lines().any(|line| line.starts_with("/xl/z.xml")),
        "{text}"
    );
}

#[test]
fn parts_that_ended_a_line_at_continued_below_are_roots_in_that_order_not_in_zip_order() {
    // c30 is level 31, and targets y, then x: both are level 32 and end
    // there. The zip holds x before y.
    let tail = format!(
        "{}{}",
        rel("rId1", "link", "y.xml"),
        rel("rId2", "link", "x.xml")
    );
    let text = chained(&rel("rId1", "link", "c1.xml"), 30, &tail, &["x", "y"]);
    assert_eq!(text.matches("(continued below)").count(), 2);
    let y = text.find("\n/xl/y.xml  [").unwrap();
    let x = text.find("\n/xl/x.xml  [").unwrap();
    assert!(y < x, "{text}");
}

#[test]
fn a_part_with_a_relationship_that_was_not_kept_says_so_after_its_media_type_and_size() {
    let body = format!(
        "{}{}",
        rel("rId1", "link", "missing.xml"),
        rel("rId2", "link", "ok.xml")
    );
    let text = linked(&[("workbook", &body)], &["ok"]);
    assert!(
        text.starts_with(concat!(
            "/ (package)\n",
            "└── officeDocument rId1 → /xl/workbook.xml  [application/xml, 4 B]  (some relationships not listed)\n",
            "    └── link rId2 → /xl/ok.xml  [application/xml, 4 B]\n",
        )),
        "{text}"
    );
    assert!(text.ends_with(" — not ok\n"), "{text}");
}

#[test]
fn a_package_relationship_that_was_not_kept_is_marked_on_the_package_line() {
    let body = rel("rId1", "link", "missing.xml");
    let text = of_bytes(&package(&[("_rels/.rels", &rels(&body))]));
    assert!(
        text.starts_with("/ (package)  (some relationships not listed)\n"),
        "{text}"
    );
}

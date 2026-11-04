//! Tests for the crate, driven through `verify` on packages the tests
//! build (`zip.rs`) with a [`Book`]. `parts.rs` covers the kinds of
//! content a part holds and how the rules judge them, `links.rs` the
//! relationships that point outside the package, `opening.rs` what
//! opening and reading the package refuses on the way, `formulas.rs`
//! the formulas found as the parts are read, and `judging.rs` how they are
//! judged; `strings.rs` covers the shared strings table, and
//! `repairs.rs` what Excel repairs in the workbook and its worksheets.

mod formulas;
mod judging;
mod links;
mod opening;
mod parts;
mod repairs;
mod strings;
mod zip;

use crate::{FunctionClass, FunctionTable, Limits, Rules, Targets, VerifyError, verify};
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location, PartKind};
use zip::Item;

/// The Transitional relationship types up to their name.
const ECMA: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
/// Microsoft's relationship types up to their name.
const MICROSOFT: &str = "http://schemas.microsoft.com/office/2006/relationships/";
/// A worksheet's media type.
const WORKSHEET: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";

/// The reader every test reads a package from: bytes in memory whose reads
/// fail when they start in a range a test sets. One reader type for every
/// test, since the coverage gate counts a line generic over the reader as
/// covered only when one instantiation covers it.
#[derive(Debug)]
struct Reader {
    inner: Cursor<Vec<u8>>,
    /// Reads that start in `start..end` fail, as the host would.
    poisoned: (u64, u64),
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let (start, end) = self.poisoned;
        if (start..end).contains(&self.inner.position()) {
            return Err(io::Error::other("the disk went away"));
        }
        self.inner.read(buf)
    }
}

impl Seek for Reader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// A table that knows `SUM` and `LAMBDA` reviewed safe, `WEBSERVICE`
/// dangerous and `RTD` unreviewed, and nothing else.
struct Table;

impl FunctionTable for Table {
    fn classify(&self, name: &str) -> Option<FunctionClass<'_>> {
        match name {
            "SUM" | "LAMBDA" => Some(FunctionClass::ReviewedSafe),
            "WEBSERVICE" => Some(FunctionClass::Dangerous {
                reason: "reaches the network",
            }),
            "RTD" => Some(FunctionClass::Unreviewed),
            _ => None,
        }
    }
}

/// A workbook the tests build: its parts, each with its media type, and
/// the relationships of each source, the package's own under `/`.
struct Book {
    parts: Vec<(String, String, Vec<u8>)>,
    relationships: Vec<(String, String)>,
    /// Replaces the item of a part of this name as the zip is built.
    replaced: Vec<Item>,
}

impl Book {
    /// A workbook part, `/xl/workbook.xml`, that the package relationships
    /// name the main document, and nothing else.
    fn new() -> Self {
        Self {
            parts: Vec::new(),
            relationships: Vec::new(),
            replaced: Vec::new(),
        }
        .part("/xl/workbook.xml", "application/xml", "<workbook/>")
        .rel("/", "officeDocument", "/xl/workbook.xml")
    }

    /// This workbook, its workbook part of `content_type` holding `xml`.
    fn main(mut self, content_type: &str, xml: &str) -> Self {
        self.parts[0].1 = content_type.to_owned();
        self.parts[0].2 = xml.into();
        self
    }

    /// This workbook, with the part `name` of `content_type` holding `xml`.
    fn part(mut self, name: &str, content_type: &str, xml: &str) -> Self {
        let entry = (name.to_owned(), content_type.to_owned(), xml.into());
        self.parts.push(entry);
        self
    }

    /// This workbook, with the part `name` holding `xml`, related from the
    /// workbook part.
    fn related(self, name: &str, xml: &str) -> Self {
        (self.part(name, "application/xml", xml)).rel("/xl/workbook.xml", "styles", name)
    }

    /// This workbook, with the item of the part `item` names built as
    /// `item` is.
    fn replace(mut self, item: Item) -> Self {
        self.replaced.push(item);
        self
    }

    /// This workbook, with `source` relating to the part `target` by the
    /// ECMA-376 type `name`, or a full type when `name` holds a `:`.
    fn rel(self, source: &str, name: &str, target: &str) -> Self {
        self.relate(source, name, target, "Internal")
    }

    /// [`rel`](Self::rel), to `target` outside the package.
    fn external(self, source: &str, name: &str, target: &str) -> Self {
        self.relate(source, name, target, "External")
    }

    /// The relationship of `rel`, its `TargetMode` `mode`.
    fn relate(mut self, source: &str, name: &str, target: &str, mode: &str) -> Self {
        let rel_type = if name.contains(':') {
            name.to_owned()
        } else {
            format!("{ECMA}{name}")
        };
        let id = 1
            + (self.relationships.iter())
                .filter(|(s, _)| s == source)
                .count();
        let xml = format!(
            r#"<Relationship Id="rId{id}" Type="{rel_type}" Target="{target}" TargetMode="{mode}"/>"#
        );
        self.relationships.push((source.to_owned(), xml));
        self
    }

    /// The items of the zip, in the order the parts were added.
    fn items(self) -> Vec<Item> {
        let mut types = String::from(
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        );
        for (name, content_type, _) in &self.parts {
            types += &format!(r#"<Override PartName="{name}" ContentType="{content_type}"/>"#);
        }
        types += "</Types>";
        let mut items = vec![Item::stored("[Content_Types].xml", types.as_bytes())];
        let mut sources: Vec<&str> = Vec::new();
        for (source, _) in &self.relationships {
            if !sources.contains(&source.as_str()) {
                sources.push(source);
            }
        }
        for source in sources {
            let body: String = (self.relationships.iter())
                .filter(|(s, _)| s == source)
                .map(|(_, xml)| xml.as_str())
                .collect();
            let ns = "http://schemas.openxmlformats.org/package/2006/relationships";
            let xml = format!(r#"<Relationships xmlns="{ns}">{body}</Relationships>"#);
            let (folder, file) = source.rsplit_once('/').unwrap();
            let name = if file.is_empty() {
                String::from("_rels/.rels")
            } else {
                format!("{}/_rels/{file}.rels", &folder[1..])
            };
            items.push(Item::stored(&name, xml.as_bytes()));
        }
        for (name, _, data) in self.parts {
            let name = &name[1..];
            let replaced = self.replaced.iter().find(|item| item.is(name));
            items.push(
                replaced
                    .cloned()
                    .unwrap_or_else(|| Item::stored(name, &data)),
            );
        }
        items
    }

    /// The zip of this workbook.
    fn zip(self) -> Vec<u8> {
        zip::build(self.items())
    }
}

/// What verifying found: each diagnostic, with where it points.
type Seen = Vec<(Diagnostic, String)>;

/// Verifies `zip` under `limits` and `rules`, its reader failing in
/// `poisoned`.
fn run(
    zip: Vec<u8>,
    limits: &Limits,
    rules: &Rules,
    poisoned: (u64, u64),
) -> Result<Seen, VerifyError> {
    let reader = Reader {
        inner: Cursor::new(zip),
        poisoned,
    };
    let mut diagnostics = Diagnostics::new();
    verify(
        reader,
        limits,
        rules,
        &Table,
        &Targets::default(),
        &mut diagnostics,
    )?;
    Ok(seen(diagnostics))
}

/// Verifies `book` under `rules` and the default limits.
fn check_with(book: Book, rules: &Rules) -> Seen {
    run(book.zip(), &Limits::default(), rules, (0, 0)).unwrap()
}

/// Verifies `book` under the default rules and limits.
fn check(book: Book) -> Seen {
    check_with(book, &Rules::default())
}

/// The diagnostics in `diagnostics`, each with where it points: its entry,
/// part, or relationships part and `#` its relationship's `Id`.
fn seen(diagnostics: Diagnostics) -> Seen {
    let place = |location: Location| {
        let id = location.relationship_id.map(|id| id.as_str().to_owned());
        match (location.entry, location.part, location.rels_part) {
            (Some(entry), _, _) => entry.as_str().to_owned(),
            (None, Some(part), _) => part.as_str().to_owned(),
            (None, None, Some(rels)) => format!("{}#{}", rels.as_str(), id.unwrap_or_default()),
            (None, None, None) => String::new(),
        }
    };
    (diagnostics.finish().into_iter())
        .map(|record| (record.diagnostic, place(record.location)))
        .collect()
}

/// `part_denied` of `kind` at `place`.
fn denied(kind: PartKind, place: &str) -> (Diagnostic, String) {
    (Diagnostic::PartDenied { kind }, place.to_owned())
}

#[test]
fn the_table_classifies_only_what_it_knows() {
    assert_eq!(Table.classify("SUM"), Some(FunctionClass::ReviewedSafe));
    assert_eq!(Table.classify("RTD"), Some(FunctionClass::Unreviewed));
    assert_eq!(Table.classify("INDIRECT"), None);
}

//! [`Collector`]: the formulas of one part, found in its XML events as the
//! part is read, with the defined names and tables it declares.
//!
//! A formula is the text of an element, found by local name in any
//! namespace, or the value of an unprefixed attribute of one, whatever the
//! part:
//!
//! - every place SpreadsheetML types a formula (`ST_Formula`, ECMA-376-1
//!   5th edition, `sml.xsd`): the text of `f` (also a chart's `c:f` and an
//!   extension's `xm:f`), `definedName`, `calculatedColumnFormula` and
//!   `totalsRowFormula` (a table column's), `formula1` and `formula2` (a
//!   data validation's), `formula` and `oldFormula` (a conditional
//!   format's, a revision's), and the `macro` of an `objectPr` and the
//!   `macro`, `linkedCell` and `listFillRange` of a `controlPr` (an
//!   embedded object's and a control's);
//! - the places that hold a formula as a string: the `val` of a `cfvo` (a
//!   conditional format's value), the `formula` of a `cacheField` and of a
//!   `calculatedItem` (a pivot cache's), the `link` of an `oleObject` (a
//!   linked object's source), the `refersTo` of a `definedName` (an
//!   external workbook's), the `textlink` of an `sp` (a drawing's text box
//!   linked to cells), the `macro` of a drawing's `sp`, `cxnSp`, `pic` and
//!   `graphicFrame` (`dml-spreadsheetDrawing.xsd`), and the `cellRange` of
//!   a `cameraTool` (a linked picture's source).
//!
//! The text is every run of text, CDATA section and reference inside the
//! element, joined across the comments and processing instructions between
//! them. A formula element nested in another is a formula of its own (an
//! extension's `formula1` holds its formula in an `xm:f`), so the formula
//! elements being read are a stack, bounded by the XML depth limit. Text
//! past the formula limit is counted, not kept.
//!
//! A formula with an element inside it, and text other than white space,
//! is handed over as markup, its text not put together, nor kept once the
//! element is found, so the stack keeps text of the innermost formula
//! alone:
//! which of it Excel reads depends on markup compatibility (a choice it
//! takes, content it processes), and a guess could split a call in two or
//! join what Excel never joins. A formula that is only white space, or
//! empty, is no formula; one left open where reading stops is not handed
//! over: why it stopped is the reader's to report.
//!
//! Where a formula is (`at`) is the reference of the cell it is in (`B2`),
//! its defined name (`definedName Rate`), or its element or attribute with
//! the range of the data validation or conditional format it is in, if any
//! (`formula1 A1:A9`, `cfvo@val C1:C9`). The cell reference, range,
//! defined name and scope are kept up to 1,024 bytes each, so
//! what the stack keeps of them is bounded too; a longer one is no value
//! Excel writes.
//!
//! An attribute is read only unprefixed (`attributes.rs`).

mod attributes;

use super::FormulaSink;
pub(crate) use attributes::attribute;
use attributes::{bytes, value};
use std::borrow::Cow;
use xlsx_graft_xml::events::BytesStart;

/// The elements whose text is a formula.
const ELEMENTS: [&str; 8] = [
    "f",
    "oldFormula",
    "definedName",
    "formula1",
    "formula2",
    "formula",
    "calculatedColumnFormula",
    "totalsRowFormula",
];

/// The formulas of one part, found as it is read.
pub(crate) struct Collector<'a> {
    /// Where what is found goes.
    sink: &'a mut dyn FormulaSink,
    /// The part's name.
    part: String,
    /// The most bytes of a formula kept.
    max_bytes: u64,
    /// The `r` of the cell being read.
    cell: Option<String>,
    /// The `sqref` of the data validation or conditional format being read.
    range: Option<String>,
    /// The formula elements being read, innermost last.
    open: Vec<Open>,
}

/// A formula element being read.
#[derive(Debug)]
struct Open {
    /// Its local name.
    element: String,
    /// Where it is.
    at: String,
    /// For a `definedName`, its `name` and `localSheetId`, as written.
    defines: Option<(String, Option<String>)>,
    /// Its text so far, up to the limit.
    text: String,
    /// The bytes of its text so far, kept or not.
    size: u64,
    /// Whether an element was found inside it.
    markup: bool,
    /// Whether its text so far is only white space.
    blank: bool,
}

impl<'a> Collector<'a> {
    /// A collector for the part `part`, handing what it finds to `sink`
    /// and keeping at most `max_bytes` of a formula.
    pub(crate) fn new(sink: &'a mut dyn FormulaSink, part: &str, max_bytes: u64) -> Self {
        Self {
            sink,
            part: part.to_owned(),
            max_bytes,
            cell: None,
            range: None,
            open: Vec::new(),
        }
    }

    /// Reads the element `start` of local name `local`: a start tag when
    /// `opened`, else an empty element.
    pub(crate) fn start(&mut self, local: &str, start: &BytesStart<'_>, opened: bool) {
        // The innermost alone: each one outside it was marked when the one
        // inside it started.
        if let Some(open) = self.open.last_mut() {
            open.markup = true;
            open.text = String::new();
        }
        match local {
            "c" if opened => self.cell = value(start, "r"),
            "dataValidation" | "conditionalFormatting" if opened => {
                self.range = value(start, "sqref");
            }
            "table" => {
                let names = [value(start, "name"), value(start, "displayName")];
                let names: Vec<String> = names.into_iter().flatten().collect();
                self.sink.table(&self.part, &names);
            }
            _ => self.attributes(local, start),
        }
        if !opened || !ELEMENTS.contains(&local) {
            return;
        }
        let defines = (local == "definedName").then(|| {
            let name = value(start, "name").unwrap_or_default();
            (name, value(start, "localSheetId"))
        });
        let at = match (&defines, &self.cell) {
            (Some((name, _)), _) if name.is_empty() => local.to_owned(),
            (Some((name, _)), _) => format!("definedName {name}"),
            (None, Some(cell)) if local == "f" => cell.clone(),
            _ => self.place(local),
        };
        self.open.push(Open {
            element: local.to_owned(),
            at,
            defines,
            text: String::new(),
            size: 0,
            markup: false,
            blank: true,
        });
    }

    /// Reads the end tag of the element of local name `local`.
    pub(crate) fn end(&mut self, local: &str) {
        match local {
            "c" => self.cell = None,
            "dataValidation" | "conditionalFormatting" => self.range = None,
            _ => {}
        }
        if self.open.last().is_some_and(|open| open.element == local)
            && let Some(open) = self.open.pop()
        {
            self.hand(open);
        }
    }

    /// Reads the text `read` gives, a run of text, a CDATA section or what
    /// a reference stands for, into the formula element read innermost.
    /// Outside a formula element, `read` is not called: a worksheet's
    /// values and a shared strings table's text are no formula's, and are
    /// not decoded.
    pub(crate) fn text<'t>(&mut self, read: impl FnOnce() -> Cow<'t, str>) {
        if let Some(open) = self.open.last_mut() {
            let text = read();
            let text = text.as_ref();
            open.size = open.size.saturating_add(bytes(text));
            open.blank &= text.chars().all(char::is_whitespace);
            if !open.markup && open.size <= self.max_bytes {
                open.text.push_str(text);
            }
        }
    }

    /// Where the formula element or attribute `what` is.
    fn place(&self, what: &str) -> String {
        match &self.range {
            Some(range) => format!("{what} {range}"),
            None => what.to_owned(),
        }
    }

    /// Hands the formula `open` to the sink, as the defined name it
    /// defines, if any, or as markup when an element was found inside it.
    fn hand(&mut self, open: Open) {
        let (part, at, text) = (&self.part, &open.at, &open.text);
        if open.size > self.max_bytes {
            self.sink.too_long(part, at, open.size);
        } else if open.blank {
            // Empty, or white space only: no formula.
        } else if open.markup {
            self.sink.markup(part, at);
        } else if let Some((name, sheet)) = &open.defines {
            (self.sink).defined_name(part, at, name, sheet.as_deref(), text);
        } else {
            self.sink.formula(part, at, text);
        }
    }
}

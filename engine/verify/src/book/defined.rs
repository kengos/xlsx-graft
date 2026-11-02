//! [`read`]: the defined names of the workbook part, as Excel reads them:
//! each `definedName` of the `definedNames` of the root `workbook`, all
//! three in the root's SpreadsheetML namespace (Transitional or Strict,
//! ECMA-376-1:2016 §18.2.5 and §18.2.6).
//!
//! A `definedName` anywhere else, in an extension, in markup
//! compatibility's alternate content or in another namespace, names
//! nothing Excel looks a word up in, so it is left out, as is one with an
//! element inside it, one whose formula is longer than the formula limit,
//! empty or white space (none of which [`FormulaJudge`] would take as a
//! defined name's formula either), and one scoped to a `localSheetId` that
//! is not an `xsd:unsignedInt`. Leaving one out errs towards refusing: a
//! word it would have resolved is denied as a function the table does not
//! know.
//!
//! The part is read in memory, under the XML limits, so it is no larger
//! than `maxXmlBytes`. What the names kept cost grows with their count,
//! a few hundred bytes each beside their text, so only the first
//! `maxDefinedNames` are kept, and the caller refuses a workbook that
//! defines more. A part that cannot be read, or stops being read, names
//! nothing.
//!
//! [`FormulaJudge`]: crate::FormulaJudge

use super::local_sheet_id;
use crate::collect::attribute;
use crate::parts::resolve;
use crate::{Limits, Names};
use std::borrow::Cow;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_package::{Package, PackageError};
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlLimits, XmlReader};

/// The namespaces of SpreadsheetML, Transitional and Strict
/// (ECMA-376-1:2016 §8.1, ECMA-376-4:2016 §8.1).
const SPREADSHEETML: [&str; 2] = [
    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "http://purl.oclc.org/ooxml/spreadsheetml/main",
];

/// The defined names kept, each its name, `localSheetId` and formula, and
/// how many there are.
type Found = (Vec<(String, Option<u32>, String)>, u64);

/// One `definedName` being read.
struct Open {
    /// Its `name`.
    name: String,
    /// Its `localSheetId`, read; `None` for the workbook's.
    sheet: Option<u32>,
    /// Its text so far.
    text: String,
    /// Whether it is left out: it has an element inside it, or its text
    /// is longer than the formula limit.
    left_out: bool,
}

/// Reads the defined names of the workbook part at `index` of `package`
/// into `names`, under `limits`, as the module says, pushing what stops
/// reading it whole into `diagnostics` (it too large, or its zip entry
/// damaged): the names, and how many there are, those past
/// `maxDefinedNames` included.
pub(super) fn read<R: Read + Seek>(
    package: &mut Package<R>,
    index: usize,
    limits: &Limits,
    names: Names,
    diagnostics: &mut Diagnostics,
) -> Result<(Names, u64), PackageError> {
    let xml = &limits.package.xml;
    let Some(bytes) = package.read_xml_part(index, xml.max_bytes, diagnostics)? else {
        return Ok((names, 0));
    };
    let max = usize::try_from(limits.max_formula_bytes.value).unwrap_or(usize::MAX);
    let kept = usize::try_from(limits.max_defined_names.value).unwrap_or(usize::MAX);
    let (found, count) = walk(&bytes, xml, max, kept).unwrap_or_default();
    let names = (found.into_iter()).fold(names, |names, (name, sheet, text)| {
        names.defined_name(&name, sheet, &text)
    });
    Ok((names, count))
}

/// The defined names of the part `bytes`, each kept up to `max` bytes of
/// formula, the first `kept` of them, and how many there are; `None` when
/// reading stops before its end.
fn walk(bytes: &[u8], limits: &XmlLimits, max: usize, kept: usize) -> Option<Found> {
    let mut reader = XmlReader::open(bytes, limits).ok()?;
    let (mut buf, mut found, mut count) = (Vec::new(), Vec::new(), 0_u64);
    // The root's namespace, once the root is a `workbook` in one; how deep
    // the walk is; and the `definedName` open, if any.
    let (mut root, mut depth, mut open): (Option<&str>, u32, Option<Open>) = (None, 0, None);
    // Whether the `definedNames` of the root is open.
    let mut in_names = false;
    loop {
        let event = reader.read_event_into(&mut buf).ok()?;
        let opened = matches!(event, Event::Start(_));
        match event {
            Event::Start(start) | Event::Empty(start) => {
                let (resolved, local) = reader.resolve_element(start.name());
                let ns = match resolved {
                    ResolveResult::Bound(Namespace(ns)) => SPREADSHEETML.iter().find(|&&n| n == ns),
                    _ => None,
                };
                let ours = ns.is_some() && (depth == 0 || ns.copied() == root);
                let local = local.as_ref();
                match (depth, &mut open) {
                    (0, _) if ours && local == "workbook" => root = ns.copied(),
                    (1, _) => {
                        in_names = opened && ours && root.is_some() && local == "definedNames"
                    }
                    (2, None) if opened && ours && in_names && local == "definedName" => {
                        open = defined(&start);
                    }
                    (_, Some(open)) => open.left_out = true,
                    _ => {}
                }
                depth = depth.saturating_add(u32::from(opened));
            }
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                if depth == 2
                    && let Some(done) = open.take()
                {
                    let blank = done.text.chars().all(char::is_whitespace);
                    if !done.left_out && !blank {
                        count = count.saturating_add(1);
                        if found.len() < kept {
                            found.push((done.name, done.sheet, done.text));
                        }
                    }
                }
                in_names &= depth >= 2;
            }
            Event::Text(text) => add(&mut open, max, || text.xml10_content()),
            Event::CData(data) => add(&mut open, max, || data.xml10_content()),
            Event::GeneralRef(reference) => add(&mut open, max, || resolve(&reference).into()),
            Event::Eof => return Some((found, count)),
            _ => {}
        }
        buf.clear();
    }
}

/// The `definedName` `start` opens, unless its `localSheetId` is no
/// `xsd:unsignedInt`.
fn defined(start: &BytesStart<'_>) -> Option<Open> {
    let sheet = match attribute(start, "localSheetId") {
        Some(sheet) => Some(local_sheet_id(&sheet)?),
        None => None,
    };
    Some(Open {
        name: attribute(start, "name").unwrap_or_default(),
        sheet,
        text: String::new(),
        left_out: false,
    })
}

/// Adds the text `read` gives to the `definedName` `open`, if any,
/// leaving it out once its text is longer than `max` bytes. Text outside
/// one is not decoded.
fn add<'t>(open: &mut Option<Open>, max: usize, read: impl FnOnce() -> Cow<'t, str>) {
    if let Some(open) = open {
        open.text.push_str(&read());
        open.left_out |= open.text.len() > max;
        if open.left_out {
            open.text = String::new();
        }
    }
}

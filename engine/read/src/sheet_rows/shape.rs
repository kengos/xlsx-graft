//! The schema shape of a worksheet part: the elements read ([`Frame`]) and
//! where each may stand ([`child`]), and the attributes each may have
//! ([`attributes`]), read into what the walk needs of them ([`Values`]).
//! A cell's inline string is a rich text string, whose shape is
//! `shared_strings/rst.rs`'s.
//!
//! Each frame knows how far into its schema sequence it is (its stage), so
//! a child is admitted only in the schema's order and as often as it may
//! come, as in the other parts.

use super::Halt;
use crate::CellRef;
use crate::compat::Ignorable;
use crate::schema::{is_space, namespace, refuse};
use crate::shared_strings::rst;
use std::io::BufRead;
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};
use xlsx_graft_xml::events::BytesStart;
use xlsx_graft_xml::name::ResolveResult;
use xlsx_graft_xml::{XmlStreamReader, XmlVersion};

/// The children of `worksheet` (CT_Worksheet), in their order.
const WORKSHEET: [&str; 38] = [
    "sheetPr",
    "dimension",
    "sheetViews",
    "sheetFormatPr",
    "cols",
    "sheetData",
    "sheetCalcPr",
    "sheetProtection",
    "protectedRanges",
    "scenarios",
    "autoFilter",
    "sortState",
    "dataConsolidate",
    "customSheetViews",
    "mergeCells",
    "phoneticPr",
    "conditionalFormatting",
    "dataValidations",
    "hyperlinks",
    "printOptions",
    "pageMargins",
    "pageSetup",
    "headerFooter",
    "rowBreaks",
    "colBreaks",
    "customProperties",
    "cellWatches",
    "ignoredErrors",
    "smartTags",
    "drawing",
    "legacyDrawing",
    "legacyDrawingHF",
    "drawingHF",
    "picture",
    "oleObjects",
    "controls",
    "webPublishItems",
    "tableParts",
];

/// An element being read, and its stage in its schema sequence.
#[derive(Debug, Clone, Copy)]
pub(super) enum Frame {
    /// `worksheet` (CT_Worksheet): the children of [`WORKSHEET`], then
    /// `extLst?`, `sheetData` the only one required; `offset` is where it
    /// starts, and `data` whether `sheetData` was read.
    Worksheet { stage: u8, offset: u64, data: bool },
    /// `sheetData` (CT_SheetData): `row*`.
    SheetData,
    /// `row` (CT_Row): `c*`, then `extLst?`.
    Row { stage: u8 },
    /// `c` (CT_Cell): `f?`, `v?`, `is?`, then `extLst?`; `inline` when its
    /// value is its `is`, else its `v`.
    Cell { stage: u8, inline: bool },
    /// `v` (ST_Xstring), whose text is the cell's value.
    Value,
    /// `is`, or an element in it.
    Rst(rst::Frame),
}

/// What a child element is to its parent.
pub(super) enum Child {
    /// An element to read.
    Read(Frame),
    /// An element to skip, with all it holds.
    Skip,
}

/// A cell's type, its `t` (ST_CellType, §18.18.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// `n`, the default: a number.
    Number,
    /// `s`: the index of a shared string.
    Shared,
    /// `str`: a formula's string.
    Formula,
    /// `inlineStr`: the string of its `is`.
    Inline,
    /// `b`: a boolean.
    Bool,
    /// `e`: an error value.
    Error,
    /// `d`: an ISO 8601 date-time.
    Date,
}

/// What the attributes of an element read give.
#[derive(Debug)]
pub(super) struct Values {
    /// `r` of a `row`: its number.
    pub(super) row: Option<u32>,
    /// `r` of a `c`: where it stands.
    pub(super) cell: Option<CellRef>,
    /// `t` of a `c`.
    pub(super) kind: Kind,
    /// `s` of a `c`, when it is an `unsignedInt`; `Some(None)` when it is
    /// not, which matters only to a number.
    pub(super) style: Option<Option<u32>>,
}

/// What the element `local` at `offset` is as a child of `parent`, moving
/// `parent` on to its stage; `None` when it may not stand there.
pub(super) fn child(parent: &mut Frame, local: &str, offset: u64) -> Option<Child> {
    let (at, stage, repeats, child) = match (parent, local) {
        (Frame::Rst(frame), _) => {
            return rst::child(frame, local, offset).map(|child| match child {
                rst::Child::Read(frame) => Child::Read(Frame::Rst(frame)),
                rst::Child::Skip => Child::Skip,
            });
        }
        (Frame::Worksheet { stage: at, .. }, "extLst") => (at, 39, false, Child::Skip),
        (
            Frame::Worksheet {
                stage: at, data, ..
            },
            _,
        ) => {
            let stage = WORKSHEET.iter().position(|&name| name == local)?;
            let child = match local {
                "sheetData" => {
                    *data = true;
                    Child::Read(Frame::SheetData)
                }
                _ => Child::Skip,
            };
            let repeats = matches!(local, "cols" | "conditionalFormatting");
            (at, u8::try_from(stage + 1).ok()?, repeats, child)
        }
        (Frame::SheetData, "row") => return Some(Child::Read(Frame::Row { stage: 0 })),
        (Frame::Row { stage: at }, "c") => {
            let cell = Frame::Cell {
                stage: 0,
                inline: false,
            };
            (at, 1, true, Child::Read(cell))
        }
        (Frame::Row { stage: at }, "extLst") => (at, 2, false, Child::Skip),
        (Frame::Cell { stage: at, inline }, _) => {
            let (stage, child) = match local {
                "f" => (1, Child::Skip),
                "v" if !*inline => (2, Child::Read(Frame::Value)),
                "v" => (2, Child::Skip),
                "is" if *inline => (3, Child::Read(Frame::Rst(rst::Frame::Rst { stage: 0 }))),
                "is" => (3, Child::Skip),
                "extLst" => (4, Child::Skip),
                _ => return None,
            };
            (at, stage, false, child)
        }
        _ => return None,
    };
    let admitted = stage > *at || (stage == *at && repeats);
    *at = stage;
    admitted.then_some(child)
}

/// Reads the attributes of `start`, an element read as `frame` at
/// `offset`, entered in `ignorable`: names in it what its `mc:Ignorable`
/// names, passes over those in them, and refuses one the schema does not
/// give it or a value it reads that is not one of its type. A row's number
/// outside the sheet, or a cell's `r` that names no cell of it, is
/// `cell_reference_invalid`.
pub(super) fn attributes<B: BufRead>(
    frame: &Frame,
    start: &BytesStart<'_>,
    reader: &XmlStreamReader<B>,
    ignorable: &mut Ignorable,
    offset: u64,
) -> Result<Values, Halt> {
    let allowed: &[&str] = match frame {
        Frame::Row { .. } => &[
            "r",
            "spans",
            "s",
            "customFormat",
            "ht",
            "hidden",
            "customHeight",
            "outlineLevel",
            "collapsed",
            "thickTop",
            "thickBot",
            "ph",
        ],
        Frame::Cell { .. } => &["r", "s", "t", "cm", "vm", "ph"],
        Frame::Rst(frame) => rst::allowed(frame),
        Frame::Worksheet { .. } | Frame::SheetData | Frame::Value => &[],
    };
    ignorable.take(start, reader, offset)?;
    let mut values = Values {
        row: None,
        cell: None,
        kind: Kind::Number,
        style: None,
    };
    // Namespace declarations are not attributes.
    let attributes = (start.attributes().flatten()).filter(|attribute| {
        let key = attribute.key.as_ref();
        key != "xmlns" && !key.starts_with("xmlns:")
    });
    for attribute in attributes {
        let name = attribute.key.as_ref();
        let (resolved, local) = reader.resolve_attribute(attribute.key);
        // `xml:space` is bound to XML's own namespace, which needs no
        // declaration.
        let unbound = matches!(resolved, ResolveResult::Unbound) || name.starts_with("xml:");
        if !(unbound && allowed.contains(&name)) {
            if ignorable.passes_over(namespace(&resolved), local.as_ref()) {
                continue;
            }
            return Err(refuse(offset, SchemaReason::UnexpectedAttribute, name).into());
        }
        let read = || attribute.normalized_value(XmlVersion::Implicit1_0);
        let invalid = || Halt::from(refuse(offset, SchemaReason::InvalidValue, name));
        match (frame, name) {
            (Frame::Row { .. }, "r") => {
                let value = read().unwrap_or_default();
                let number = value.trim_matches(is_space).parse::<u32>();
                let number = number.map_err(|_| invalid())?;
                if number == 0 || number > CellRef::MAX_ROW {
                    return Err(reference(&value));
                }
                values.row = Some(number);
            }
            (Frame::Cell { .. }, "r") => {
                let value = read().unwrap_or_default();
                values.cell = Some(CellRef::parse(&value).map_err(|_| reference(&value))?);
            }
            (Frame::Cell { .. }, "t") => {
                values.kind = match read().unwrap_or_default().as_ref() {
                    "n" => Kind::Number,
                    "s" => Kind::Shared,
                    "str" => Kind::Formula,
                    "inlineStr" => Kind::Inline,
                    "b" => Kind::Bool,
                    "e" => Kind::Error,
                    "d" => Kind::Date,
                    _ => return Err(invalid()),
                };
            }
            (Frame::Cell { .. }, "s") => {
                let value = read().unwrap_or_default();
                values.style = Some(value.trim_matches(is_space).parse::<u32>().ok());
            }
            _ => {}
        }
    }
    Ok(values)
}

/// `cell_reference_invalid` for `reference`.
pub(super) fn reference(reference: &str) -> Halt {
    Halt::from(Diagnostic::CellReferenceInvalid {
        reference: Echo::new(reference),
    })
}

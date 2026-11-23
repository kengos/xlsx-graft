//! The elements the walk reads ([`Frame`]) and which child of each it
//! reads ([`Walk::child`]).

use super::attributes::{defined_name, refuse, sheet, view};
use super::{MAX_DEFINED_NAMES, Walk};
use crate::structure::model::DefinedName;
use xlsx_graft_diagnostics::{Diagnostic, SchemaReason};
use xlsx_graft_xml::XmlReader;
use xlsx_graft_xml::events::BytesStart;

/// An element read, held while it is open.
pub(super) enum Frame {
    /// The root `workbook`, which starts at `offset`.
    Workbook { offset: u64 },
    /// `sheets`, which starts at `offset`.
    Sheets { offset: u64 },
    /// `definedNames`.
    DefinedNames,
    /// `bookViews`.
    BookViews,
    /// A `definedName`, whose text is taken in.
    DefinedName(DefinedName),
    /// An element whose children are skipped: a `sheet`, a `workbookView`
    /// or `calcPr` (their attributes are read), or a `definedName` past
    /// `MAX_DEFINED_NAMES` (its attributes are not read either).
    Leaf,
}

impl Frame {
    /// The local name of the element, which [`Walk::child`] matches a child
    /// element against.
    pub(super) fn name(&self) -> &'static str {
        match self {
            Self::Workbook { .. } => "workbook",
            Self::Sheets { .. } => "sheets",
            Self::DefinedNames => "definedNames",
            Self::BookViews => "bookViews",
            Self::DefinedName(_) => "definedName",
            Self::Leaf => "",
        }
    }
}

/// Refuses a second `name`, an element of the root that may come once.
fn once(seen: &mut bool, name: &str, offset: u64) -> Result<(), Diagnostic> {
    if std::mem::replace(seen, true) {
        return Err(refuse(offset, SchemaReason::UnexpectedElement, name));
    }
    Ok(())
}

impl Walk {
    /// The element `local` as a child of `parent`, with what its attributes
    /// give taken in; `None` when it is not one that is read.
    pub(super) fn child(
        &mut self,
        parent: &str,
        local: &str,
        start: &BytesStart<'_>,
        reader: &XmlReader<'_>,
        offset: u64,
    ) -> Result<Option<Frame>, Diagnostic> {
        let name = start.name();
        let name = name.as_ref();
        Ok(Some(match (parent, local) {
            ("workbook", "sheets") => {
                once(&mut self.seen.sheets, name, offset)?;
                Frame::Sheets { offset }
            }
            ("workbook", "definedNames") => {
                once(&mut self.seen.defined_names, name, offset)?;
                Frame::DefinedNames
            }
            ("workbook", "bookViews") => {
                once(&mut self.seen.book_views, name, offset)?;
                Frame::BookViews
            }
            ("workbook", "calcPr") => {
                once(&mut self.seen.calc_pr, name, offset)?;
                self.calc_pr = true;
                Frame::Leaf
            }
            ("sheets", "sheet") => {
                self.sheets.push(sheet(start, reader, self.flavor, offset)?);
                Frame::Leaf
            }
            ("definedNames", "definedName") => {
                // Past the limit the element is counted and not held: it is
                // read as an element whose children are not looked at.
                self.defined_name_count = self.defined_name_count.saturating_add(1);
                if self.defined_name_count > MAX_DEFINED_NAMES {
                    return Ok(Some(Frame::Leaf));
                }
                Frame::DefinedName(defined_name(start, offset)?)
            }
            ("bookViews", "workbookView") => {
                self.views.push(view(start, offset)?);
                Frame::Leaf
            }
            _ => return Ok(None),
        }))
    }
}

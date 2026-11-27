//! `SheetEdit::open`: the worksheet part read once as a stream, for the
//! `mergeCell`s of each `mergeCells`, whether the first `sheetView` of
//! `workbookViewId` 0 has a `pane`, and whether there is a
//! `sheetProtection`.

use super::merges::{Entry, Merges};
use super::name::is_main;
use super::panes::PaneEdit;
use super::protection::ProtectionEdit;
use super::tables;
use super::tag::attribute;
use super::{SheetEdit, columns::ColumnOps, rows::RowOps};
use crate::WriteLimits;
use crate::error::WriteError;
use crate::sheet::CellRange;
use std::io::{BufReader, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location, SchemaReason};
use xlsx_graft_package::{Package, PartName, Presence};
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::{XmlStreamError, XmlStreamReader};

/// Which child of the root the scan is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    MergeCells,
    SheetViews,
}

/// What the scan reads of the part.
#[derive(Default)]
struct Scan {
    /// The elements open: 0 outside the root.
    depth: usize,
    section: Option<Section>,
    /// Whether the first `sheetView` of `workbookViewId` 0 is being read.
    in_view: bool,
    view_seen: bool,
    has_pane: bool,
    has_protection: bool,
    /// How many `cols` the part has.
    cols: usize,
    merges: Vec<Entry>,
}

impl Scan {
    /// The element `start` starts at the current depth; `opened` is whether
    /// it has an end tag.
    fn enter<R: std::io::BufRead>(
        &mut self,
        reader: &XmlStreamReader<R>,
        start: &BytesStart<'_>,
        opened: bool,
    ) -> Result<(), XmlStreamError> {
        let is = |local| is_main(&reader.resolve_element(start.name()), local);
        match self.depth {
            0 if !is("worksheet") => return Err(not_a_worksheet(reader, start)),
            1 if is("mergeCells") => {
                self.section = opened.then_some(Section::MergeCells);
            }
            1 if is("sheetViews") => self.section = opened.then_some(Section::SheetViews),
            1 if is("sheetProtection") => self.has_protection = true,
            1 if is("cols") => self.cols += 1,
            2 if self.section == Some(Section::MergeCells) && is("mergeCell") => {
                let range =
                    attribute(start, "ref").and_then(|text| CellRange::parse(text.trim()).ok());
                self.merges.push(Entry::new(range));
            }
            2 if self.section == Some(Section::SheetViews) && is("sheetView") => {
                let id = attribute(start, "workbookViewId");
                let first =
                    !self.view_seen && id.is_some_and(|id| id.trim().parse::<u32>() == Ok(0));
                self.view_seen |= first;
                self.in_view = first && opened;
            }
            3 if self.in_view && is("pane") => self.has_pane = true,
            _ => {}
        }
        Ok(())
    }

    /// An end tag closes an element.
    fn leave(&mut self) {
        match self.depth {
            1 => self.section = None,
            2 => self.in_view = false,
            _ => {}
        }
    }
}

/// `xml_schema_invalid` for a root that is not a `worksheet` of the main
/// namespace.
fn not_a_worksheet<R: std::io::BufRead>(
    reader: &XmlStreamReader<R>,
    root: &BytesStart<'_>,
) -> XmlStreamError {
    let name = reader.resolve_element(root.name()).1;
    XmlStreamError::Invalid(Diagnostic::XmlSchemaInvalid {
        offset: reader.offset(),
        reason: SchemaReason::UnexpectedElement,
        name: Echo::new(name.as_ref()),
    })
}

/// Reads the part to its end.
fn scan<R: std::io::BufRead>(reader: &mut XmlStreamReader<R>) -> Result<Scan, XmlStreamError> {
    let mut scan = Scan::default();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(start) => {
                scan.enter(reader, &start, true)?;
                scan.depth += 1;
            }
            Event::Empty(start) => scan.enter(reader, &start, false)?,
            Event::End(_) => {
                scan.depth = scan.depth.saturating_sub(1);
                scan.leave();
            }
            Event::Eof => return Ok(scan),
            _ => {}
        }
    }
}

impl SheetEdit {
    /// Reads the worksheet `part` of `package`, as `SheetEdit::open` says.
    pub(super) fn read<R: Read + Seek>(
        package: &mut Package<R>,
        part: &PartName,
        limits: &WriteLimits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Self>, WriteError> {
        if part.is_relationships() {
            return Err(WriteError::PartReserved {
                name: part.to_string(),
            });
        }
        let Presence::Part(index) = package.find(part) else {
            return Err(WriteError::PartMissing {
                name: part.to_string(),
            });
        };
        let (scanned, stopped) = {
            let Some(entry) = package.open_part(index, diagnostics)? else {
                return Ok(None);
            };
            let mut reader = XmlStreamReader::new(BufReader::new(entry), &limits.xml);
            let scanned = scan(&mut reader);
            (scanned, reader.get_ref().get_ref().is_stopped())
        };
        let scan = match scanned {
            Ok(scan) => scan,
            Err(XmlStreamError::Invalid(diagnostic)) => {
                diagnostics.push(diagnostic, Location::default().with_part(part.as_str()));
                return Ok(None);
            }
            // A failure of the zip reader that refused the entry has its
            // diagnostic already; any other is one of the host.
            Err(error) => return if stopped { Ok(None) } else { Err(error.into()) },
        };
        let Some(tables) = tables::read(package, index, &limits.xml, diagnostics)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            part: part.clone(),
            limits: *limits,
            merges: Merges::new(scan.merges, tables),
            columns: ColumnOps::new(scan.cols),
            rows: RowOps::default(),
            panes: PaneEdit::new(scan.has_pane),
            protection: ProtectionEdit::new(scan.has_protection),
        }))
    }
}

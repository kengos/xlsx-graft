//! [`kinds`]: the kinds of content the elements of one part show, and the
//! [`features`] of the compatibility table, read as XML as a stream so a
//! large worksheet costs no more memory than a small one.
//!
//! Every part is read as XML, whatever its media type says: which parts
//! Excel would read as XML is not guessed, so a worksheet that calls
//! itself a binary is read all the same. What its elements show up to the
//! end, or up to where reading stops, counts.
//!
//! Why reading stops is reported for a part whose media type is XML. For
//! any other part, an image, a VBA project or a VML drawing (often not
//! well-formed), it is not when it says only that the part is not XML:
//! malformed XML, or bytes that do not decode or sniff as UTF-32. Excel
//! does not read such a part as
//! the XML part it would have to be to act on what it holds. A limit
//! crossed, a DOCTYPE, and a declaration naming an encoding XML does not
//! use (but UTF-32) or one the bytes are not in, are reported for every
//! part: an event too large may be a comment that hides what follows.
//!
//! What its formulas state is handed to a [`Collector`] as it is read,
//! and a worksheet's XML to its [`SheetCheck`], the part being one the
//! [`Worksheets`] hold.
//!
//! It also tells which of the part's hyperlinks to a Web page or a mail
//! address are used as hyperlinks: named by the `id` of a `hyperlink`,
//! `hlinkClick` or `hlinkHover` element, by no other attribute, and in a
//! part read to its end.

use super::{features, kinds};
use crate::collect::Collector;
use crate::sheet::SheetCheck;
use crate::worksheets::Worksheets;
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Seek};
use stops::not_xml;
use xlsx_graft_compat::Feature;
use xlsx_graft_diagnostics::{Diagnostics, Location, PartKind};
use xlsx_graft_package::{Package, PackageError};
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::{BytesRef, BytesStart, Event};
use xlsx_graft_xml::name::{Namespace, ResolveResult};
use xlsx_graft_xml::{XmlLimits, XmlStreamError, XmlStreamReader};

/// The relationships namespaces, Transitional and Strict
/// (ECMA-376-1:2016 §22.8, ECMA-376-4:2016 §8.1), of an `r:id`.
const RELATIONSHIPS: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "http://purl.oclc.org/ooxml/officeDocument/relationships",
];

/// A part to read.
#[derive(Debug, Clone, Copy)]
pub(super) struct Part<'l> {
    /// Its index in [`Package::parts`].
    pub(super) index: usize,
    /// Whether its media type is XML.
    pub(super) declared: bool,
    /// The `Id`s of its relationships that are hyperlinks to a Web page
    /// or a mail address.
    pub(super) links: &'l BTreeSet<String>,
}

/// What the elements of `part` show, read to its end under `limits`, its
/// formulas handed to `formulas`, and its XML to the checks of its
/// worksheet when `worksheets` holds it, whose problems come before what
/// its zip entry reports. What stops reading is pushed against the part
/// as this module's header says, and what was
/// found before that is kept; a part that cannot be opened, for a reason
/// already reported, shows nothing.
pub(super) fn read<R: Read + Seek>(
    package: &mut Package<R>,
    part: Part<'_>,
    limits: &XmlLimits,
    formulas: &mut Collector<'_>,
    worksheets: &mut Worksheets,
    diagnostics: &mut Diagnostics,
) -> Result<Shown, PackageError> {
    let Part {
        index,
        declared,
        links,
    } = part;
    let location = (package.parts().get(index))
        .map(|part| Location::default().with_part(part.name().as_str()))
        .unwrap_or_default();
    let mut walked = Walked::default();
    // What the zip entry pushes, as it is opened and as its damage shows,
    // kept apart while a worksheet's checks push as it is read.
    let mut opened = Diagnostics::new();
    let refused = {
        let Some(entry) = package.open_part(index, &mut opened)? else {
            append(opened, diagnostics);
            return Ok(walked.shown());
        };
        let mut xml = XmlStreamReader::new(BufReader::new(entry), limits);
        let name = location.part.as_ref().map_or("", |part| part.as_str());
        let mut sheet = worksheets.check(index, name, diagnostics);
        let walked_through = walk(&mut xml, links, &mut walked, formulas, sheet.as_mut());
        if let Some(sheet) = sheet {
            worksheets.read(index, sheet.finish());
        }
        if walked_through.is_err() {
            // What follows where reading stops may use a hyperlink for
            // something else: none is admitted.
            walked.linked.clear();
        }
        match walked_through {
            Ok(()) => Ok(None),
            Err(XmlStreamError::Invalid(diagnostic)) => Ok(Some(diagnostic)),
            // A zip entry that stops on damage of its own has reported it.
            Err(XmlStreamError::Io(_)) if xml.get_ref().get_ref().is_stopped() => Ok(None),
            Err(XmlStreamError::Io(error)) => Err(error),
        }
    };
    append(opened, diagnostics);
    let refused = refused.map_err(PackageError::Io)?;
    if let Some(diagnostic) = refused
        && (declared || !not_xml(&diagnostic))
    {
        diagnostics.push(diagnostic, location);
    }
    Ok(walked.shown())
}

/// Pushes what `from` holds into `diagnostics`.
fn append(from: Diagnostics, diagnostics: &mut Diagnostics) {
    for record in from.finish() {
        diagnostics.push(record.diagnostic, record.location);
    }
}

/// What a part's elements show.
#[derive(Debug, Default)]
pub(super) struct Shown {
    /// The kinds of content they name.
    pub(super) kinds: BTreeSet<PartKind>,
    /// The features of the compatibility table they show.
    pub(super) features: BTreeSet<Feature>,
    /// The `Id`s of the hyperlinks they use as hyperlinks alone.
    pub(super) hyperlinks: BTreeSet<String>,
    /// Whether one is a `ddeLink` or an `oleLink`, which makes an external
    /// workbook reference that link alone.
    pub(super) dde_or_ole_link: bool,
}

/// What reading a part found.
#[derive(Debug, Default)]
struct Walked {
    /// The kinds its elements show.
    kinds: BTreeSet<PartKind>,
    /// The features its elements show.
    features: BTreeSet<Feature>,
    /// The hyperlinks a hyperlink element names.
    linked: BTreeSet<String>,
    /// The hyperlinks another element names.
    misused: BTreeSet<String>,
    /// Whether it opened a `ddeLink` or an `oleLink`.
    dde_or_ole_link: bool,
}

impl Walked {
    /// Notes the hyperlinks among `links` that an attribute of `start`
    /// names: as used as a hyperlink by the `id` of a `hyperlink` element,
    /// and as used for something else by any other attribute.
    fn note(&mut self, start: &BytesStart<'_>, hyperlink: bool, links: &BTreeSet<String>) {
        for attribute in start.attributes().flatten() {
            let value = attribute.normalized_value(XmlVersion::Implicit1_0);
            let Some(id) = value.ok().and_then(|value| links.get(value.as_ref())) else {
                continue;
            };
            let set = if hyperlink && attribute.key.local_name().as_ref() == "id" {
                &mut self.linked
            } else {
                &mut self.misused
            };
            set.insert(id.clone());
        }
    }

    /// What was found.
    fn shown(self) -> Shown {
        let hyperlinks = self.linked.difference(&self.misused).cloned().collect();
        Shown {
            kinds: self.kinds,
            features: self.features,
            hyperlinks,
            dde_or_ole_link: self.dde_or_ole_link,
        }
    }
}

/// Reads `xml` to its end, putting in `walked` the kind of every element
/// that names one and the use of each of `links` an element names, and
/// handing every element and text to `formulas` and `sheet`, if any.
fn walk<R: BufRead>(
    xml: &mut XmlStreamReader<R>,
    links: &BTreeSet<String>,
    walked: &mut Walked,
    formulas: &mut Collector<'_>,
    mut sheet: Option<&mut SheetCheck<'_>>,
) -> Result<(), XmlStreamError> {
    let mut buf = Vec::new();
    loop {
        let event = xml.read_event_into(&mut buf)?;
        let opened = matches!(event, Event::Start(_));
        match event {
            Event::Start(start) | Event::Empty(start) => {
                let (resolved, local) = xml.resolve_element(start.name());
                let local = local.as_ref();
                if let Some(sheet) = sheet.as_deref_mut() {
                    let id = || relationship_id(xml, &start);
                    sheet.start(&resolved, local, &start, opened, id);
                }
                formulas.start(local, &start, opened);
                walked.kinds.extend(kinds::element(local));
                if let ResolveResult::Bound(Namespace(ns)) = resolved {
                    walked.features.extend(features::element(ns, local, &start));
                }
                walked.dde_or_ole_link |= matches!(local, "ddeLink" | "oleLink");
                if !links.is_empty() {
                    let hyperlink = matches!(local, "hyperlink" | "hlinkClick" | "hlinkHover");
                    walked.note(&start, hyperlink, links);
                }
            }
            Event::End(end) => {
                if let Some(sheet) = sheet.as_deref_mut() {
                    sheet.end();
                }
                formulas.end(xml.resolve_element(end.name()).1.as_ref());
            }
            Event::Text(text) => {
                if let Some(sheet) = sheet.as_deref_mut() {
                    sheet.text(|| text.xml10_content());
                }
                formulas.text(|| text.xml10_content());
            }
            Event::CData(data) => {
                if let Some(sheet) = sheet.as_deref_mut() {
                    sheet.text(|| data.xml10_content());
                }
                formulas.text(|| data.xml10_content());
            }
            Event::GeneralRef(reference) => {
                if let Some(sheet) = sheet.as_deref_mut() {
                    sheet.text(|| resolve(&reference).into());
                }
                formulas.text(|| resolve(&reference).into());
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
        buf.clear();
    }
}

/// The `r:id` of `start`, read by `xml`: its attribute `id` in a
/// relationships namespace.
pub(crate) fn relationship_id<R: BufRead>(
    xml: &XmlStreamReader<R>,
    start: &BytesStart<'_>,
) -> Option<String> {
    (start.attributes().flatten()).find_map(|attribute| {
        let (resolved, local) = xml.resolve_attribute(attribute.key);
        let ours =
            matches!(resolved, ResolveResult::Bound(Namespace(ns)) if RELATIONSHIPS.contains(&ns));
        let value = attribute.normalized_value(XmlVersion::Implicit1_0);
        (ours && local.as_ref() == "id").then(|| value.ok().map(|value| value.into_owned()))?
    })
}

/// The text a reference the XML reader admitted stands for: one of the
/// five predefined entities or a character reference.
pub(crate) fn resolve(reference: &BytesRef<'_>) -> String {
    const ENTITIES: [(&str, char); 5] = [
        ("lt", '<'),
        ("gt", '>'),
        ("amp", '&'),
        ("apos", '\''),
        ("quot", '"'),
    ];
    let named = ENTITIES.iter().find(|(name, _)| *name == &**reference);
    let c = reference.resolve_char_ref().ok().flatten();
    c.or(named.map(|&(_, c)| c))
        .map(String::from)
        .unwrap_or_default()
}

mod stops;

pub(crate) use stops::stops_reading;

#[cfg(test)]
mod tests;

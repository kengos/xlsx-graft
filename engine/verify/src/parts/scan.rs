//! [`kinds`]: the kinds of content the elements of one part show, read as
//! XML as a stream so a large worksheet costs no more memory than a small
//! one.
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
//! What its formulas state is handed to a [`Collector`] as it is read.
//!
//! It also tells which of the part's hyperlinks to a Web page or a mail
//! address are used as hyperlinks: named by the `id` of a `hyperlink`,
//! `hlinkClick` or `hlinkHover` element, by no other attribute, and in a
//! part read to its end.

use super::kinds;
use crate::collect::Collector;
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, EncodingReason, Location, PartKind};
use xlsx_graft_package::{Package, PackageError};
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::{BytesRef, BytesStart, Event};
use xlsx_graft_xml::{XmlLimits, XmlStreamError, XmlStreamReader};

/// What the elements of the part at `index` show, read to its end under
/// `limits`; `declared` when its media type is XML, and `links` the `Id`s
/// of its relationships that are hyperlinks to a Web page or a mail
/// address, its formulas handed to `formulas`. What stops reading is
/// pushed against the part as this module's header says, and what was
/// found before that is kept; a part that cannot be opened, for a reason
/// already reported, shows nothing.
pub(super) fn read<R: Read + Seek>(
    package: &mut Package<R>,
    index: usize,
    declared: bool,
    links: &BTreeSet<String>,
    limits: &XmlLimits,
    formulas: &mut Collector<'_>,
    diagnostics: &mut Diagnostics,
) -> Result<Shown, PackageError> {
    let location = (package.parts().get(index))
        .map(|part| Location::default().with_part(part.name().as_str()))
        .unwrap_or_default();
    let mut walked = Walked::default();
    let refused = {
        let Some(entry) = package.open_part(index, diagnostics)? else {
            return Ok(walked.shown());
        };
        let mut xml = XmlStreamReader::new(BufReader::new(entry), limits);
        let walked_through = walk(&mut xml, links, &mut walked, formulas);
        if walked_through.is_err() {
            // What follows where reading stops may use a hyperlink for
            // something else: none is admitted.
            walked.linked.clear();
        }
        match walked_through {
            Ok(()) => None,
            Err(XmlStreamError::Invalid(diagnostic)) => Some(diagnostic),
            // A zip entry that stops on damage of its own has reported it.
            Err(XmlStreamError::Io(_)) if xml.get_ref().get_ref().is_stopped() => None,
            Err(XmlStreamError::Io(error)) => return Err(PackageError::Io(error)),
        }
    };
    if let Some(diagnostic) = refused
        && (declared || !not_xml(&diagnostic))
    {
        diagnostics.push(diagnostic, location);
    }
    Ok(walked.shown())
}

/// What a part's elements show.
#[derive(Debug, Default)]
pub(super) struct Shown {
    /// The kinds of content they name.
    pub(super) kinds: BTreeSet<PartKind>,
    /// The `Id`s of the hyperlinks they use as hyperlinks alone.
    pub(super) hyperlinks: BTreeSet<String>,
    /// Whether one is a `ddeLink` or an `oleLink`, which makes an external
    /// workbook reference that link alone.
    pub(super) dde_or_ole_link: bool,
}

/// Whether `diagnostic`, stopping a part's reading, says only that the
/// part is not XML.
fn not_xml(diagnostic: &Diagnostic) -> bool {
    match diagnostic {
        Diagnostic::XmlMalformed { .. } => true,
        // Bytes sniffed as UTF-32 are not XML: an EMF image starts
        // `01 00 00 00`. A declaration naming UTF-32 is silenced with them;
        // Excel reads no such part either.
        Diagnostic::XmlEncodingUnsupported {
            encoding,
            reason: EncodingReason::OtherEncoding,
        } => encoding.as_str().eq_ignore_ascii_case("UTF-32"),
        // A declaration that names an encoding XML does not use, or one the
        // bytes are not in, was read as XML; bytes that do not decode were not.
        Diagnostic::XmlEncodingUnsupported { reason, .. } => {
            *reason != EncodingReason::DeclarationMismatch
        }
        _ => false,
    }
}

/// What reading a part found.
#[derive(Debug, Default)]
struct Walked {
    /// The kinds its elements show.
    kinds: BTreeSet<PartKind>,
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
            hyperlinks,
            dde_or_ole_link: self.dde_or_ole_link,
        }
    }
}

/// Reads `xml` to its end, putting in `walked` the kind of every element
/// that names one and the use of each of `links` an element names, and
/// handing every element and text to `formulas`.
fn walk<R: BufRead>(
    xml: &mut XmlStreamReader<R>,
    links: &BTreeSet<String>,
    walked: &mut Walked,
    formulas: &mut Collector<'_>,
) -> Result<(), XmlStreamError> {
    let mut buf = Vec::new();
    loop {
        let event = xml.read_event_into(&mut buf)?;
        let opened = matches!(event, Event::Start(_));
        match event {
            Event::Start(start) | Event::Empty(start) => {
                let (_, local) = xml.resolve_element(start.name());
                let local = local.as_ref();
                formulas.start(local, &start, opened);
                walked.kinds.extend(kinds::element(local));
                walked.dde_or_ole_link |= matches!(local, "ddeLink" | "oleLink");
                if !links.is_empty() {
                    let hyperlink = matches!(local, "hyperlink" | "hlinkClick" | "hlinkHover");
                    walked.note(&start, hyperlink, links);
                }
            }
            Event::End(end) => formulas.end(xml.resolve_element(end.name()).1.as_ref()),
            Event::Text(text) => formulas.text(|| text.xml10_content()),
            Event::CData(data) => formulas.text(|| data.xml10_content()),
            Event::GeneralRef(reference) => formulas.text(|| resolve(&reference).into()),
            Event::Eof => return Ok(()),
            _ => {}
        }
        buf.clear();
    }
}

/// The text a reference the XML reader admitted stands for: one of the
/// five predefined entities or a character reference.
fn resolve(reference: &BytesRef<'_>) -> String {
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

#[cfg(test)]
mod tests;

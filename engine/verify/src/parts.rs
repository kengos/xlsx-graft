//! [`check`]: the kinds of content each part holds (`kinds.rs`), each
//! judged by the [`Rules`].
//!
//! A relationship that points outside the package is admitted only as a
//! hyperlink to a Web page or a mail address that its part uses as a
//! hyperlink and as nothing else (`scan.rs`): the type alone does not make
//! one, so an OLE link cannot borrow it.
//!
//! A part's kinds come from its media type, the types of the relationships
//! that target it and its elements (`scan.rs`), every part but a
//! Relationships part (which the package reader has read) being read as
//! XML whatever its media type. An external workbook reference that holds
//! a DDE or OLE link is that link alone. A relationship that points
//! outside the package counts against the part that declares it, at the
//! relationship, as an external link and as the kind its type names, each
//! unless that part already holds it. Each part and kind is reported once,
//! at the first place found; the package's own relationships come first,
//! then each part in the order of [`Package::parts`].

mod kinds;
mod scan;

use crate::collect::{Collector, FormulaSink};
use crate::{Rules, Ruling};
use scan::Shown;
use std::collections::BTreeSet;
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location, PartKind};
use xlsx_graft_package::{Package, PackageError, Relationship, Target};
use xlsx_graft_xml::XmlLimits;

/// The package's own Relationships part.
const PACKAGE_RELATIONSHIPS: &str = "/_rels/.rels";

/// Pushes a diagnostic for each kind of content a part of `package` holds
/// that `rules` do not allow, reading its XML parts under `limits` and
/// handing their formulas, kept up to `max_formula_bytes`, to `formulas`.
pub(crate) fn check<R: Read + Seek>(
    package: &mut Package<R>,
    limits: &XmlLimits,
    rules: &Rules,
    max_formula_bytes: u64,
    formulas: &mut dyn FormulaSink,
    diagnostics: &mut Diagnostics,
) -> Result<(), PackageError> {
    let mut held = targeted(package);
    let mut scanned = Vec::new();
    for (index, part) in package.parts().iter().enumerate() {
        let media_type = part.content_type().map(|t| t.essence());
        if let Some(set) = held.get_mut(index) {
            set.extend(media_type.and_then(kinds::media_type));
        }
        if !part.name().is_relationships() {
            let links = web_or_mail_links(part.relationships());
            let name = part.name().as_str().to_owned();
            scanned.push((index, name, media_type.is_some_and(is_xml), links));
        }
    }
    let mut shown: Vec<Shown> = held.iter().map(|_| Shown::default()).collect();
    for (index, name, declared, links) in scanned {
        let mut collector = Collector::new(formulas, &name, max_formula_bytes);
        let found = &mut collector;
        let read = scan::read(package, index, declared, &links, limits, found, diagnostics)?;
        if let Some(slot) = shown.get_mut(index) {
            *slot = read;
        }
    }
    let mut findings = Findings::default();
    let outside = package.relationships();
    findings.outside(None, outside, &BTreeSet::new(), PACKAGE_RELATIONSHIPS);
    let parts = package.parts().iter().enumerate().zip(held).zip(shown);
    for (((index, part), mut set), shown) in parts {
        set.extend(shown.kinds);
        let link = shown.dde_or_ole_link;
        set.retain(|kind| !(link && *kind == PartKind::ExternalLink));
        let name = part.name().as_str();
        for kind in set {
            findings.add(Some(index), kind, Location::default().with_part(name));
        }
        let rels = rels_of(name);
        findings.outside(Some(index), part.relationships(), &shown.hyperlinks, &rels);
    }
    findings.judge(rules, diagnostics);
    Ok(())
}

/// The kinds of content each part of `package` is the target of a
/// relationship for, by its index in [`Package::parts`].
fn targeted<R: Read + Seek>(package: &Package<R>) -> Vec<BTreeSet<PartKind>> {
    let mut held = vec![BTreeSet::new(); package.parts().len()];
    let declared = (package.parts().iter()).flat_map(|part| part.relationships());
    for relationship in package.relationships().iter().chain(declared) {
        if let Target::Internal(target) = relationship.target()
            && let Some(set) = held.get_mut(*target)
        {
            set.extend(kinds::relationship(relationship.rel_type()));
        }
    }
    held
}

/// The `Id`s of `relationships` that are hyperlinks to a Web page or a
/// mail address.
fn web_or_mail_links(relationships: &[Relationship]) -> BTreeSet<String> {
    (relationships.iter())
        .filter(|relationship| match relationship.target() {
            Target::External(target) => kinds::web_or_mail_link(relationship.rel_type(), target),
            Target::Internal(_) => false,
        })
        .map(|relationship| relationship.id().to_owned())
        .collect()
}

/// Whether a part of the media type `essence` is XML (RFC 7303 §4.2, §9.2).
fn is_xml(essence: &str) -> bool {
    essence.ends_with("+xml") || essence == "application/xml" || essence == "text/xml"
}

/// The name of the Relationships part of the part `name` (ECMA-376-2:2021
/// §6.5.2.3), which a diagnostic about one of its relationships points to.
fn rels_of(name: &str) -> String {
    let (folder, file) = name.rsplit_once('/').unwrap_or_default();
    format!("{folder}/_rels/{file}.rels")
}

/// The kinds of content found, each at the first place found, once per part.
#[derive(Debug, Default)]
struct Findings {
    /// Each part (`None` for the package) and kind found so far.
    seen: BTreeSet<(Option<usize>, PartKind)>,
    /// Each kind, where it was first found, in the order found.
    found: Vec<(PartKind, Location)>,
}

impl Findings {
    /// Notes that `place` holds content of `kind`, found at `location`.
    fn add(&mut self, place: Option<usize>, kind: PartKind, location: Location) {
        if self.seen.insert((place, kind)) {
            self.found.push((kind, location));
        }
    }

    /// Notes each of `relationships`, declared in `rels` for `place`, that
    /// points outside the package, but the `hyperlinks` its part uses as
    /// hyperlinks: as an external link, and as the kind its type names, if
    /// any. Allowing that kind does not let it reach outside.
    fn outside(
        &mut self,
        place: Option<usize>,
        relationships: &[Relationship],
        hyperlinks: &BTreeSet<String>,
        rels: &str,
    ) {
        for relationship in relationships {
            if !matches!(relationship.target(), Target::External(_))
                || hyperlinks.contains(relationship.id())
            {
                continue;
            }
            let location =
                (Location::default().with_rels_part(rels)).with_relationship_id(relationship.id());
            let named = kinds::relationship(relationship.rel_type());
            for kind in [Some(PartKind::ExternalLink), named].into_iter().flatten() {
                self.add(place, kind, location.clone());
            }
        }
    }

    /// Pushes a diagnostic for each kind found that `rules` do not allow.
    fn judge(self, rules: &Rules, diagnostics: &mut Diagnostics) {
        for (kind, location) in self.found {
            let diagnostic = match rules.part(kind) {
                Ruling::Forbidden { reason } => Diagnostic::PartForbidden {
                    kind,
                    reason: Echo::new(reason),
                },
                Ruling::Allowed => continue,
                Ruling::Unruled => Diagnostic::PartDenied { kind },
            };
            diagnostics.push(diagnostic, location);
        }
    }
}

#[cfg(test)]
mod tests;

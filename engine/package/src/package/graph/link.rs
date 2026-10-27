//! [`Package::link`]: one relationship of a Relationships part, its target
//! resolved and linked into the graph.

use super::super::{Incoming, Package, Presence, RelationshipSource};
use crate::relationships::{self, Raw};
use crate::{Relationship, Target};
use std::io::{Read, Seek};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};

/// What every relationship of one Relationships part is resolved with.
pub(super) struct Context {
    /// The Relationships part's name.
    pub(super) rels: String,
    /// The name targets are resolved against.
    pub(super) base: String,
    pub(super) from: RelationshipSource,
}

impl<R: Read + Seek> Package<R> {
    /// `raw` as the relationship at `index` of its source, linked into the
    /// graph; `None` when its target names no part, and `Err` when it is
    /// refused.
    pub(super) fn link(
        &mut self,
        raw: Raw,
        context: &Context,
        index: usize,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Relationship>, ()> {
        let location = Location::default()
            .with_rels_part(&context.rels)
            .with_relationship_id(&raw.id);
        let target = if raw.external {
            Target::External(raw.target)
        } else {
            let echo = Echo::new(&raw.target);
            let resolved = match relationships::resolve(&context.base, &raw.target) {
                Ok(resolved) => resolved,
                Err(reason) => {
                    let diagnostic = Diagnostic::RelationshipTargetInvalid {
                        target: echo,
                        reason,
                    };
                    diagnostics.push(diagnostic, location);
                    return Err(());
                }
            };
            if resolved.above_root {
                let diagnostic = Diagnostic::RelationshipTargetAboveRoot { target: echo };
                diagnostics.push(diagnostic, location.clone());
            }
            let name = resolved.name;
            if name.is_relationships() {
                diagnostics.push(Diagnostic::RelationshipRelsPart {}, location);
                return Err(());
            }
            let target = match self.find(&name) {
                Presence::Part(target) => target,
                Presence::Absent => {
                    let part_name = Echo::new(name.as_str());
                    let diagnostic = Diagnostic::RelationshipTargetMissing { part_name };
                    diagnostics.push(diagnostic, location);
                    return Ok(None);
                }
                Presence::Unknown => return Ok(None),
            };
            if let Some(part) = self.parts.get_mut(target) {
                part.incoming.push(Incoming {
                    source: context.from,
                    index,
                });
            }
            Target::Internal(target)
        };
        Ok(Some(Relationship {
            id: raw.id,
            rel_type: raw.rel_type,
            target,
        }))
    }
}

//! [`Found`]: the findings so far, one per part, defined name and
//! finding, each with the first place it was found and how many times.

use super::Place;
use super::body::{Finding, Verdict};
use std::collections::HashMap;
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location};

/// The findings so far, in the order first found.
#[derive(Default)]
pub(super) struct Found {
    /// Where each part's, defined name's and finding's entry is.
    index: HashMap<(String, Option<usize>, Finding), usize>,
    /// Each finding, in the order first found.
    entries: Vec<Entry>,
}

/// One finding, where it was first found, and how many times.
struct Entry {
    finding: Finding,
    location: Location,
    count: u64,
}

impl Found {
    /// Counts `finding` at `place`, held by the formula of the defined
    /// name `holder` (its index and name) when it was reached through one.
    pub(super) fn add(
        &mut self,
        place: &Place<'_>,
        holder: Option<(usize, &str)>,
        finding: &Finding,
    ) {
        let key = (
            place.part.to_owned(),
            holder.map(|(index, _)| index),
            finding.clone(),
        );
        if let Some(entry) = self
            .index
            .get(&key)
            .and_then(|&at| self.entries.get_mut(at))
        {
            entry.count = entry.count.saturating_add(1);
            return;
        }
        let mut location = Location::default()
            .with_part(place.part)
            .with_place(place.at);
        if let Some((_, name)) = holder {
            location = location.with_defined_name(name);
        }
        self.index.insert(key, self.entries.len());
        self.entries.push(Entry {
            finding: finding.clone(),
            location,
            count: 1,
        });
    }

    /// Pushes a diagnostic for each finding into `diagnostics`.
    pub(super) fn finish(self, diagnostics: &mut Diagnostics) {
        for entry in self.entries {
            diagnostics.push(diagnostic(entry.finding, entry.count), entry.location);
        }
    }
}

/// The diagnostic of `finding`, found `count` times.
fn diagnostic(finding: Finding, count: u64) -> Diagnostic {
    match finding {
        Finding::Malformed(reason) => Diagnostic::FormulaMalformed { reason, count },
        Finding::Denied(kind) => Diagnostic::FormulaDenied { kind, count },
        Finding::Forbidden(kind, reason) => Diagnostic::FormulaForbidden {
            kind,
            reason: Echo::new(&reason),
            count,
        },
        Finding::Function(name, verdict) => {
            let name = Echo::new(&name);
            match verdict {
                Verdict::Forbidden(reason) => Diagnostic::FunctionForbidden {
                    name,
                    reason: Echo::new(&reason),
                    count,
                },
                Verdict::Dangerous(reason) => Diagnostic::FunctionDangerous {
                    name,
                    reason: Echo::new(&reason),
                    count,
                },
                Verdict::Denied(denial) => Diagnostic::FunctionDenied {
                    name,
                    denial,
                    count,
                },
            }
        }
    }
}

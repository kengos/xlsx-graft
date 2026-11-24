//! The `definedName` elements: each original one dropped, written again or
//! written as it was, and the `definedNames` element that is dropped when no name is
//! left or written in full when it was an empty element.

use super::insert::{text_of, with_attributes};
use super::{NameOut, Run, Section};
use crate::WriteError;
use xlsx_graft_xml::events::{BytesEnd, BytesStart, BytesText, Event};

/// `start` with `out`'s name and `localSheetId` (left out when `out` has
/// none), the other attributes as they are.
pub(super) fn rebuild(start: &BytesStart<'_>, out: &NameOut) -> BytesStart<'static> {
    let local = out.local.map(|local| local.to_string());
    let set = [
        ("name", Some(out.name.as_str())),
        ("localSheetId", local.as_deref()),
    ];
    with_attributes(start, &set)
}

impl Run<'_> {
    /// Handles the `definedNames` `start` when the plan changes a name:
    /// dropped with all it holds when no name is written, else its `start`
    /// is written as it was (and its names are handled one by one) or, for an empty
    /// element, it is written with the names added.
    pub(super) fn names_start(
        &mut self,
        start: &BytesStart<'_>,
        opened: bool,
    ) -> Result<bool, WriteError> {
        if self.plan.names_total == 0 {
            self.skipping = opened.then_some(self.depth);
            return Ok(true);
        }
        if opened {
            self.section = Section::Names;
            return Ok(false);
        }
        let end = BytesEnd::new(start.name().as_ref().to_owned());
        self.rw.write(Event::Start(start.to_owned()))?;
        self.write_added()?;
        self.rw.write(Event::End(end))?;
        Ok(true)
    }

    /// Handles the `definedName` `start` inside `definedNames`: the one at
    /// the next position among the original names.
    pub(super) fn name(
        &mut self,
        start: &BytesStart<'_>,
        opened: bool,
    ) -> Result<bool, WriteError> {
        let index = self.name_index;
        self.name_index = index.saturating_add(1);
        if self.plan.removed.contains(&index) {
            self.skipping = opened.then_some(self.depth);
            return Ok(true);
        }
        let Some(out) = self.plan.rebuilt.get(&index) else {
            return Ok(false);
        };
        let end = BytesEnd::new(start.name().as_ref().to_owned());
        self.rw.write(Event::Start(rebuild(start, out)))?;
        self.rw
            .write(Event::Text(BytesText::from_escaped(text_of(&out.text))))?;
        self.rw.write(Event::End(end))?;
        self.skipping = opened.then_some(self.depth);
        Ok(true)
    }
}

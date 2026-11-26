//! The `workbookView` of a workbook that has none, which `set_active_sheet`
//! adds: written in the `bookViews` the part has (an empty element, or a
//! start tag whose children are not `workbookView`s), or in a new `bookViews`
//! before `sheets`, the place CT_Workbook gives it (ECMA-376-1:2016
//! §18.2.27). A `bookViews` the part has is used wherever it is.

use super::{Run, with_attributes};
use crate::WriteError;
use xlsx_graft_xml::events::{BytesEnd, BytesStart, Event};

/// The new values of the attributes of a `workbookView`.
pub(super) struct ViewOut {
    pub(super) active: Option<String>,
    pub(super) first: Option<String>,
}

impl ViewOut {
    /// Whether an attribute is written.
    pub(super) fn is_set(&self) -> bool {
        self.active.is_some() || self.first.is_some()
    }
}

impl Run<'_> {
    /// Handles the `bookViews` `start` (a start tag when `opened`, else an
    /// empty element) when the plan adds a `workbookView` and no `bookViews`
    /// was written for it: writes the element with the view in it. Whether
    /// the element was written, so the event is not written as it came.
    pub(super) fn views_start(
        &mut self,
        start: &BytesStart<'_>,
        opened: bool,
    ) -> Result<bool, WriteError> {
        // `pending_views` (and the `.filter(|_| added)` below) is a guard
        // only: the model refuses a second `bookViews` (`model/walk/child.rs`,
        // `once`), so it is always true when this is reached.
        let added = std::mem::take(&mut self.pending_views);
        let plan = self.plan;
        let Some(view) = plan.insert_views.as_ref().filter(|_| added) else {
            return Ok(false);
        };
        let end = BytesEnd::new(start.name().as_ref().to_owned());
        self.rw.write(Event::Start(start.clone().into_owned()))?;
        self.write_view(view)?;
        if !opened {
            self.rw.write(Event::End(end))?;
        }
        Ok(true)
    }

    /// Writes `bookViews` with the `workbookView` the plan adds.
    pub(super) fn views_block(&mut self) -> Result<(), WriteError> {
        let plan = self.plan;
        let name = format!("{}bookViews", self.prefix);
        self.rw.write(Event::Start(BytesStart::new(name.clone())))?;
        for view in plan.insert_views.iter() {
            self.write_view(view)?;
        }
        Ok(self.rw.write(Event::End(BytesEnd::new(name)))?)
    }

    /// Writes the `workbookView` of `view`.
    fn write_view(&mut self, view: &ViewOut) -> Result<(), WriteError> {
        let start = BytesStart::new(format!("{}workbookView", self.prefix));
        let set = [
            ("activeTab", view.active.as_deref()),
            ("firstSheet", view.first.as_deref()),
        ];
        self.put(with_attributes(&start, &set), false)
    }
}

//! The pieces of the workbook part written new or changed in place that are
//! not `sheets` or `definedName`: the attributes of `workbookView` and
//! `calcPr`, the `definedNames` and `calcPr` elements the part lacked, and
//! the start tag [`with_attributes`] makes.

use super::Run;
use super::names::rebuild;
use crate::WriteError;
use std::convert::Infallible;
use xlsx_graft_cell::{xml_attribute, xml_text};
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::{BytesEnd, BytesStart, BytesText, Event};

/// The children of the root `bookViews` is written before, when the part
/// has none: the ones that follow it (ECMA-376-1:2016 §18.2.27).
const VIEWS_BEFORE: [&str; 14] = [
    "sheets",
    "functionGroups",
    "externalReferences",
    "definedNames",
    "calcPr",
    "oleSize",
    "customWorkbookViews",
    "pivotCaches",
    "smartTagPr",
    "smartTagTypes",
    "webPublishing",
    "fileRecoveryPr",
    "webPublishObjects",
    "extLst",
];

/// The children of the root `definedNames` is written before, when the part
/// has none: the ones that follow it (ECMA-376-1:2016 §18.2.27).
const NAMES_BEFORE: [&str; 10] = [
    "calcPr",
    "oleSize",
    "customWorkbookViews",
    "pivotCaches",
    "smartTagPr",
    "smartTagTypes",
    "webPublishing",
    "fileRecoveryPr",
    "webPublishObjects",
    "extLst",
];

/// The children of the root `calcPr` is written before: those of
/// [`NAMES_BEFORE`] that follow it.
const CALC_BEFORE: [&str; 9] = [
    "oleSize",
    "customWorkbookViews",
    "pivotCaches",
    "smartTagPr",
    "smartTagTypes",
    "webPublishing",
    "fileRecoveryPr",
    "webPublishObjects",
    "extLst",
];

/// `start` with the attributes of `set` given the value (`None` leaves the
/// attribute out), in place when the element has it and after the others
/// when it does not; every other attribute keeps its value. The values are
/// written through `xml_attribute`, so one that is a value of ST_Xstring has
/// been through `escape` already.
pub(in crate::structure::edit) fn with_attributes(
    start: &BytesStart<'_>,
    set: &[(&str, Option<&str>)],
) -> BytesStart<'static> {
    let name = start.name().as_ref().to_owned();
    let mut content = name.clone();
    let mut written: Vec<String> = Vec::new();
    for attribute in start.attributes().flatten() {
        let key = attribute.key.as_ref();
        let value = match set.iter().find(|(set_key, _)| *set_key == key) {
            Some((_, value)) => {
                written.push(key.to_owned());
                value.map(str::to_owned)
            }
            None => {
                let value = attribute.normalized_value(XmlVersion::Implicit1_0);
                Some(value.unwrap_or_default().into_owned())
            }
        };
        push(&mut content, key, value.as_deref());
    }
    for (key, value) in set {
        if !written.iter().any(|done| done == key) {
            push(&mut content, key, *value);
        }
    }
    BytesStart::from_content(content, name.len())
}

/// Adds ` key="value"` to `content`, the value escaped for an attribute.
fn push(content: &mut String, key: &str, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    content.push(' ');
    content.push_str(key);
    content.push_str("=\"");
    xml_attribute::<Infallible>(value, |piece| {
        content.push_str(piece);
        Ok(())
    })
    .unwrap_or_default();
    content.push('"');
}

/// `text` escaped for XML text.
pub(in crate::structure::edit) fn text_of(text: &str) -> String {
    let mut out = String::new();
    xml_text::<Infallible>(text, |piece| {
        out.push_str(piece);
        Ok(())
    })
    .unwrap_or_default();
    out
}

impl Run<'_> {
    /// Writes `start` as a start tag when `opened`, else as an empty
    /// element.
    pub(super) fn put(
        &mut self,
        start: BytesStart<'static>,
        opened: bool,
    ) -> Result<(), WriteError> {
        let event = if opened {
            Event::Start(start)
        } else {
            Event::Empty(start)
        };
        Ok(self.rw.write(event)?)
    }

    /// Writes the `definedNames` and `calcPr` elements still to be added,
    /// when `before` is `None` (the end of the root) or names a child they
    /// are written before.
    pub(super) fn insert_pending(&mut self, before: Option<&str>) -> Result<(), WriteError> {
        let views = before.is_none_or(|local| VIEWS_BEFORE.contains(&local));
        if self.plan.views_block && self.pending_views && views {
            self.views_block()?;
            self.pending_views = false;
        }
        let names = before.is_none_or(|local| NAMES_BEFORE.contains(&local));
        if self.pending_names && names {
            self.names_block()?;
            self.pending_names = false;
        }
        let calc = before.is_none_or(|local| CALC_BEFORE.contains(&local));
        if self.pending_calc && calc {
            let name = format!("{}calcPr", self.prefix);
            let set = [("fullCalcOnLoad", Some("1"))];
            self.put(with_attributes(&BytesStart::new(name), &set), false)?;
            self.pending_calc = false;
        }
        Ok(())
    }

    /// Writes `definedNames` with the names added.
    fn names_block(&mut self) -> Result<(), WriteError> {
        let name = format!("{}definedNames", self.prefix);
        self.rw.write(Event::Start(BytesStart::new(name.clone())))?;
        self.write_added()?;
        Ok(self.rw.write(Event::End(BytesEnd::new(name)))?)
    }

    /// Writes the `definedName` elements the plan adds.
    pub(super) fn write_added(&mut self) -> Result<(), WriteError> {
        for out in &self.plan.added {
            let name = format!("{}definedName", self.prefix);
            let start = rebuild(&BytesStart::new(name), out);
            self.rw.write(Event::Start(start.clone()))?;
            self.rw
                .write(Event::Text(BytesText::from_escaped(text_of(&out.text))))?;
            let end = BytesEnd::new(start.name().as_ref().to_owned());
            self.rw.write(Event::End(end))?;
        }
        Ok(())
    }

    /// Writes `calcPr` (the empty element or the start tag) with
    /// `fullCalcOnLoad="1"`.
    pub(super) fn calc(
        &mut self,
        start: &BytesStart<'_>,
        opened: bool,
    ) -> Result<bool, WriteError> {
        self.put(
            with_attributes(start, &[("fullCalcOnLoad", Some("1"))]),
            opened,
        )?;
        Ok(true)
    }

    /// Writes the `workbookView` `start` with the values the plan gives it,
    /// when it gives it any.
    pub(super) fn view(
        &mut self,
        start: &BytesStart<'_>,
        opened: bool,
    ) -> Result<bool, WriteError> {
        let index = self.view_index;
        self.view_index = index.saturating_add(1);
        let view = self.plan.views.get(index);
        let values = [
            ("activeTab", view.and_then(|view| view.active.as_deref())),
            ("firstSheet", view.and_then(|view| view.first.as_deref())),
        ];
        let set: Vec<(&str, Option<&str>)> = (values.iter())
            .filter(|(_, value)| value.is_some())
            .copied()
            .collect();
        if set.is_empty() {
            return Ok(false);
        }
        self.put(with_attributes(start, &set), opened)?;
        Ok(true)
    }
}

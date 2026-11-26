//! The `sheets` element: its `sheet` elements kept until its end, then
//! written in the order the plan gives, the new ones with them. A child that
//! is no `sheet` of the root's namespace (another namespace's element,
//! alternate content, an unknown element) is not a sheet of the model, which
//! counts only the `sheet` elements: it is written after the sheets, in its
//! order, as it was. So is a comment or a processing instruction between the
//! `sheet` elements; whitespace text between them is not written.

use super::insert::with_attributes;
use super::{Run, Section, SheetOut, namespace};
use crate::WriteError;
use crate::structure::edit::ns::RELATIONSHIPS;
use xlsx_graft_xml::XmlVersion;
use xlsx_graft_xml::events::{BytesStart, Event};

/// What a `sheet` writes its `r:id` as.
#[derive(Clone)]
struct Rid {
    /// The attribute's name as written, with its prefix.
    key: String,
    /// The declaration of the prefix, when the `sheet` itself makes it.
    declaration: Option<(String, String)>,
}

impl Default for Rid {
    fn default() -> Self {
        Self {
            key: "r:id".to_owned(),
            declaration: None,
        }
    }
}

/// The children of `sheets` read so far, each as the events it is made of.
#[derive(Default)]
pub(super) struct Collected {
    /// The `sheet` elements, by position among them.
    groups: Vec<Vec<Event<'static>>>,
    /// The other children.
    others: Vec<Vec<Event<'static>>>,
    /// The child that is open: whether it is a `sheet`, and its events.
    open: Option<(bool, Vec<Event<'static>>)>,
    /// Whether the first `sheet` was read, which `rid` is taken from.
    taken: bool,
    rid: Rid,
}

impl Collected {
    /// Keeps `group`, the events of a child that is a `sheet` or not.
    fn file(&mut self, sheet: bool, group: Vec<Event<'static>>) {
        let list = if sheet {
            &mut self.groups
        } else {
            &mut self.others
        };
        list.push(group);
    }
}

impl Run<'_> {
    /// Keeps `event`, read inside `sheets`, to write later; at the end of
    /// `sheets` writes the sheets and the end.
    pub(super) fn collect(&mut self, event: Event<'_>, before: usize) -> Result<(), WriteError> {
        match &event {
            Event::Start(start) if before == 2 => {
                let sheet = self.local(start).as_deref() == Some("sheet");
                self.take_rid(start, sheet);
                self.sheets.open = Some((sheet, vec![event.into_owned()]));
            }
            Event::Empty(start) if before == 2 => {
                let sheet = self.local(start).as_deref() == Some("sheet");
                self.take_rid(start, sheet);
                self.sheets.file(sheet, vec![event.into_owned()]);
            }
            Event::Comment(_) | Event::PI(_) if before == 2 => {
                self.sheets.file(false, vec![event.into_owned()]);
            }
            Event::End(_) if before == 2 => return self.flush(event),
            Event::End(_) if before == 3 => {
                let (sheet, mut group) = self.sheets.open.take().unwrap_or_default();
                group.push(event.into_owned());
                self.sheets.file(sheet, group);
            }
            _ => {
                let owned = event.into_owned();
                (self.sheets.open.iter_mut()).for_each(|(_, group)| group.push(owned.clone()));
            }
        }
        Ok(())
    }

    /// Remembers how the first `sheet` `start` writes `r:id`; `sheet` is
    /// whether `start` is a `sheet`.
    fn take_rid(&mut self, start: &BytesStart<'_>, sheet: bool) {
        if !sheet || std::mem::replace(&mut self.sheets.taken, true) {
            return;
        }
        let ours = |uri: Option<&str>| RELATIONSHIPS.iter().any(|ns| uri == Some(*ns));
        let key = start.attributes().flatten().find_map(|attribute| {
            let (resolved, local) = self.rw.resolve_attribute(attribute.key);
            let found = local.as_ref() == "id" && ours(namespace(&resolved));
            found.then(|| attribute.key.as_ref().to_owned())
        });
        let found = key.map(|key| {
            let prefix = key
                .split_once(':')
                .map(|(prefix, _)| format!("xmlns:{prefix}"));
            let declaration = start.attributes().flatten().find_map(|attribute| {
                let name = attribute.key.as_ref();
                let value = attribute.normalized_value(XmlVersion::Implicit1_0);
                let found = prefix.as_deref() == Some(name);
                found.then(|| (name.to_owned(), value.unwrap_or_default().into_owned()))
            });
            Rid { key, declaration }
        });
        self.sheets.rid = found.unwrap_or_default();
    }

    /// Writes the sheets of the plan, then the end of `sheets`.
    fn flush(&mut self, end: Event<'_>) -> Result<(), WriteError> {
        let plan = self.plan;
        for out in plan.sheets.iter().flatten() {
            match out {
                SheetOut::Original { index, name, state } => {
                    let mut set: Vec<(&str, Option<&str>)> = Vec::new();
                    set.extend(name.as_deref().map(|name| ("name", Some(name))));
                    set.extend(state.map(|state| ("state", state)));
                    let group = self.sheets.groups.get(*index).cloned().unwrap_or_default();
                    for (at, event) in group.into_iter().enumerate() {
                        self.original(at, event, &set)?;
                    }
                }
                SheetOut::Added {
                    name,
                    sheet_id,
                    rid,
                    state,
                } => self.added(name, *sheet_id, rid, *state)?,
            }
        }
        for event in self.sheets.others.iter().flatten() {
            self.rw.write(event.clone())?;
        }
        self.section = Section::None;
        self.sheets = Collected::default();
        Ok(self.rw.write(end)?)
    }

    /// Writes `event`, the `at`th of an original sheet's events: the first
    /// one with the attributes `set` gives (`None` leaves one out).
    fn original(
        &mut self,
        at: usize,
        event: Event<'static>,
        set: &[(&str, Option<&str>)],
    ) -> Result<(), WriteError> {
        let changed = match &event {
            Event::Start(start) if at == 0 && !set.is_empty() => {
                Event::Start(with_attributes(start, set))
            }
            Event::Empty(start) if at == 0 && !set.is_empty() => {
                Event::Empty(with_attributes(start, set))
            }
            _ => event,
        };
        Ok(self.rw.write(changed)?)
    }

    /// Writes an element for a sheet added, `r:id` written as the first
    /// original sheet writes it, and `state` when it has one.
    fn added(
        &mut self,
        name: &str,
        sheet_id: u32,
        rid: &str,
        state: Option<&str>,
    ) -> Result<(), WriteError> {
        let Rid { key, declaration } = self.sheets.rid.clone();
        let id = sheet_id.to_string();
        let start = BytesStart::new(format!("{}sheet", self.prefix));
        let mut set = vec![("name", Some(name)), ("sheetId", Some(id.as_str()))];
        set.extend(
            declaration
                .iter()
                .map(|(k, v)| (k.as_str(), Some(v.as_str()))),
        );
        set.push((key.as_str(), Some(rid)));
        set.push(("state", state));
        Ok(self.rw.write(Event::Empty(with_attributes(&start, &set)))?)
    }
}

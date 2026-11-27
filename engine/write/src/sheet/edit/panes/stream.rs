//! Writes the `sheetViews` of the part with the `pane` and the `selection`s
//! of the first `sheetView` of `workbookViewId` 0 replaced: a `pane` of the
//! frozen panes and a `selection` in the pane that scrolls, written right
//! after its start tag. Nothing frozen removes its `pane` and the `selection`s
//! that name a pane.

use super::PaneEdit;
use crate::error::WriteError;
use crate::sheet::FrozenPanes;
use crate::sheet::edit::apply::Rw;
use crate::sheet::edit::name::is_main;
use crate::sheet::edit::pass::through;
use crate::sheet::edit::tag::{attribute, name_of, write_empty, write_end, write_start};
use xlsx_graft_xml::events::{BytesStart, Event};

/// The attributes of the `pane` and of the `selection` to write.
type Attributes = Vec<(&'static str, String)>;

/// The `pane` and the `selection` that freezing writes, `None` when nothing
/// is frozen.
type Children = Option<(Attributes, Attributes)>;

/// The `pane` and the `selection` the frozen panes write.
fn new_children(panes: &FrozenPanes) -> Children {
    (panes.pane_attributes()).zip(panes.selection_attributes())
}

/// Writes the `pane` and the `selection`.
fn write_children(rw: &mut Rw<'_>, prefix: &str, children: &Children) -> Result<(), WriteError> {
    if let Some((pane, selection)) = children {
        write_empty(rw, &format!("{prefix}pane"), pane)?;
        write_empty(rw, &format!("{prefix}selection"), selection)?;
    }
    Ok(())
}

/// Writes a `sheetView` of `workbookViewId` 0 holding the `children`, none
/// when nothing is frozen.
fn write_new_view(rw: &mut Rw<'_>, prefix: &str, children: &Children) -> Result<(), WriteError> {
    children.iter().try_for_each(|_| {
        let name = format!("{prefix}sheetView");
        write_start(rw, &name, &[("workbookViewId", "0".to_owned())])?;
        write_children(rw, prefix, children)?;
        write_end(rw, &name)
    })
}

/// Whether `child` is `local` of the main namespace.
fn is(rw: &Rw<'_>, child: &BytesStart<'_>, local: &str) -> bool {
    is_main(&rw.resolve_element(child.name()), local)
}

/// Whether `view` is a `sheetView` of `workbookViewId` 0.
fn is_first_view(rw: &Rw<'_>, view: &BytesStart<'_>) -> bool {
    let id = attribute(view, "workbookViewId");
    is(rw, view, "sheetView") && id.is_some_and(|id| id.trim().parse::<u32>() == Ok(0))
}

/// Whether `ext` is the `extLst` of a `sheetViews` that a new `sheetView`
/// goes before: nothing in the `sheetViews` took the place yet and
/// something is frozen.
fn is_ext_after_view(rw: &Rw<'_>, ext: &BytesStart<'_>, done: bool, children: &Children) -> bool {
    !done && children.is_some() && is(rw, ext, "extLst")
}

/// Whether `child` of a `sheetView` is not written: its `pane`, and its
/// `selection`s when something is frozen, else the ones that name a pane.
fn removes(rw: &Rw<'_>, child: &BytesStart<'_>, children: &Children) -> bool {
    is(rw, child, "pane")
        || (is(rw, child, "selection")
            && (children.is_some() || attribute(child, "pane").is_some()))
}

impl PaneEdit {
    /// Writes the `sheetView` `view` of `workbookViewId` 0 (`opened`: it has
    /// an end tag) with its `pane` and `selection`s replaced.
    fn view(
        rw: &mut Rw<'_>,
        prefix: &str,
        (view, opened): (&BytesStart<'_>, bool),
        children: &Children,
    ) -> Result<(), WriteError> {
        rw.write(Event::Start(view.borrow()))?;
        write_children(rw, prefix, children)?;
        if !opened {
            return write_end(rw, &name_of(view));
        }
        let mut buf = Vec::new();
        while let Some(event) = rw.next(&mut buf)? {
            match event {
                Event::Start(child) if removes(rw, &child, children) => {
                    through(rw, 1, false)?;
                }
                Event::Empty(child) if removes(rw, &child, children) => {}
                Event::Start(other) => {
                    rw.write(Event::Start(other))?;
                    through(rw, 1, true)?;
                }
                Event::End(end) => {
                    rw.write(Event::End(end))?;
                    break;
                }
                other => rw.write(other)?,
            }
        }
        Ok(())
    }

    /// Writes the `sheetViews` `start` (`opened`: it has an end tag) with
    /// the panes of the last `freeze_panes` call.
    ///
    /// # Errors
    ///
    /// The error of the reader or the writer.
    pub(in crate::sheet::edit) fn stream(
        &self,
        rw: &mut Rw<'_>,
        prefix: &str,
        (start, opened): (BytesStart<'static>, bool),
    ) -> Result<(), WriteError> {
        let children = self.request.as_ref().and_then(new_children);
        if !opened {
            let name = name_of(&start);
            rw.write(Event::Start(start))?;
            write_new_view(rw, prefix, &children)?;
            return write_end(rw, &name);
        }
        rw.write(Event::Start(start))?;
        let mut done = false;
        let mut buf = Vec::new();
        while let Some(event) = rw.next(&mut buf)? {
            match event {
                Event::Start(view) if !done && is_first_view(rw, &view) => {
                    done = true;
                    Self::view(rw, prefix, (&view, true), &children)?;
                }
                Event::Empty(view) if !done && is_first_view(rw, &view) => {
                    done = true;
                    Self::view(rw, prefix, (&view, false), &children)?;
                }
                Event::Start(ext) if is_ext_after_view(rw, &ext, done, &children) => {
                    done = true;
                    write_new_view(rw, prefix, &children)?;
                    rw.write(Event::Start(ext))?;
                    through(rw, 1, true)?;
                }
                Event::Empty(ext) if is_ext_after_view(rw, &ext, done, &children) => {
                    done = true;
                    write_new_view(rw, prefix, &children)?;
                    rw.write(Event::Empty(ext))?;
                }
                Event::Start(other) => {
                    rw.write(Event::Start(other))?;
                    through(rw, 1, true)?;
                }
                Event::End(end) => {
                    if !done {
                        write_new_view(rw, prefix, &children)?;
                    }
                    rw.write(Event::End(end))?;
                    break;
                }
                other => rw.write(other)?,
            }
        }
        Ok(())
    }
}

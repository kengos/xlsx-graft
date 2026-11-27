//! Writes a `mergeCells` of the part with the ranges removed left out and
//! the ranges added written before the end of the first one.

use super::Merges;
use crate::error::WriteError;
use crate::sheet::edit::apply::Rw;
use crate::sheet::edit::name::is_main;
use crate::sheet::edit::pass::through;
use crate::sheet::edit::tag::{name_of, retag, write_empty, write_end};
use xlsx_graft_xml::events::{BytesStart, Event};

impl Merges {
    /// How many `mergeCell`s the `mergeCells` has when written: the ones not
    /// removed, and those added.
    fn kept(&self) -> usize {
        let part = self.entries.iter().filter(|entry| !entry.removed);
        part.count() + self.added.len()
    }

    /// Whether `child`, a child of the `mergeCells`, is a `mergeCell` that
    /// was removed; it moves `position`, the number of `mergeCell`s before
    /// it, past any `mergeCell`.
    fn leaves_out(&self, rw: &Rw<'_>, child: &BytesStart<'_>, position: &mut usize) -> bool {
        if !is_main(&rw.resolve_element(child.name()), "mergeCell") {
            return false;
        }
        *position += 1;
        let entry = self.entries.get(*position - 1);
        entry.is_some_and(|entry| entry.removed)
    }

    /// Writes the ranges added as `mergeCell`s.
    fn write_added(&self, rw: &mut Rw<'_>, prefix: &str) -> Result<(), WriteError> {
        for range in &self.added {
            let name = format!("{prefix}mergeCell");
            write_empty(rw, &name, &[("ref", range.to_string())])?;
        }
        Ok(())
    }

    /// Writes the `mergeCells` of the part, `start` being its
    /// start tag (`opened`: it has an end tag), with its `count` set to the
    /// number written. Nothing is written for one with no `mergeCell` left.
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
        let kept = self.kept();
        if kept == 0 {
            return through(rw, usize::from(opened), false);
        }
        let counted = retag(&start, &[("count", Some(kept.to_string()))]);
        rw.write(Event::Start(counted))?;
        if !opened {
            self.write_added(rw, prefix)?;
            return write_end(rw, &name_of(&start));
        }
        let mut buf = Vec::new();
        let mut position = 0;
        while let Some(event) = rw.next(&mut buf)? {
            match event {
                Event::Start(child) if self.leaves_out(rw, &child, &mut position) => {
                    through(rw, 1, false)?;
                }
                Event::Start(child) => {
                    rw.write(Event::Start(child))?;
                    through(rw, 1, true)?;
                }
                Event::Empty(child) if self.leaves_out(rw, &child, &mut position) => {}
                Event::End(end) => {
                    self.write_added(rw, prefix)?;
                    rw.write(Event::End(end))?;
                    break;
                }
                other => rw.write(other)?,
            }
        }
        Ok(())
    }
}

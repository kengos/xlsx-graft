//! The calls that add, rename, delete and move a sheet.
//!
//! Each call checks everything first and changes the state only when every
//! check passed, so a call that returns an `Err` changes nothing.

use super::change::{Change, Deleted};
use super::state::{Origin, Scope, SheetEntry, State};
use crate::PartWriter;
use crate::structure::model::SheetState;
use crate::structure::{StructureError, check_sheet_name};

impl<'r> State<'r> {
    /// Adds a sheet named `name` at position `at` (after the last sheet when
    /// `None`), with `content` (the default empty worksheet when `None`).
    pub(super) fn add_sheet(
        &mut self,
        name: &str,
        at: Option<usize>,
        content: Option<Box<dyn PartWriter + 'r>>,
    ) -> Result<(), StructureError> {
        check_sheet_name(name, self.sheets.iter().map(|sheet| sheet.name.as_str()))?;
        let at = at.unwrap_or(self.sheets.len());
        if at > self.sheets.len() {
            return Err(StructureError::PositionOutOfRange {
                at,
                sheets: self.sheets.len(),
            });
        }
        let sheet_id = self.next_sheet_id.ok_or(StructureError::SheetIdExhausted)?;
        let name_id = self.sheet_ids.id(name);
        self.next_sheet_id = sheet_id.checked_add(1);
        let key = self.next_key;
        self.next_key = key.saturating_add(1);
        self.sheets.insert(
            at,
            SheetEntry {
                key,
                name: name.to_owned(),
                name_id,
                renamed: false,
                sheet_id,
                state: SheetState::Visible,
                origin: Origin::Added {
                    index: self.added.len(),
                },
            },
        );
        self.added.push(content);
        Ok(())
    }

    /// Renames the sheet `old` to `new`, and the sheet in each defined
    /// name's formula.
    pub(super) fn rename_sheet(&mut self, old: &str, new: &str) -> Result<(), StructureError> {
        let (position, sheet) = self.find_sheet(old)?;
        let (key, old) = (sheet.key, sheet.name.clone());
        let others = (self.sheets.iter().enumerate())
            .filter(|(index, _)| *index != position)
            .map(|(_, sheet)| sheet.name.as_str());
        check_sheet_name(new, others)?;
        let name_id = self.sheet_ids.id(new);
        (self.sheets.iter_mut())
            .filter(|sheet| sheet.key == key)
            .for_each(|sheet| {
                sheet.name = new.to_owned();
                sheet.name_id = name_id;
                sheet.renamed = true;
            });
        self.rewrite_formulas(Change::Rename {
            old: old.into(),
            new: new.to_owned(),
        });
        Ok(())
    }

    /// Deletes the sheet `name`: its formulas' prefixes, the defined names
    /// scoped to it and the views that point at it are changed.
    pub(super) fn delete_sheet(&mut self, name: &str) -> Result<(), StructureError> {
        let (position, sheet) = self.find_sheet(name)?;
        let (key, name) = (sheet.key, sheet.name.clone());
        if self.sheets.len() == 1 {
            return Err(StructureError::LastSheet { name });
        }
        let other_visible = (self.sheets.iter())
            .any(|other| other.key != key && other.state == SheetState::Visible);
        if !other_visible {
            return Err(StructureError::LastVisibleSheet { name });
        }
        let mut kept = Vec::new();
        for entry in std::mem::take(&mut self.names) {
            if entry.scope == Scope::Sheet(key) {
                self.removed_names.extend(entry.origin);
            } else {
                kept.push(entry);
            }
        }
        self.names = kept;
        let deleted = Deleted::new(&self.sheets, position, &self.sheet_ids);
        self.rewrite_formulas(Change::Delete(deleted));
        self.retarget_views(position);
        let deleted = self.sheets.remove(position);
        self.deleted.push(deleted);
        Ok(())
    }

    /// Moves the sheet `name` to position `to`.
    pub(super) fn move_sheet(&mut self, name: &str, to: usize) -> Result<(), StructureError> {
        let (position, _) = self.find_sheet(name)?;
        if to >= self.sheets.len() {
            return Err(StructureError::PositionOutOfRange {
                at: to,
                sheets: self.sheets.len(),
            });
        }
        let sheet = self.sheets.remove(position);
        self.sheets.insert(to, sheet);
        Ok(())
    }

    /// The position and entry of the sheet `name`, letter case aside, or
    /// [`StructureError::SheetMissing`].
    pub(super) fn find_sheet(&self, name: &str) -> Result<(usize, &SheetEntry), StructureError> {
        let folded = name.to_lowercase();
        (self.sheets.iter().enumerate())
            .find(|(_, sheet)| sheet.name.to_lowercase() == folded)
            .ok_or_else(|| StructureError::SheetMissing {
                name: name.to_owned(),
            })
    }
}

//! The calls that set and remove a defined name.
//!
//! A defined name is found by the spelling it is written with, letter case
//! aside, among the names of the same scope. Like the sheet calls, each
//! checks everything before it changes the state.

use super::state::{NameEntry, Scope, State, Text, escaped};
use crate::structure::model::MAX_DEFINED_NAMES;
use crate::structure::{StructureError, builtin_spelling, check_defined_name};
use xlsx_graft_formula::analyze;

impl State<'_> {
    /// Sets the defined name `name`, scoped to the sheet `scope` (the
    /// workbook when `None`), to `formula`: it replaces the name of the same
    /// scope and spelling, letter case aside, in the place it has, or else is
    /// written after the others.
    pub(super) fn set_defined_name(
        &mut self,
        name: &str,
        scope: Option<&str>,
        formula: &str,
    ) -> Result<(), StructureError> {
        check_defined_name(
            name,
            scope.is_some(),
            self.tables.iter().map(String::as_str),
        )?;
        let scope = self.scope_for(scope)?;
        let written = escaped(&builtin_spelling(name).unwrap_or_else(|| name.to_owned()));
        self.check_formula(formula)?;
        let folded = written.to_lowercase();
        let existing = (self.names.iter_mut())
            .find(|entry| entry.scope == scope && entry.written.to_lowercase() == folded);
        match existing {
            Some(entry) => {
                entry.name_changed |= entry.written != written;
                entry.written = written;
                entry.text = Text::Changed(formula.to_owned());
                entry.unresolved = None;
            }
            None => {
                let count = u64::try_from(self.names.len()).unwrap_or(u64::MAX);
                if count >= MAX_DEFINED_NAMES {
                    return Err(StructureError::DefinedNamesTooMany);
                }
                self.names.push(NameEntry {
                    written,
                    scope,
                    text: Text::Changed(formula.to_owned()),
                    origin: None,
                    name_changed: false,
                    unresolved: None,
                });
            }
        }
        Ok(())
    }

    /// Removes the defined name `name` scoped to the sheet `scope` (the
    /// workbook when `None`).
    pub(super) fn remove_defined_name(
        &mut self,
        name: &str,
        scope: Option<&str>,
    ) -> Result<(), StructureError> {
        let scope = self.scope_for(scope)?;
        let folded = escaped(name).to_lowercase();
        let index = (self.names.iter())
            .position(|entry| entry.scope == scope && entry.written.to_lowercase() == folded)
            .ok_or_else(|| StructureError::DefinedNameMissing {
                name: name.to_owned(),
            })?;
        let removed = self.names.remove(index);
        self.removed_names.extend(removed.origin);
        Ok(())
    }

    /// The scope of the sheet `sheet`, the workbook's for `None`.
    fn scope_for(&self, sheet: Option<&str>) -> Result<Scope, StructureError> {
        sheet.map_or(Ok(Scope::Book), |name| {
            self.find_sheet(name)
                .map(|(_, sheet)| Scope::Sheet(sheet.key))
        })
    }

    /// Checks that `formula` is read whole and names, of this workbook's
    /// sheets, only ones it has.
    fn check_formula(&self, formula: &str) -> Result<(), StructureError> {
        let read = analyze(&escaped(formula));
        // A definedName with no formula is not written.
        if formula.trim().is_empty() || !read.facts.malformed.is_empty() {
            return Err(StructureError::FormulaMalformed {
                formula: formula.to_owned(),
            });
        }
        let named = (read.facts.sheet_prefixes.iter())
            .filter(|prefix| prefix.book.is_none())
            .flat_map(|prefix| std::iter::once(&prefix.first).chain(prefix.last.as_ref()));
        for sheet in named {
            if self.find_sheet(sheet).is_err() {
                return Err(StructureError::FormulaSheetMissing {
                    formula: formula.to_owned(),
                    sheet: sheet.clone(),
                });
            }
        }
        Ok(())
    }
}

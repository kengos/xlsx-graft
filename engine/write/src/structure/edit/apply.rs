//! [`StructureEdit::apply`]: the state written to a [`PackageEdit`] of the
//! package `open` read.
//!
//! In this order: the sheets added become parts with a relationship from the
//! workbook part; the parts of the sheets deleted and the parts that only they
//! target through relationships are removed (`cleanup.rs`), and the calcChain
//! part with them when a sheet that the opened workbook had is deleted; the
//! formulas of the other parts are rewritten for the renames and deletes made,
//! and what is left as it was goes in the report (`parts.rs`); the app part is
//! rewritten without its sheet lists when the sheets changed (`app.rs`); and
//! the workbook part is rewritten when anything in it changed (`rewrite.rs`),
//! with `fullCalcOnLoad` when a `#REF!` was written in any part.

use super::StructureEdit;
use super::app::AppRewriter;
use super::cleanup::removals;
use super::new_sheet::NewSheet;
use super::ns::{RELATIONSHIPS, WORKSHEET_MEDIA, flavor};
use super::parts;
use super::rewrite::{Plan, WorkbookRewriter};
use super::state::Origin;
use crate::structure::StructureReport;
use crate::{Link, LinkSource, PackageEdit, PartWriter, WriteError};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek};
use xlsx_graft_package::{ContentType, PartName};

impl<'r> StructureEdit<'r> {
    /// Writes what the calls made to `edit`, an edit of the package this
    /// was opened from: the sheets added as parts, the parts of the sheets
    /// deleted left out, the formulas of the other parts rewritten for the
    /// sheets renamed and deleted, and the workbook part (and the app part)
    /// written again for what changed in them.
    ///
    /// The formulas of the places `xlsx_graft_formula::places` marks
    /// `Rewrite` get their sheet prefixes changed as a defined name's do; the
    /// parts are read only when a sheet was renamed or deleted, and a part
    /// with no formula changed is not rewritten. What it does not rewrite is
    /// kept as it is and is in the report that is returned.
    ///
    /// A sheet renamed or deleted makes `apply` replace the parts of the
    /// package that hold a formula it rewrites (worksheet, macrosheet,
    /// dialogsheet, chart, table and pivot cache definition parts), through
    /// `PackageEdit::replace`. When the caller replaces one of them too, only
    /// the replace made last stays, as `PackageEdit::replace` says.
    ///
    /// # Errors
    ///
    /// [`WriteError::GraphIncomplete`] when a sheet the opened workbook had was
    /// deleted and the package's relationships were not all read, so the parts
    /// to remove are not known: nothing was done to `edit`.
    ///
    /// [`WriteError::Io`] when reading a part of the package fails.
    ///
    /// Any other [`WriteError`] is that of the `PackageEdit` call that refused
    /// a part (`PartExists` for a name the package holds, by
    /// `PackageEdit::add`), or that of a media type or part name this edit
    /// could not make (`PartExists`). `edit` may then hold part of the
    /// changes and must be dropped, except when the media type or the first
    /// sheet's part name could not be made: like `GraphIncomplete`, that is
    /// returned before anything is changed.
    pub fn apply<R: Read + Seek>(
        self,
        edit: &mut PackageEdit<'r, R>,
    ) -> Result<StructureReport, WriteError> {
        let Self {
            snapshot,
            mut state,
        } = self;
        let roots: Vec<usize> = (state.deleted.iter())
            .filter_map(|sheet| match sheet.origin {
                Origin::Original { part, .. } => Some(part),
                Origin::Added { .. } => None,
            })
            .collect();
        if !roots.is_empty() && !snapshot.graph_complete {
            return Err(WriteError::GraphIncomplete);
        }
        let strict = snapshot.workbook.strict();
        let folder = (snapshot.workbook_name.as_str().rsplit_once('/')).map_or("", |(dir, _)| dir);
        let rel_type = format!(
            "{}/worksheet",
            RELATIONSHIPS.get(flavor(strict)).unwrap_or(&"")
        );
        let media = media(WORKSHEET_MEDIA)?;
        let mut taken = snapshot.taken.clone();
        let mut rids = BTreeMap::new();
        for entry in &state.sheets {
            let Origin::Added { index } = entry.origin else {
                continue;
            };
            let content = state.added.get_mut(index).and_then(Option::take);
            let writer: Box<dyn PartWriter + 'r> = content.unwrap_or(Box::new(NewSheet { strict }));
            let name = sheet_part(folder, &mut taken)?;
            let link = Link {
                source: LinkSource::Part(snapshot.workbook_name.clone()),
                rel_type: rel_type.clone(),
            };
            let id = edit.add(name, media.clone(), writer, Some(link))?;
            rids.insert(index, id.unwrap_or_default());
        }
        let removed = removals(&snapshot.graph, &roots, snapshot.calc_chain);
        for name in &removed {
            edit.remove(name)?;
        }
        let changes = std::mem::take(&mut state.changes);
        let ids = std::mem::take(&mut state.sheet_ids);
        let found = parts::rewrite(edit, &snapshot, &state, (changes, ids), &removed)?;
        state.ref_written |= found.ref_written;
        let plan = Plan::build(&state, &snapshot.workbook, &rids);
        let titles = (snapshot.app.as_ref()).filter(|app| plan.sheets_changed() && app.has_titles);
        if let Some(app) = titles {
            let limits = snapshot.limits;
            edit.replace(&app.name, Box::new(AppRewriter { limits }))?;
        }
        if plan.changed() {
            let limits = snapshot.limits;
            let rewriter = WorkbookRewriter { plan, limits };
            edit.replace(&snapshot.workbook_name, Box::new(rewriter))?;
        }
        Ok(StructureReport::new(found.kept, found.omitted))
    }
}

/// The name of a new sheet part: `<folder>/worksheets/sheet<N>.xml`, `N` the
/// least positive number whose name is free: not in `taken` (as the key of a
/// name), its Relationships part's name not in `taken`, and no name in `taken`
/// below it as a folder (`sheet<N>.xml/...`). The name is added to `taken`.
fn sheet_part(folder: &str, taken: &mut BTreeSet<String>) -> Result<PartName, WriteError> {
    // Each name in `taken` rules out at most one number (the one whose part
    // name, Relationships part name or folder it is), so one of these is free.
    // The workbook part's own name in `taken` rules out no number (it is not
    // under `worksheets/`), so the bound has one candidate to spare.
    let last = taken.len().saturating_add(1);
    let mut found = None;
    for number in 1..=last {
        let name = part_name(&format!("{folder}/worksheets/sheet{number}.xml"))?;
        let key = name.key().to_owned();
        let folder_prefix = format!("{key}/");
        let in_folder = (taken.range(folder_prefix.clone()..).next())
            .is_some_and(|other| other.starts_with(&folder_prefix));
        let used =
            taken.contains(&key) || taken.contains(name.relationships_part().key()) || in_folder;
        if !used {
            taken.insert(key);
            found = Some(name);
            break;
        }
    }
    found.ok_or(WriteError::PartExists {
        name: folder.to_owned(),
    })
}

/// `value` as a media type.
///
/// # Errors
///
/// [`WriteError::PartExists`] naming `value`, when it is not one: the part
/// cannot be added under a type the package cannot hold.
pub(super) fn media(value: &str) -> Result<ContentType, WriteError> {
    let refused = || WriteError::PartExists {
        name: value.to_owned(),
    };
    ContentType::parse(value).ok_or_else(refused)
}

/// `path` as a part name.
///
/// # Errors
///
/// [`WriteError::PartExists`] naming `path`, when it is not one.
pub(super) fn part_name(path: &str) -> Result<PartName, WriteError> {
    let refused = |_| WriteError::PartExists {
        name: path.to_owned(),
    };
    PartName::parse(path).map_err(refused)
}

//! [`Book`]: a workbook opened to read its sheets' rows, as the crate root
//! says: its package, its workbook part, shared strings table and styles,
//! each read once, and a [`SheetRows`] for a worksheet's rows.

use crate::schema::fold;
use crate::{
    DateSystem, ReadError, ReadLimits, SharedStrings, Sheet, SheetKind, SheetRows, Styles,
    Workbook, WorkbookPart,
};
use std::cell::RefCell;
use std::io::{Read, Seek};
use std::rc::Rc;
use xlsx_graft_diagnostics::{Diagnostics, Location};
use xlsx_graft_package::{Limits, Package};
use xlsx_graft_spool::SpoolFactory;

/// What a sheet name finds in a [`Book`] ([`Book::sheet`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetLookup {
    /// The one sheet of the name, at this index of
    /// [`sheets`](Book::sheets).
    Found(usize),
    /// No sheet has the name.
    Missing,
    /// More than one sheet has the name.
    Duplicate,
}

/// A workbook read from `R`, to read its sheets' rows from.
///
/// It shares its package, shared strings table and styles with each
/// [`SheetRows`] it makes, which needs no borrow of it: a caller may keep
/// both side by side, drop the book first, or read two sheets in turns.
/// Neither is [`Send`].
pub struct Book<R> {
    package: Rc<RefCell<Package<R>>>,
    strings: Rc<RefCell<SharedStrings>>,
    styles: Rc<Styles>,
    workbook: WorkbookPart,
    /// The limits each sheet is read under.
    limits: ReadLimits,
}

impl<R: Read + Seek> Book<R> {
    /// Opens the workbook in `reader`: its package under `package_limits`,
    /// then its workbook part, shared strings table and styles under
    /// `limits`, the shared strings spilling to a spool from `spools`;
    /// pushing each problem found. `None` when there is no workbook to read
    /// rows from: the package, or its workbook part, cannot be read or is
    /// refused, which the diagnostics say.
    ///
    /// # Errors
    ///
    /// [`ReadError::Package`] when reading the package fails,
    /// [`ReadError::Spool`] when the spool does.
    pub fn open(
        reader: R,
        package_limits: &Limits,
        limits: &ReadLimits,
        spools: &mut dyn SpoolFactory,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<Self>, ReadError> {
        let Some(mut package) = Package::open(reader, package_limits, diagnostics)? else {
            return Ok(None);
        };
        let Some(workbook) = Workbook::find(&package, diagnostics) else {
            return Ok(None);
        };
        let Some(part) = WorkbookPart::read(&mut package, &workbook, limits, diagnostics)? else {
            return Ok(None);
        };
        let strings = SharedStrings::read(&mut package, &workbook, limits, spools, diagnostics)?;
        let styles = Styles::read(&mut package, &workbook, limits, diagnostics)?;
        Ok(Some(Self {
            package: Rc::new(RefCell::new(package)),
            strings: Rc::new(RefCell::new(strings)),
            styles: Rc::new(styles),
            workbook: part,
            limits: *limits,
        }))
    }

    /// The sheets, in the order the workbook part lists them.
    #[must_use]
    pub fn sheets(&self) -> &[Sheet] {
        self.workbook.sheets()
    }

    /// How the workbook counts days.
    #[must_use]
    pub fn date_system(&self) -> DateSystem {
        self.workbook.date_system()
    }

    /// The sheet named `name`, compared as sheet names are, letter case
    /// aside: its index in [`sheets`](Self::sheets), or that no sheet has
    /// the name, or that more than one has, which `sheet_name_duplicate`
    /// reported: readers would differ on which one it names.
    #[must_use]
    pub fn sheet(&self, name: &str) -> SheetLookup {
        let name = fold(name);
        let mut found = (self.sheets().iter().enumerate())
            .filter(|(_, sheet)| fold(sheet.name()) == name)
            .map(|(index, _)| index);
        match (found.next(), found.next()) {
            (None, _) => SheetLookup::Missing,
            (Some(index), None) => SheetLookup::Found(index),
            (Some(_), Some(_)) => SheetLookup::Duplicate,
        }
    }

    /// A reader of the rows of the sheet at `index` of
    /// [`sheets`](Self::sheets). `None` when there is no such sheet, it is
    /// not a worksheet (a chartsheet or a dialogsheet holds no cells, and a
    /// macro sheet's are not read), or its part is not known or cannot be
    /// read, which the diagnostics say already.
    #[must_use]
    pub fn rows(&self, index: usize) -> Option<SheetRows<R>> {
        let part = self.sheets().get(index)?.part()?;
        if part.kind() != SheetKind::Worksheet {
            return None;
        }
        let package = self.package.try_borrow().ok()?;
        let stream = package.stream_part(part.index())?;
        let location = (package.parts().get(part.index()))
            .map(|part| Location::default().with_part(part.name().as_str()))
            .unwrap_or_default();
        drop(package);
        Some(SheetRows::new(
            Rc::clone(&self.package),
            stream,
            Rc::clone(&self.strings),
            Rc::clone(&self.styles),
            self.date_system(),
            &self.limits,
            location,
        ))
    }
}

//! The edits `layout` makes to the first sheet of each workbook.

use crate::repack::Refusal;
use std::fs::File;
use xlsx_graft_diagnostics::{Diagnostics, Limit};
use xlsx_graft_package::{Package, PartName, Presence};
use xlsx_graft_write::sheet::{CellRange, PasswordHash, SheetEdit, SheetProtection};
use xlsx_graft_xml::events::{BytesStart, Event};
use xlsx_graft_xml::{XmlLimits, XmlReader, XmlVersion};

/// The password of the protection.
pub(super) const PASSWORD: &str = "xlsx-graft";

/// The salt of the protection's hash, fixed so that the same workbook gives
/// the same bytes: 0 to 15.
pub(super) const SALT: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

/// The spin count of the protection's hash.
pub(super) const SPIN_COUNT: u32 = 100_000;

/// The range merged, then the one merged when the first is refused (the
/// sheet merges or has a table over a cell of it), far from the cells and
/// tables the workbooks of `fixtures/` have.
pub(super) const MERGES: [&str; 2] = ["AA1000:AB1001", "AD2000:AE2001"];

/// The column set 30 wide when the sheet has no `col` that covers two
/// columns or more: F.
pub(super) const DEFAULT_COLUMN: u16 = 6;

/// The middle column of `start`, a `col`, when it covers two columns or more
/// (`min` to `max` in columns 1 to 16,384): the first of the two in the
/// middle for an even number of columns.
fn middle(start: &BytesStart<'_>) -> Option<u16> {
    let number = |key: &str| {
        let found = start.attributes().flatten().find(|a| a.key.as_ref() == key);
        let value = found?.normalized_value(XmlVersion::Implicit1_0).ok()?;
        value.trim().parse::<u16>().ok()
    };
    let (min, max) = (number("min")?, number("max")?);
    ((1..max).contains(&min) && max <= 16_384).then(|| min + (max - min) / 2)
}

/// The middle column of the first `col` of the worksheet `bytes` that covers
/// two columns or more; `None` when there is none or the XML cannot be read.
/// An element is a `col` by its local name alone, whatever its prefix and
/// namespace.
fn first_middle(bytes: &[u8]) -> Option<u16> {
    let mut reader = XmlReader::open(bytes, &XmlLimits::default()).ok()?;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let Ok(event) = reader.read_event_into(&mut buf) else { break };
        if let Event::Start(start) | Event::Empty(start) = &event
            && reader.resolve_element(start.name()).1.as_ref() == "col"
            && let Some(middle) = middle(start)
        {
            return Some(middle);
        }
        if matches!(event, Event::Eof) {
            break;
        }
    }
    None
}

/// The column to set 30 wide in the sheet whose part is `part`: the middle
/// column of its first `col` that covers two columns or more, so that `col`
/// is cut by the width; [`DEFAULT_COLUMN`] when it has none or the part
/// cannot be read.
pub(super) fn wide_column(package: &mut Package<File>, part: &PartName) -> u16 {
    let Presence::Part(index) = package.find(part) else { return DEFAULT_COLUMN };
    let limit = Limit::default_of(XmlLimits::DEFAULT_MAX_BYTES);
    let bytes = (package.read_xml_part(index, limit, &mut Diagnostics::new())).ok();
    let bytes = bytes.flatten().unwrap_or_default();
    first_middle(&bytes).unwrap_or(DEFAULT_COLUMN)
}

/// Makes the edits on `edit`, the first sheet's, with the width 30 on the
/// column `wide` (see [`wide_column`]). The notes say which merge
/// was made when the first range was refused, or that none was: a range that
/// overlaps another range or a table of the sheet is the edit's refusal, not
/// the workbook's.
///
/// # Errors
///
/// A [`Refusal`] for the error of a call other than the merge, or of a
/// range that is not one, which the values this module passes do not give.
pub(super) fn edit(edit: &mut SheetEdit, wide: u16) -> Result<Vec<String>, Refusal> {
    let mut notes = Vec::new();
    let mut refused = Vec::new();
    let mut merged = false;
    for text in MERGES {
        match edit.merge(CellRange::parse(text)?) {
            Ok(()) => {
                if !refused.is_empty() {
                    notes.push(format!("merged {text}, not {}", refused.join(", ")));
                }
                merged = true;
                break;
            }
            Err(error) => refused.push(format!("{text} ({error})")),
        }
    }
    if !merged {
        notes.push(format!("merged no range: {}", refused.join(", ")));
    }
    edit.set_column_width(2, 4, Some(20.0))?;
    edit.set_column_width(wide, wide, Some(30.0))?;
    edit.set_row_height(1, Some(30.0))?;
    edit.set_row_height(500, Some(24.0))?;
    edit.freeze_panes(1, 1)?;
    edit.protect(SheetProtection {
        password: Some(PasswordHash::sha512(PASSWORD, SALT, SPIN_COUNT)?),
        ..SheetProtection::default()
    });
    Ok(notes)
}

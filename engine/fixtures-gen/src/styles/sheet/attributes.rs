//! The attributes of the row and cell elements `SheetCells` reads and
//! rewrites.

use super::refuse;
use xlsx_graft_cell::CellRef;
use xlsx_graft_write::WriteError;
use xlsx_graft_xml::events::BytesStart;

/// The value of the attribute `key` of `start`, which the element must have.
fn attribute(start: &BytesStart<'_>, key: &str) -> Result<String, WriteError> {
    let found = start.try_get_attribute(key).map_err(refuse)?;
    let value = found.map(|attribute| attribute.value.into_owned());
    value.ok_or_else(|| {
        let name = start.name().as_ref().to_owned();
        refuse(format!("a <{name}> has no {key} attribute"))
    })
}

/// The number a `<row>`'s `r` gives.
pub(super) fn row_number(row: &BytesStart<'_>) -> Result<u32, WriteError> {
    attribute(row, "r")?.parse().map_err(refuse)
}

/// The column a `<c>`'s `r`, a cell reference, gives: 1 for A.
pub(super) fn column(cell: &BytesStart<'_>) -> Result<u16, WriteError> {
    let at = CellRef::parse(&attribute(cell, "r")?).map_err(refuse)?;
    Ok(at.column())
}

/// `start` with its attribute `drop` removed, and the attribute `s` with the
/// value `style` added at the end if there is one. The other attributes are
/// as they were, in their order.
pub(super) fn rebuilt(
    start: &BytesStart<'_>,
    drop: &str,
    style: Option<u32>,
) -> Result<BytesStart<'static>, WriteError> {
    let mut out = BytesStart::new(start.name().as_ref().to_owned());
    for attribute in start.attributes() {
        let attribute = attribute.map_err(refuse)?;
        if attribute.key.as_ref() != drop {
            out.push_attribute(attribute);
        }
    }
    if let Some(style) = style {
        out.push_attribute(("s", style.to_string().as_str()));
    }
    Ok(out)
}

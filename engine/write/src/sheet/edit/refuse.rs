//! The refusal of a part whose edited element cannot be placed: one that
//! comes after the place its kind was written (the part's children are out
//! of the schema's order) or that is wrapped in `mc:AlternateContent`.

use crate::error::WriteError;
use crate::sheet::WorksheetChild;
use xlsx_graft_diagnostics::{Diagnostic, Echo, SchemaReason};

/// The error that refuses the part for the element of `kind` that starts at
/// `offset` (the element, or the wrapper that holds it):
/// `xml_schema_invalid`, `unexpected_element`, naming the kind.
pub(super) fn refuse(offset: u64, kind: WorksheetChild) -> WriteError {
    WriteError::Invalid(Diagnostic::XmlSchemaInvalid {
        offset,
        reason: SchemaReason::UnexpectedElement,
        name: Echo::new(kind.local_name()),
    })
}

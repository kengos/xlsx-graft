//! What stops a part being read as XML as `scan.rs` reads it: what it
//! reports for every part it reads ([`stops_reading`]), and what it says
//! of a part only that it is not XML ([`not_xml`]).

use xlsx_graft_diagnostics::{Diagnostic, EncodingReason};

/// Whether `diagnostic`, found as another reader read a part, is one the
/// stream `scan.rs` reads every part with reports again when it reads
/// that part: what stops the XML reader, which the stream stops at alike.
///
/// So is the workbook and styles readers' `xml_schema_invalid`, which is
/// not reported, though no one reports it again: their schema holds a
/// part to more than Excel does, so a part they refuse may be one Excel
/// opens as it is, and only they do not believe it.
pub(crate) fn stops_reading(diagnostic: &Diagnostic) -> bool {
    matches!(
        diagnostic,
        Diagnostic::XmlMalformed { .. }
            | Diagnostic::XmlDoctype { .. }
            | Diagnostic::XmlEncodingUnsupported { .. }
            | Diagnostic::XmlTooDeep { .. }
            | Diagnostic::XmlTooManyAttributes { .. }
            | Diagnostic::XmlTooManyNamespaces { .. }
            | Diagnostic::XmlNamespaceTooLong { .. }
            | Diagnostic::XmlEventTooLarge { .. }
            | Diagnostic::XmlSchemaInvalid { .. }
    )
}

/// Whether `diagnostic`, stopping a part's reading, says only that the
/// part is not XML.
pub(super) fn not_xml(diagnostic: &Diagnostic) -> bool {
    match diagnostic {
        Diagnostic::XmlMalformed { .. } => true,
        // Bytes sniffed as UTF-32 are not XML: an EMF image starts
        // `01 00 00 00`. A declaration naming UTF-32 is silenced with them;
        // Excel reads no such part either.
        Diagnostic::XmlEncodingUnsupported {
            encoding,
            reason: EncodingReason::OtherEncoding,
        } => encoding.as_str().eq_ignore_ascii_case("UTF-32"),
        // A declaration that names an encoding XML does not use, or one the
        // bytes are not in, was read as XML; bytes that do not decode were not.
        Diagnostic::XmlEncodingUnsupported { reason, .. } => {
            *reason != EncodingReason::DeclarationMismatch
        }
        _ => false,
    }
}

//! The schema shape of the shared strings part: the elements read
//! ([`Frame`]) and where each may stand ([`child`]), and the attributes
//! each may have ([`attributes`]).
//!
//! Each frame knows how far into its schema sequence it is (its stage), so
//! a child is admitted only in the schema's order and as often as it may
//! come.

use crate::compat::Ignorable;
use crate::schema::{self, namespace};
use crate::shared_strings::Halt;
use std::io::BufRead;
use xlsx_graft_diagnostics::SchemaReason;
use xlsx_graft_xml::XmlStreamReader;
use xlsx_graft_xml::events::BytesStart;

/// An element being read, and its stage in its schema sequence.
#[derive(Debug, Clone, Copy)]
pub(super) enum Frame {
    /// `sst` (CT_Sst): `si*`, then `extLst?`.
    Sst { stage: u8 },
    /// `si` (CT_Rst): `t?`, `r*`, `rPh*`, then `phoneticPr?`.
    Si { stage: u8 },
    /// `r` (CT_RElt): `rPr?`, then `t`; `offset` is where it starts.
    Run { stage: u8, offset: u64 },
    /// `t` (ST_Xstring).
    Text,
}

/// What a child element is to its parent.
pub(super) enum Child {
    /// An element to read.
    Read(Frame),
    /// An element to skip, with all it holds.
    Skip,
}

/// What the element `local` at `offset` is as a child of `parent`, moving
/// `parent` on to its stage; `None` when it may not stand there.
pub(super) fn child(parent: &mut Frame, local: &str, offset: u64) -> Option<Child> {
    // The child's stage in the sequence, whether it may repeat, and what it
    // is.
    let (at, stage, repeats, child) = match (parent, local) {
        (Frame::Sst { stage: at }, "si") => (at, 1, true, Child::Read(Frame::Si { stage: 0 })),
        (Frame::Sst { stage: at }, "extLst") => (at, 2, false, Child::Skip),
        (Frame::Si { stage: at }, "t") => (at, 1, false, Child::Read(Frame::Text)),
        (Frame::Si { stage: at }, "r") => {
            (at, 2, true, Child::Read(Frame::Run { stage: 0, offset }))
        }
        (Frame::Si { stage: at }, "rPh") => (at, 3, true, Child::Skip),
        (Frame::Si { stage: at }, "phoneticPr") => (at, 4, false, Child::Skip),
        (Frame::Run { stage: at, .. }, "rPr") => (at, 1, false, Child::Skip),
        (Frame::Run { stage: at, .. }, "t") => (at, 2, false, Child::Read(Frame::Text)),
        _ => return None,
    };
    let admitted = stage > *at || (stage == *at && repeats);
    *at = stage;
    admitted.then_some(child)
}

/// Reads the attributes of `start`, an element read as `frame` at
/// `offset`, entered in `ignorable`: names in it what its `mc:Ignorable`
/// names, passes over those in them, and refuses one the schema does not
/// give it. Namespace declarations are not attributes.
pub(super) fn attributes<B: BufRead>(
    frame: &Frame,
    start: &BytesStart<'_>,
    reader: &XmlStreamReader<B>,
    ignorable: &mut Ignorable,
    offset: u64,
) -> Result<(), Halt> {
    let allowed: &[&str] = match frame {
        Frame::Sst { .. } => &["count", "uniqueCount"],
        Frame::Text => &["xml:space"],
        Frame::Si { .. } | Frame::Run { .. } => &[],
    };
    ignorable
        .take(start, reader, offset)
        .map_err(Halt::Refused)?;
    for attribute in start.attributes().flatten() {
        let key = attribute.key.as_ref();
        if key == "xmlns" || key.starts_with("xmlns:") || allowed.contains(&key) {
            continue;
        }
        let (resolved, local) = reader.resolve_attribute(attribute.key);
        if !ignorable.passes_over(namespace(&resolved), local.as_ref()) {
            return Err(refuse(offset, SchemaReason::UnexpectedAttribute, key));
        }
    }
    Ok(())
}

/// `xml_schema_invalid` at `offset`, for `reason`, naming `name`.
pub(super) fn refuse(offset: u64, reason: SchemaReason, name: &str) -> Halt {
    Halt::Refused(schema::refuse(offset, reason, name))
}

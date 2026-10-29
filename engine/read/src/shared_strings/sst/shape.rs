//! The schema shape of the shared strings part: the elements read
//! ([`Frame`]) and where each may stand ([`child`]), and the attributes
//! each may have ([`attributes`]). Each `si` is a rich text string, whose
//! shape is `rst.rs`'s.
//!
//! Each frame knows how far into its schema sequence it is (its stage), so
//! a child is admitted only in the schema's order and as often as it may
//! come.

use crate::compat::Ignorable;
use crate::schema::{self, namespace};
use crate::shared_strings::{Halt, rst};
use std::io::BufRead;
use xlsx_graft_diagnostics::SchemaReason;
use xlsx_graft_xml::XmlStreamReader;
use xlsx_graft_xml::events::BytesStart;

/// An element being read, and its stage in its schema sequence.
#[derive(Debug, Clone, Copy)]
pub(super) enum Frame {
    /// `sst` (CT_Sst): `si*`, then `extLst?`.
    Sst { stage: u8 },
    /// `si`, or an element in it.
    Rst(rst::Frame),
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
    let at = match parent {
        Frame::Rst(frame) => {
            return rst::child(frame, local, offset).map(|child| match child {
                rst::Child::Read(frame) => Child::Read(Frame::Rst(frame)),
                rst::Child::Skip => Child::Skip,
            });
        }
        Frame::Sst { stage } => stage,
    };
    // The child's stage in the sequence, whether it may repeat, and what it
    // is.
    let (stage, repeats, child) = match local {
        "si" => (
            1,
            true,
            Child::Read(Frame::Rst(rst::Frame::Rst { stage: 0 })),
        ),
        "extLst" => (2, false, Child::Skip),
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
        Frame::Rst(frame) => rst::allowed(frame),
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

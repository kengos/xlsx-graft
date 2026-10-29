//! The schema shape of a rich text string (CT_Rst): a shared string's `si`
//! (ECMA-376-1:2016 §18.4.8) and a cell's inline string `is` (§18.3.1.53)
//! alike. The elements read ([`Frame`]), where each may stand
//! ([`child`]), the attributes each may have ([`allowed`]) and the child
//! one may not end without ([`missing`]).
//!
//! Its value is its `t`, then the `t` of each run `r`; the phonetic runs
//! (`rPh`), their properties (`phoneticPr`) and a run's properties (`rPr`)
//! are not part of it, and are skipped.

/// An element of a rich text string being read, and its stage in its
/// schema sequence.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Frame {
    /// The string itself, `si` or `is` (CT_Rst): `t?`, `r*`, `rPh*`, then
    /// `phoneticPr?`.
    Rst { stage: u8 },
    /// `r` (CT_RElt): `rPr?`, then `t`; `offset` is where it starts.
    Run { stage: u8, offset: u64 },
    /// `t` (ST_Xstring), whose text is the value.
    Text,
}

/// What a child element is to its parent.
pub(crate) enum Child {
    /// An element to read.
    Read(Frame),
    /// An element to skip, with all it holds.
    Skip,
}

/// What the element `local` at `offset` is as a child of `parent`, moving
/// `parent` on to its stage; `None` when it may not stand there.
pub(crate) fn child(parent: &mut Frame, local: &str, offset: u64) -> Option<Child> {
    // The child's stage in the sequence, whether it may repeat, and what it
    // is.
    let (at, stage, repeats, child) = match (parent, local) {
        (Frame::Rst { stage: at }, "t") => (at, 1, false, Child::Read(Frame::Text)),
        (Frame::Rst { stage: at }, "r") => {
            (at, 2, true, Child::Read(Frame::Run { stage: 0, offset }))
        }
        (Frame::Rst { stage: at }, "rPh") => (at, 3, true, Child::Skip),
        (Frame::Rst { stage: at }, "phoneticPr") => (at, 4, false, Child::Skip),
        (Frame::Run { stage: at, .. }, "rPr") => (at, 1, false, Child::Skip),
        (Frame::Run { stage: at, .. }, "t") => (at, 2, false, Child::Read(Frame::Text)),
        _ => return None,
    };
    let admitted = stage > *at || (stage == *at && repeats);
    *at = stage;
    admitted.then_some(child)
}

/// The attributes `frame` may have besides namespace declarations and
/// those markup compatibility passes over.
pub(crate) fn allowed(frame: &Frame) -> &'static [&'static str] {
    match frame {
        Frame::Text => &["xml:space"],
        Frame::Rst { .. } | Frame::Run { .. } => &[],
    }
}

/// The child `frame`, ending, has not had and must: where it starts and
/// the child's name.
pub(crate) fn missing(frame: &Frame) -> Option<(u64, &'static str)> {
    match *frame {
        Frame::Run { stage, offset } if stage < 2 => Some((offset, "t")),
        _ => None,
    }
}

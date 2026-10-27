//! [`resolve`]: an internal relationship's `Target`, a relative reference,
//! resolved against its source part's name to a part name
//! (ECMA-376-2:2021 §6.4, RFC 3986 §5.2).

use crate::PartName;
use crate::part_name::decode_unreserved;
use xlsx_graft_diagnostics::PartNameReason;

/// A target resolved to a part name.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Resolved {
    /// The part name it names.
    pub(crate) name: PartName,
    /// Whether a `..` climbed above the root and was dropped (§6.4.3.5).
    pub(crate) above_root: bool,
}

/// Resolves `target` against `base`: the source part's name, or `/` for
/// the package's own relationships.
///
/// The fragment and the query are dropped: they do not narrow a reference
/// to a part (§6.3.2; 4th edition M1.32). A reference with a scheme or an
/// authority is not relative, and an empty one names its base.
pub(crate) fn resolve(base: &str, target: &str) -> Result<Resolved, PartNameReason> {
    let reference = target.split(['#', '?']).next().unwrap_or_default();
    if reference.starts_with("//") {
        return Err(PartNameReason::HasAuthority);
    }
    if reference
        .split('/')
        .next()
        .is_some_and(|first| first.contains(':'))
    {
        return Err(PartNameReason::HasScheme);
    }
    let path = if reference.is_empty() {
        base.to_owned()
    } else if reference.starts_with('/') {
        reference.to_owned()
    } else {
        let folder = base.rsplit_once('/').map_or("", |(folder, _)| folder);
        format!("{folder}/{reference}")
    };
    // Normalized first (RFC 3986 §6.2.2.2), so `%2E%2E` is a `..`.
    let (path, above_root) = remove_dot_segments(&decode_unreserved(&path));
    let name = PartName::parse(&path)?;
    Ok(Resolved { name, above_root })
}

/// RFC 3986 §5.2.4 on an absolute path: `.` segments go, a `..` takes the
/// segment before it, and one with none before it is dropped, which the
/// second value tells. A path ending in `.` or `..` keeps its final `/`.
fn remove_dot_segments(path: &str) -> (String, bool) {
    let segments: Vec<&str> = path.split('/').skip(1).collect();
    let mut out: Vec<&str> = Vec::with_capacity(segments.len());
    let mut above_root = false;
    for segment in &segments {
        match *segment {
            "." => {}
            ".." => above_root |= out.pop().is_none(),
            segment => out.push(segment),
        }
    }
    if segments
        .last()
        .is_some_and(|last| matches!(*last, "." | ".."))
    {
        out.push("");
    }
    (format!("/{}", out.join("/")), above_root)
}

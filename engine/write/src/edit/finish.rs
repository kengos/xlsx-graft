//! [`PackageEdit::finish`]: the zip written entry by entry from the plan.

use super::PackageEdit;
use super::plan::Plan;
use super::{content_types, relationships};
use crate::error::WriteError;
use crate::limits::WriteLimits;
use crate::zip::ZipWriter;
use context::Context;
use std::collections::BTreeMap;
use std::io::{BufRead, Read, Seek, Write};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Location};
use xlsx_graft_package::Package;
use xlsx_graft_spool::SpoolFactory;

mod context;

/// What writes the content of a new entry.
type NewContent<'a> = Box<dyn FnOnce(&mut dyn Write) -> Result<(), WriteError> + 'a>;

impl<'r, R: Read + Seek> PackageEdit<'r, R> {
    /// Writes the edited package as a zip to `out`.
    ///
    /// `Ok(Some(out))` when the zip is written, after the central directory.
    /// `Ok(None)` when it is not, for a problem of the workbook, which is
    /// a diagnostic: a zip entry of the original that cannot be read
    /// (`write_entry_unreadable`, after the diagnostic of the entry), or
    /// an entry found damaged while it is copied or read, or content that a
    /// [`PartRewriter`](crate::PartRewriter) or a
    /// [`PartWriter`](crate::PartWriter) refuses ([`WriteError::Invalid`]), or
    /// XML that is too large or malformed
    /// in the content types stream or a Relationships part the edit changes
    /// (pushed at the part, or at the entry for the content types stream, as
    /// the package does). When the edit adds or removes a part and the
    /// package has no content types stream it could read whole, nothing is
    /// written and `finish` pushes no diagnostic of its own. The reason is
    /// the diagnostic the package pushed when it was read, which depends on
    /// the case: `content_types_missing` when there is no stream; a
    /// duplicate-name or `entry_name_invalid` diagnostic of the entry when
    /// there are two or the entry is refused for its name; an XML limit or
    /// well-formedness diagnostic of the stream; the schema diagnostic of a
    /// mapping that names nothing. A host that wants to show
    /// the reason keeps the `Diagnostics` it passed to `Package::open`. A part
    /// added would have no media type that could be known.
    ///
    /// After `Ok(None)` or an error, `out` holds the bytes written so far,
    /// which are not a zip: a host that passed a pipe or a socket may have
    /// received part of one, and a host that writes to a path uses
    /// [`write_file_atomically`](crate::write_file_atomically).
    ///
    /// Each part is read and written one at a time: a part rewritten or
    /// added is deflated into memory until it passes
    /// [`WriteLimits::part_spill_bytes`], then into a spool from `spools`.
    /// A part copied is not held at all.
    ///
    /// # Errors
    ///
    /// A [`WriteError`] for a failure of the host (`out`, the spool, the
    /// reader of the original), or one a rewriter returns that is not a
    /// problem of the workbook.
    pub fn finish<W: Write>(
        self,
        out: W,
        spools: &mut dyn SpoolFactory,
        limits: &WriteLimits,
        diagnostics: &mut Diagnostics,
    ) -> Result<Option<W>, WriteError> {
        let plan = Plan::build(&self);
        if refuse_unreadable(&self.package, &plan, diagnostics) {
            return Ok(None);
        }
        // The content types stream is rewritten only when the edit changes it
        // and the package read it and every mapping in it says what it names.
        // With no such stream no part added has a media type, and the package
        // has reported why already.
        let changes = plan.changes_content_types();
        let content_types = self.package.content_types_entry();
        if changes && content_types.is_none() {
            return Ok(None);
        }
        let content_types = changes.then_some(content_types).flatten();
        let mut parts = BTreeMap::new();
        for part in self.package.parts() {
            parts.insert(part.entry(), part.name().to_string());
        }
        let Self {
            package,
            mut replaced,
            added,
            ..
        } = self;
        let mut archive = package.into_archive();
        let mut zip = ZipWriter::new(out);
        let mut context = Context {
            spools,
            limits,
            diagnostics,
            parts,
        };
        for index in 0..archive.entries().len() {
            let written = if plan.removed.contains(&index) {
                true
            } else if Some(index) == content_types {
                let (drop, add) = (&plan.drop_overrides, &plan.add_overrides);
                let rewrite = |original: &mut dyn BufRead, out: &mut dyn Write| {
                    content_types::rewrite(original, out, &limits.xml, drop, add)
                };
                context.rewritten(&mut zip, &mut archive, index, rewrite)?
            } else if let Some(change) = plan.rels.get(&index) {
                let rewrite = |original: &mut dyn BufRead, out: &mut dyn Write| {
                    relationships::rewrite(original, out, &limits.xml, &change.drop, &change.add)
                };
                context.rewritten(&mut zip, &mut archive, index, rewrite)?
            } else if let Some(mut rewriter) = replaced.remove(&index) {
                let rewrite = |original: &mut dyn BufRead, out: &mut dyn Write| {
                    rewriter.rewrite(original, out)
                };
                context.rewritten(&mut zip, &mut archive, index, rewrite)?
            } else {
                zip.copy_entry(&mut archive, index, context.diagnostics)?
            };
            if !written {
                return Ok(None);
            }
        }
        // The parts added, then the Relationships parts made for a source that
        // had none. A new Relationships part is the empty root of
        // `relationships::EMPTY` with the relationships added to it, so the
        // XML reader reads nothing but that root.
        let mut new: Vec<(String, NewContent<'_>)> = Vec::new();
        for mut part in added {
            let write = move |out: &mut dyn Write| part.writer.write(out);
            new.push((part.name.to_string(), Box::new(write)));
        }
        for (name, add) in &plan.new_rels {
            let write = |out: &mut dyn Write| relationships::create(out, &limits.xml, add);
            new.push((name.to_string(), Box::new(write)));
        }
        for (name, write) in new {
            if !context.added(&mut zip, &name, write)? {
                return Ok(None);
            }
        }
        zip.finish().map(Some)
    }
}

/// Pushes `write_entry_unreadable` for each entry of `package` that cannot
/// be read and is not left out. Returns whether there was one.
fn refuse_unreadable<R: Read + Seek>(
    package: &Package<R>,
    plan: &Plan,
    diagnostics: &mut Diagnostics,
) -> bool {
    let mut refused = false;
    for (index, entry) in package.archive().entries().iter().enumerate() {
        if !entry.is_readable() && !plan.removed.contains(&index) {
            let location = Location::default().with_entry_bytes(entry.name());
            diagnostics.push(Diagnostic::WriteEntryUnreadable {}, location);
            refused = true;
        }
    }
    refused
}

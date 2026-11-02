//! Which sheet `read` reads: the one `--sheet` names, else the one at
//! `--sheet-index`, else the first, whatever its kind. Choosing it, and
//! why none is read ([`Refusal`]), are the read crate's, so every host
//! refuses alike.

use super::ReadArgs;
use xlsx_graft_read::SheetChoice;

/// The sheet `args` asks for.
pub(super) fn choice(args: &ReadArgs) -> SheetChoice<'_> {
    match args.sheet.as_deref() {
        Some(name) => SheetChoice::Name(name),
        None => SheetChoice::Index(args.sheet_index.unwrap_or(0)),
    }
}

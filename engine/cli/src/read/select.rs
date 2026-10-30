//! Which sheet `read` reads: the one `--sheet` names, else the one at
//! `--sheet-index`, else the first, whatever its kind. Choosing it, and
//! why none is read ([`Refusal`]), are the read crate's, so every host
//! refuses alike; the JSON a refusal prints as is here.

use super::ReadArgs;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use xlsx_graft_read::{Refusal, SheetChoice};

/// The sheet `args` asks for.
pub(super) fn choice(args: &ReadArgs) -> SheetChoice<'_> {
    match args.sheet.as_deref() {
        Some(name) => SheetChoice::Name(name),
        None => SheetChoice::Index(args.sheet_index.unwrap_or(0)),
    }
}

/// A [`Refusal`] as the end line prints it: `{"code": ..., "message": ...}`.
#[derive(Debug)]
pub(super) struct Refused(pub(super) Refusal);

impl Serialize for Refused {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("code", self.0.code())?;
        map.serialize_entry("message", &self.0.to_string())?;
        map.end()
    }
}

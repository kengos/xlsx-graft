//! [`ContentTypes`]: the media types the content types stream
//! (`[Content_Types].xml`, ECMA-376-2:2021 §7.2.3) gives the parts.
//!
//! A `Default` maps an extension to a media type and an `Override` a part
//! name; an `Override` takes precedence (§7.2.3.5). Both are matched with
//! ASCII letter case aside. A `Default` or `Override` that is refused, for
//! whatever reason, still covers what it names, as refused, keyed as well
//! as its text allows: a part it would give a type to is not reported again
//! for having none. One that does not say what it names (no `Extension` or
//! `PartName`, or refused whole by the schema) covers every part.

mod media;
mod read;

pub use media::ContentType;
pub(crate) use read::read;

use crate::PartName;
use std::collections::{BTreeMap, BTreeSet};

/// The media type of a Relationships part (§6.5.2.1).
pub(crate) const RELATIONSHIPS: &str = "application/vnd.openxmlformats-package.relationships+xml";

/// The `Default` and `Override` mappings, each keyed lowered, with `None`
/// for one that is refused.
#[derive(Debug, Default)]
pub(crate) struct ContentTypes {
    defaults: BTreeMap<String, Option<ContentType>>,
    overrides: BTreeMap<String, Option<ContentType>>,
    /// The keys a refused mapping would have named, as far as its text
    /// tells, which no mapping maps: a later mapping of the same key is not
    /// a duplicate of it.
    default_holes: BTreeSet<String>,
    override_holes: BTreeSet<String>,
    /// Whether a mapping that does not say what it names was refused, so
    /// what no mapping names is not known either.
    wild: bool,
}

/// What the stream gives a part.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Lookup<'a> {
    /// This media type.
    Found(&'a ContentType),
    /// A mapping that was refused, already reported.
    Refused,
    /// Nothing.
    Missing,
}

impl ContentTypes {
    /// The media type of the part `name`: an `Override`'s, else a
    /// `Default`'s. A hole where a refused mapping would stand refuses
    /// whatever stands under the same key.
    pub(crate) fn lookup(&self, name: &PartName) -> Lookup<'_> {
        let key = name.key();
        let mapping = if self.override_holes.contains(key) {
            Some(None)
        } else if let Some(mapping) = self.overrides.get(key) {
            Some(mapping.as_ref())
        } else {
            name.extension().and_then(|extension| {
                if self.default_holes.contains(extension) {
                    Some(None)
                } else {
                    self.defaults.get(extension).map(Option::as_ref)
                }
            })
        };
        match mapping {
            Some(Some(content_type)) => Lookup::Found(content_type),
            Some(None) => Lookup::Refused,
            None if self.wild => Lookup::Refused,
            None => Lookup::Missing,
        }
    }
}

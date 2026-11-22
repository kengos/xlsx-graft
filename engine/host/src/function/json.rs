//! The JSON shape of a [`FunctionDescription`](super::FunctionDescription):
//! one private type per object, whose fields are written in the order the
//! contract gives, and the conversions from the compatibility table's types.

use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use xlsx_graft_compat::{
    Added, App, Entry, Feature, Known, Lookup, Prefix, Row, Safety, Support, Version,
};

/// The object of one state of [`Known`], whichever the name is in.
#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum Body<'a> {
    Versioned(Versioned<'a>),
    Baseline(BaselineBody<'a>),
    Unknown(UnknownBody<'a>),
}

/// A function with a row of versions.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Versioned<'a> {
    name: &'a str,
    known: &'static str,
    file_name: Option<String>,
    prefix: Option<&'static str>,
    support: SupportJson,
    requires: Vec<Requirement>,
    safety: SafetyJson,
    sources: &'static [&'static str],
}

/// A function the table gives a safety but no row of versions.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BaselineBody<'a> {
    name: &'a str,
    known: &'static str,
    file_name: Option<String>,
    prefix: Option<&'static str>,
    safety: SafetyJson,
    sources: &'static [&'static str],
}

/// A name the table does not list.
#[derive(Serialize)]
pub(super) struct UnknownBody<'a> {
    name: &'a str,
    known: &'static str,
}

/// The six columns of a row, as the object keyed by application.
pub(super) struct SupportJson(pub(super) Vec<(App, EntryJson)>);

/// One feature a function needs as well.
#[derive(Serialize)]
pub(super) struct Requirement {
    pub(super) feature: &'static str,
    pub(super) support: SupportJson,
    pub(super) sources: &'static [&'static str],
}

/// One cell of the table.
#[derive(Serialize)]
pub(super) struct EntryJson {
    pub(super) added: AddedJson,
    pub(super) removed: Option<&'static str>,
    pub(super) partial: bool,
    pub(super) fallback: Option<&'static str>,
    pub(super) notes: Option<&'static str>,
}

/// When an application added a function: a version written as the table
/// writes it (`2021`, `<=14.4`, `>2007,<=2016`), `false` for never, `null`
/// for not known.
pub(super) enum AddedJson {
    Version(String),
    Never,
    Unknown,
}

/// A function's safety: its class and, for a dangerous one, the kind of
/// danger.
#[derive(Serialize)]
pub(super) struct SafetyJson {
    pub(super) class: &'static str,
    pub(super) danger: Option<&'static str>,
}

impl Body<'_> {
    /// The object that says what `lookup` found.
    pub(super) fn of(lookup: &Lookup) -> Body<'_> {
        let name = lookup.name.as_str();
        match lookup.known {
            Known::Versioned { row, safety } => Body::Versioned(Versioned {
                name,
                known: "versioned",
                file_name: file_name(row.prefix, name),
                prefix: row.prefix.as_str(),
                support: SupportJson::of(&row.support),
                requires: row.requires.iter().map(|f| Requirement::of(*f)).collect(),
                safety: SafetyJson::of(safety),
                sources: row.sources,
            }),
            Known::Baseline { baseline, safety } => Body::Baseline(BaselineBody {
                name,
                known: "baseline",
                file_name: file_name(baseline.prefix, name),
                prefix: baseline.prefix.as_str(),
                safety: SafetyJson::of(safety),
                sources: baseline.sources,
            }),
            _ => Body::Unknown(UnknownBody {
                name,
                known: "unknown",
            }),
        }
    }
}

/// The function's name as Excel writes it in a file; `None` when the table
/// does not say how.
pub(super) fn file_name(prefix: Prefix, name: &str) -> Option<String> {
    prefix.as_str().map(|text| format!("{text}{name}"))
}

impl SupportJson {
    /// The columns of `support`, in the table's order.
    pub(super) fn of(support: &Support) -> Self {
        Self(vec![
            (App::Excel, EntryJson::of(&support.excel)),
            (App::ExcelMac, EntryJson::of(&support.excel_mac)),
            (App::LibreOffice, EntryJson::of(&support.libreoffice)),
            (App::GoogleSheets, EntryJson::of(&support.google_sheets)),
            (App::Numbers, EntryJson::of(&support.numbers)),
            (App::OnlyOffice, EntryJson::of(&support.onlyoffice)),
        ])
    }
}

impl Serialize for SupportJson {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (app, entry) in &self.0 {
            map.serialize_entry(app.as_str(), entry)?;
        }
        map.end()
    }
}

impl Requirement {
    /// The feature's row, as the function that needs it lists it.
    pub(super) fn of(feature: Feature) -> Self {
        let row: &Row = feature.row();
        Self {
            feature: feature.as_str(),
            support: SupportJson::of(&row.support),
            sources: row.sources,
        }
    }
}

impl EntryJson {
    /// The cell `entry` as JSON.
    pub(super) fn of<V: Version>(entry: &Entry<V>) -> Self {
        Self {
            added: AddedJson::of(entry.added),
            removed: entry.removed.map(Version::as_str),
            partial: entry.partial,
            fallback: entry.fallback.map(|fallback| fallback.as_str()),
            notes: entry.notes,
        }
    }
}

impl AddedJson {
    fn of<V: Version>(added: Added<V>) -> Self {
        match added {
            Added::In(version) => Self::Version(version.as_str().to_owned()),
            Added::AtMost(version) => Self::Version(format!("<={}", version.as_str())),
            Added::Between(missing, present) => {
                Self::Version(format!(">{},<={}", missing.as_str(), present.as_str()))
            }
            Added::Never => Self::Never,
            Added::Unknown => Self::Unknown,
        }
    }
}

impl Serialize for AddedJson {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Version(text) => serializer.serialize_str(text),
            Self::Never => serializer.serialize_bool(false),
            Self::Unknown => serializer.serialize_none(),
        }
    }
}

impl SafetyJson {
    /// The class of `safety`. `Safety` is `#[non_exhaustive]`: a variant the
    /// compat crate adds later is written as "unreviewed" until
    /// `SafetyJson::of` matches it.
    pub(super) fn of(safety: Safety) -> Self {
        match safety {
            Safety::ReviewedSafe(_) => Self {
                class: "reviewed_safe",
                danger: None,
            },
            Safety::Dangerous(dangerous) => Self {
                class: "dangerous",
                danger: Some(dangerous.danger.as_str()),
            },
            _ => Self {
                class: "unreviewed",
                danger: None,
            },
        }
    }
}

#[cfg(test)]
mod tests;

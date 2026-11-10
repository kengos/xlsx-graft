//! Reading a JSON setting (the policy or the rules) key by key, reporting
//! every fault as `policy_invalid` or `rules_invalid` rather than stopping
//! at the first.

use serde_json::{Map, Value};
use xlsx_graft_diagnostics::{Diagnostic, Diagnostics, Echo, Location, PartKind, SettingReason};

/// Whose setting is read, which picks the code a fault is reported as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    /// The host's policy: `policy_invalid`.
    Policy,
    /// The caller's rules: `rules_invalid`.
    Rules,
}

/// Where in a setting a value is, as the diagnostic writes it: `forbid`,
/// `forbid.parts[1].kind`. The top of the setting is empty.
#[derive(Debug, Clone, Default)]
pub(crate) struct KeyPath(String);

impl KeyPath {
    /// The key `name` of the object this path is.
    pub(crate) fn key(&self, name: &str) -> Self {
        if self.0.is_empty() {
            return Self(name.to_owned());
        }
        Self(format!("{}.{name}", self.0))
    }

    /// The element `index` of the array this path is.
    pub(crate) fn index(&self, index: usize) -> Self {
        Self(format!("{}[{index}]", self.0))
    }
}

/// The faults found in one setting, pushed into the collector the policy,
/// the rules and the targets all report to.
pub(crate) struct Reporter<'d> {
    side: Side,
    diagnostics: &'d mut Diagnostics,
    failed: bool,
}

impl<'d> Reporter<'d> {
    pub(crate) fn new(side: Side, diagnostics: &'d mut Diagnostics) -> Self {
        Self {
            side,
            diagnostics,
            failed: false,
        }
    }

    /// Whether any fault was reported, kept or not.
    pub(crate) fn failed(&self) -> bool {
        self.failed
    }

    /// Reports `reason` at `at`, with `hint` as the reason defines it.
    pub(crate) fn push(&mut self, at: &KeyPath, reason: SettingReason, hint: &str) {
        self.failed = true;
        let (key, hint) = (Echo::new(&at.0), Echo::new(hint));
        let diagnostic = match self.side {
            Side::Policy => Diagnostic::PolicyInvalid { key, reason, hint },
            Side::Rules => Diagnostic::RulesInvalid { key, reason, hint },
        };
        self.diagnostics.push(diagnostic, Location::default());
    }

    /// The object `value` is, with a fault for each key not in `known`;
    /// `None`, reported, when it is no object.
    pub(crate) fn object<'v>(
        &mut self,
        value: &'v Value,
        at: &KeyPath,
        known: &[&str],
    ) -> Option<&'v Map<String, Value>> {
        let Some(map) = value.as_object() else {
            self.push(at, SettingReason::NotObject, "");
            return None;
        };
        for key in map.keys().filter(|key| !known.contains(&key.as_str())) {
            let hint = camel_case(key).filter(|camel| known.contains(&camel.as_str()));
            self.push(
                &at.key(key),
                SettingReason::UnknownKey,
                hint.as_deref().unwrap_or(""),
            );
        }
        Some(map)
    }

    /// The whole number `value` is, from 1 to `hard_max`.
    pub(crate) fn limit(&mut self, value: &Value, at: &KeyPath, hard_max: u64) -> Option<u64> {
        match value.as_u64() {
            Some(n) if (1..=hard_max).contains(&n) => Some(n),
            Some(_) => self.out_of_range(at, hard_max),
            // A negative number is a number out of range; a float or a
            // string is another type.
            None if value.as_i64().is_some() => self.out_of_range(at, hard_max),
            None => self.wrong_type(at, "a positive integer"),
        }
    }

    fn out_of_range<T>(&mut self, at: &KeyPath, hard_max: u64) -> Option<T> {
        self.push(at, SettingReason::OutOfRange, &format!("1 to {hard_max}"));
        None
    }

    fn wrong_type<T>(&mut self, at: &KeyPath, expected: &str) -> Option<T> {
        self.push(at, SettingReason::WrongType, expected);
        None
    }

    /// The boolean `value` is.
    pub(crate) fn boolean(&mut self, value: &Value, at: &KeyPath) -> Option<bool> {
        let read = value.as_bool();
        if read.is_none() {
            self.push(at, SettingReason::WrongType, "true or false");
        }
        read
    }

    /// The string `value` is, which may not be empty.
    pub(crate) fn text<'v>(&mut self, value: &'v Value, at: &KeyPath) -> Option<&'v str> {
        match value.as_str() {
            Some("") => {
                self.push(at, SettingReason::Empty, "");
                None
            }
            Some(text) => Some(text),
            None => self.wrong_type(at, "a string"),
        }
    }

    /// The function name `text` is, or `None` when it carries the prefix a
    /// workbook writes a function with (`_xlfn.`, `_xlws.`, `_xludf.`): the
    /// prefix is removed before a function is ruled on, so a name with it
    /// would match nothing.
    pub(crate) fn function_name<'t>(&mut self, text: &'t str, at: &KeyPath) -> Option<&'t str> {
        let prefix = text.split_inclusive('.').next().filter(|prefix| {
            let start = prefix.as_bytes().get(..3);
            prefix.ends_with('.') && start.is_some_and(|start| start.eq_ignore_ascii_case(b"_xl"))
        });
        match prefix {
            Some(prefix) => {
                self.push(at, SettingReason::Prefixed, prefix);
                None
            }
            None => Some(text),
        }
    }

    /// The array `value` is, of strings that may not be empty; every
    /// element that is not one is reported, and `None` when any is.
    pub(crate) fn texts<'v>(&mut self, value: &'v Value, at: &KeyPath) -> Option<Vec<&'v str>> {
        let Some(array) = value.as_array() else {
            return self.wrong_type(at, "an array of strings");
        };
        let read: Vec<Option<&str>> = (array.iter().enumerate())
            .map(|(index, element)| self.text(element, &at.index(index)))
            .collect();
        read.into_iter().collect()
    }

    /// The elements of the array `value` is, each to read as an object;
    /// `None`, reported, when it is no array.
    pub(crate) fn array<'v>(&mut self, value: &'v Value, at: &KeyPath) -> Option<&'v [Value]> {
        match value.as_array() {
            Some(array) => Some(array),
            None => self.wrong_type(at, "an array of objects"),
        }
    }

    /// The one of `values` that `text` is the `wire` string of; the values
    /// are listed when it is none.
    pub(crate) fn named<T: Copy>(
        &mut self,
        text: &str,
        at: &KeyPath,
        values: &[T],
        wire: impl Fn(T) -> String,
    ) -> Option<T> {
        let found = values.iter().copied().find(|&value| wire(value) == text);
        if found.is_none() {
            let listed: Vec<String> = values.iter().map(|&value| wire(value)).collect();
            self.push(at, SettingReason::UnknownValue, &listed.join(", "));
        }
        found
    }

    /// The kind of part `text` names.
    pub(crate) fn part_kind(&mut self, text: &str, at: &KeyPath) -> Option<PartKind> {
        self.named(text, at, PartKind::ALL, |kind| kind.to_string())
    }
}

/// `key` in camelCase, when it is written in snake_case: `max_xml_depth`
/// as `maxXmlDepth`.
fn camel_case(key: &str) -> Option<String> {
    if !key.contains('_') {
        return None;
    }
    let mut camel = String::with_capacity(key.len());
    for (position, word) in key.split('_').enumerate() {
        let mut chars = word.chars();
        match (position, chars.next()) {
            (0, first) => camel.extend(first.into_iter().chain(chars)),
            (_, Some(first)) => {
                camel.extend(first.to_uppercase());
                camel.extend(chars);
            }
            (_, None) => {}
        }
    }
    Some(camel)
}

#[cfg(test)]
mod tests;

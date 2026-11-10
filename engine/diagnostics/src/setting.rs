//! The message of `policy_invalid` and `rules_invalid`, which differ only in
//! whose setting they say is wrong.

use crate::{Echo, SettingReason};

/// The message for `reason` at `key` of `side` (`the policy` or `the
/// rules`), with `hint` as the reason defines it.
pub(crate) fn message(side: &str, key: &Echo, reason: SettingReason, hint: &Echo) -> String {
    let at = if key.as_str().is_empty() {
        side.to_owned()
    } else {
        format!("{side} `{key}`")
    };
    match reason {
        SettingReason::TooLarge => format!("{at} is larger than the {hint} bytes it may take"),
        SettingReason::Malformed => format!("{at} is not JSON ({hint})"),
        SettingReason::NotObject => format!("{at} is not a JSON object"),
        SettingReason::UnknownKey if hint.as_str().is_empty() => {
            format!("{at} is not a key it has")
        }
        SettingReason::UnknownKey => format!("{at} is not a key it has; did you mean `{hint}`?"),
        SettingReason::WrongType => format!("{at} is not {hint}"),
        SettingReason::OutOfRange => format!("{at} is out of range; it must be {hint}"),
        SettingReason::UnknownValue => format!("{at} is not a value it takes; it takes {hint}"),
        SettingReason::Required => format!("{at} has no `{hint}`"),
        SettingReason::Empty => format!("{at} is empty"),
        SettingReason::Duplicate => format!("{at} names what an earlier entry already names"),
        SettingReason::Prefixed => {
            format!(
                "{at} names a function with its `{hint}` prefix, which is removed before a function is ruled on; write the name without it"
            )
        }
    }
}

#[cfg(test)]
mod tests;

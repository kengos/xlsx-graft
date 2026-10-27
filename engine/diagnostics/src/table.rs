//! The `diagnostics!` macro that turns the code table (`code.rs`) into
//! [`Diagnostic`] and [`Code`], plus the impls that do not depend on a row.
//!
//! A new code is one row of the table:
//!
//! ```text
//! /// What the variant means.
//! ZipEntryCountExceeded {
//!     /// How many entries the central directory lists.
//!     count: u64,
//!     /// The limit in force.
//!     limit: Limit,
//! } => "zip_entry_count_exceeded", Error,
//!     message: format!("the zip lists {count} entries; the limit is {limit}"),
//!     decided_by: Some(DecidedBy::limit("maxEntries", *limit)),
//!     fix: Some(Fix::policy(format!("maxEntries: {count}")));
//! ```
//!
//! The fields are the code's `args`, written under their camelCase names; each
//! must be an [`ArgValue`](crate::arg::ArgValue), and none is a raw
//! identifier (`r#type` would be written as `r#type`; name it `rel_type`).
//! The three expressions see the fields as references and need not use all of
//! them.

use crate::{Code, Diagnostic};
use serde::{Serialize, Serializer};
use std::fmt;

/// Expands the code table into [`Diagnostic`], [`Code`] and their accessors.
macro_rules! diagnostics {
    ($(
        $(#[$meta:meta])*
        $variant:ident { $( $(#[$field_meta:meta])* $field:ident : $ty:ty ),* $(,)? }
            => $wire:literal, $severity:ident,
            message: $message:expr,
            decided_by: $decided_by:expr,
            fix: $fix:expr;
    )*) => {
        /// One problem the engine found, with its code's arguments as fields.
        ///
        /// [`Diagnostics::push`](crate::Diagnostics::push) records it with
        /// where it was found.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Diagnostic {
            $( $(#[$meta])* $variant { $( $(#[$field_meta])* $field: $ty, )* }, )*
        }

        /// A diagnostic's code alone. Its wire string (snake_case) is the
        /// stable key a consumer matches on.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Code {
            $( #[doc = concat!("`", $wire, "`")] $variant, )*
        }

        impl Code {
            /// Every code, in the table's order.
            pub const ALL: &'static [Code] = &[ $( Code::$variant, )* ];

            /// The stable wire string.
            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self { $( Code::$variant => $wire, )* }
            }

            /// The severity every diagnostic of this code carries.
            #[must_use]
            pub fn severity(self) -> $crate::Severity {
                match self { $( Code::$variant => $crate::Severity::$severity, )* }
            }
        }

        #[allow(unused_variables, reason = "a row's expression need not use every argument")]
        impl Diagnostic {
            /// The code this diagnostic is a case of.
            #[must_use]
            pub fn code(&self) -> Code {
                match self { $( Self::$variant { .. } => Code::$variant, )* }
            }

            /// The code's severity.
            #[must_use]
            pub fn severity(&self) -> $crate::Severity {
                self.code().severity()
            }

            /// The English message, filled in from the arguments.
            #[must_use]
            pub fn message(&self) -> String {
                match self { $( Self::$variant { $($field),* } => $message, )* }
            }

            /// What decided that this is a problem, when anything citable did.
            #[must_use]
            pub fn decided_by(&self) -> Option<$crate::DecidedBy> {
                match self { $( Self::$variant { $($field),* } => $decided_by, )* }
            }

            /// How to resolve it, when the caller can.
            #[must_use]
            pub fn fix(&self) -> Option<$crate::Fix> {
                match self { $( Self::$variant { $($field),* } => $fix, )* }
            }

            /// Writes the arguments into the `args` object.
            pub(crate) fn write_args<M: serde::ser::SerializeMap>(&self, map: &mut M) -> Result<(), M::Error> {
                match self {
                    $( Self::$variant { $($field),* } => {
                        $( $crate::arg::entry(map, stringify!($field), $field)?; )*
                    } )*
                }
                Ok(())
            }
        }
    };
}

pub(crate) use diagnostics;

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Code {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// A diagnostic's arguments, serialized as the `args` object.
pub(crate) struct Args<'a>(pub(crate) &'a Diagnostic);

impl Serialize for Args<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self.0.write_args(&mut map)?;
        map.end()
    }
}

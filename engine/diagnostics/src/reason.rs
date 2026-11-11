//! The reason enums: closed sets of causes (and of kinds) a code takes as
//! an argument, written as their snake_case names.
//!
//! A new enum is one entry of `reasons!`: the enum's doc, then each variant's
//! doc and its wire string. The macro derives the rest (`Display` and
//! `Serialize` both write the wire string) and admits it as an argument type.

use crate::arg::ArgValue;
use serde::{Serialize, Serializer};
use std::fmt;

pub use target::{TargetApp, TargetFallback, TargetList, TargetReason};

/// Expands each entry into an enum whose `Display` and `Serialize` write its
/// wire strings.
macro_rules! reasons {
    ($(
        $(#[$meta:meta])*
        $name:ident { $( $(#[$variant_meta:meta])* $variant:ident => $wire:literal, )* }
    )*) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $( $(#[$variant_meta])* $variant, )*
        }

        impl $name {
            /// Every value, in the order they are declared.
            pub const ALL: &'static [Self] = &[ $( Self::$variant, )* ];

            /// The wire string.
            fn as_str(self) -> &'static str {
                match self { $( Self::$variant => $wire, )* }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl ArgValue for $name {}
    )*};
}

reasons! {
    /// Why a setting the host or the caller passes (the policy or the
    /// rules) is not valid.
    SettingReason {
        /// The JSON is larger than the bytes a setting may take.
        TooLarge => "too_large",
        /// The text is not JSON.
        Malformed => "malformed",
        /// A JSON object was expected.
        NotObject => "not_object",
        /// The key is not one the setting has.
        UnknownKey => "unknown_key",
        /// The value is not of the type the key takes.
        WrongType => "wrong_type",
        /// A limit is 0, or more than the engine can hold to.
        OutOfRange => "out_of_range",
        /// The value is not one the key takes.
        UnknownValue => "unknown_value",
        /// A key an entry needs is missing.
        Required => "required",
        /// A string that must name something is empty.
        Empty => "empty",
        /// A second entry names what an earlier one does.
        Duplicate => "duplicate",
        /// A function is named with a prefix a workbook writes it with
        /// (`_xlfn.`, `_xlws.`, `_xludf.`: any prefix that starts with `_xl`,
        /// in any letter case, and ends with `.`), which is removed before a
        /// function is ruled on, so the name would match nothing.
        Prefixed => "prefixed",
    }

    /// Why a part's XML could not be read as UTF-8 or UTF-16, the only
    /// encodings a package's XML may use.
    EncodingReason {
        /// The byte order mark or the XML declaration names an encoding other
        /// than UTF-8 or UTF-16.
        OtherEncoding => "other_encoding",
        /// The XML declaration names UTF-8 or UTF-16, but the bytes are the
        /// other one, or the other byte order.
        DeclarationMismatch => "declaration_mismatch",
        /// The bytes are read as UTF-8 and are not valid UTF-8.
        InvalidUtf8 => "invalid_utf8",
        /// The bytes are read as UTF-16 and are not valid UTF-16: an odd
        /// length or an unpaired surrogate.
        InvalidUtf16 => "invalid_utf16",
        /// The bytes look like UTF-16 but start without a byte order mark,
        /// which XML requires of UTF-16.
        Utf16WithoutBom => "utf16_without_bom",
    }

    /// Why a zip's central directory cannot be read as its end records
    /// describe it.
    ZipDirectoryReason {
        /// A central directory header is not where the one before it, or the
        /// end record, says it starts, or runs past the directory's size: the
        /// directory holds fewer headers than the end record lists.
        MissingHeader => "missing_header",
        /// Bytes are left after the headers the end record lists: in the
        /// directory, which holds more than it says, or between it and the
        /// end records.
        ExtraBytes => "extra_bytes",
        /// The zip64 end of central directory locator points at no zip64
        /// record, or at one that does not end where the locator starts.
        Zip64RecordMissing => "zip64_record_missing",
        /// Bytes follow the end record's comment: the comment stops before
        /// the end of the file.
        TrailingBytes => "trailing_bytes",
        /// The end record gives a value that is neither left to the zip64
        /// record nor the one the zip64 record gives, so a reader that reads
        /// the zip64 record only when it must would read another directory.
        EndRecordsDisagree => "end_records_disagree",
        /// A header leaves a size or an offset to the zip64 extra field, and
        /// the field does not hold it.
        Zip64FieldMissing => "zip64_field_missing",
    }

    /// The field of a zip entry's local header, or of its data descriptor,
    /// that disagrees with the entry's central directory header.
    ZipHeaderField {
        /// No local header signature where the central directory says the
        /// local header starts.
        Signature => "signature",
        /// The file name.
        Name => "name",
        /// The general purpose bit flags.
        Flags => "flags",
        /// The compression method.
        Method => "method",
        /// The CRC-32 of the inflated data.
        Crc32 => "crc32",
        /// The size of the data as stored.
        CompressedSize => "compressed_size",
        /// The size of the inflated data.
        UncompressedSize => "uncompressed_size",
        /// The data descriptor: what lies between the data and the next
        /// record is not 12, 16, 20 or 24 bytes, or lacks the signature its
        /// length calls for.
        Descriptor => "descriptor",
    }

    /// Why a zip entry's data did not inflate to what its header declares.
    InflateReason {
        /// The deflate stream is not valid.
        CorruptData => "corrupt_data",
        /// The data ends before the inflated size the header declares.
        EndedEarly => "ended_early",
        /// The data inflates to more than the header declares.
        LongerThanDeclared => "longer_than_declared",
    }

    /// Why a zip entry name, a relationship's target or an `Override`'s
    /// `PartName` does not name a part (ECMA-376-2:2021 §6.2.2.2).
    PartNameReason {
        /// The zip entry name is not UTF-8.
        NotUtf8 => "not_utf8",
        /// The name does not start with `/`.
        MissingLeadingSlash => "missing_leading_slash",
        /// A segment is empty: the name is empty, ends with `/`, or holds
        /// `//` (a zip entry name that starts with `/` does).
        EmptySegment => "empty_segment",
        /// A segment holds a character a part name may not: one outside
        /// `ipchar` (RFC 3987 §2.2), such as `\`, `[` or a space, or a `%`
        /// not followed by two hexadecimal digits.
        InvalidCharacter => "invalid_character",
        /// A segment percent-encodes `/` or `\`.
        EncodedSlash => "encoded_slash",
        /// A segment percent-encodes an unreserved ASCII character, such as
        /// `%41` for `A`.
        EncodedUnreserved => "encoded_unreserved",
        /// A segment ends with `.`, as `.` and `..` do.
        EndsWithDot => "ends_with_dot",
        /// The reference has a scheme, or a colon in its first segment: it is
        /// not a relative reference to a part.
        HasScheme => "has_scheme",
        /// The reference starts with `//`, naming an authority: it is not a
        /// reference to a part.
        HasAuthority => "has_authority",
    }

    /// How an XML part breaks its schema: a package XML stream
    /// (ECMA-376-2:2021 Annex C) or a SpreadsheetML part (ECMA-376-1:2016
    /// Annex A).
    SchemaReason {
        /// An element the schema does not allow where it stands: another
        /// root, an unknown child, or one in another namespace.
        UnexpectedElement => "unexpected_element",
        /// An attribute the element does not have.
        UnexpectedAttribute => "unexpected_attribute",
        /// A required attribute is not there.
        MissingAttribute => "missing_attribute",
        /// A required child element is not there.
        MissingElement => "missing_element",
        /// Text other than white space where only elements may stand.
        UnexpectedText => "unexpected_text",
        /// An attribute's value is not one its type allows.
        InvalidValue => "invalid_value",
    }

    /// Why Excel does not take a sheet's name (MS-OI29500 on
    /// ECMA-376-1:2016 §18.2.19, note a).
    SheetNameReason {
        /// The name is empty.
        Empty => "empty",
        /// The name is longer than 31 characters, counted in UTF-16 code
        /// units.
        TooLong => "too_long",
        /// The name holds `[`, `]`, `:`, `*`, `?`, `/` or `\`.
        Character => "character",
        /// The name starts or ends with `'`.
        Apostrophe => "apostrophe",
    }

    /// The kind of content a part holds that the engine denies by default,
    /// as a template's `parts.allow` and a policy's forbid name it.
    PartKind {
        /// VBA or XLM macros: a VBA project, a macro-enabled workbook, a
        /// macro sheet or a dialog sheet.
        Macro => "macro",
        /// A link to another workbook or a resource outside the package.
        ExternalLink => "external_link",
        /// A DDE link: a value another program serves.
        Dde => "dde",
        /// An embedded or linked OLE object.
        Ole => "ole",
        /// An ActiveX control.
        Activex => "activex",
        /// A data connection: a connection, a query table, an XML map's data
        /// binding or a Power Query definition.
        DataConnection => "data_connection",
        /// A Web publishing item, which republishes part of the workbook
        /// to a location of its own.
        WebPublish => "web_publish",
    }

    /// Why a formula could not be read: the first place the formula
    /// reader stopped trusting it.
    FormulaMalformedReason {
        /// An `_xHHHH_` escape that names a surrogate, which no character
        /// is.
        UndecodableEscape => "undecodable_escape",
        /// An escape written `_XHHHH_`, which ECMA-376 does not say is one.
        UpperCaseEscape => "upper_case_escape",
        /// A string with no closing `"`.
        UnclosedString => "unclosed_string",
        /// A quoted sheet name with no closing `'`.
        UnclosedSheetName => "unclosed_sheet_name",
        /// Brackets with no closing `]`.
        UnclosedBracket => "unclosed_bracket",
        /// Brackets nested deeper than the formula reader follows.
        BracketsTooDeep => "brackets_too_deep",
        /// A character no part of a formula starts with.
        UnknownCharacter => "unknown_character",
        /// A quoted sheet name with no `!` or `|` after it.
        StraySheetName => "stray_sheet_name",
        /// An element inside the formula's element, which a formula is
        /// not: which of its text Excel reads is not known.
        Markup => "markup",
    }

    /// Why the engine denies a function it has not found dangerous.
    FunctionDenial {
        /// The engine knows the function but has not reviewed it yet.
        Unreviewed => "unreviewed",
        /// The engine does not know the function: a user-defined one, one
        /// newer than the engine, or a name that is no function at all.
        Unknown => "unknown",
    }
}

// After the macro: a `macro_rules!` is in scope only below its definition.
mod target;

#[cfg(test)]
mod tests;

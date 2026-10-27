//! The reason enums: closed sets of causes a code takes as an argument,
//! written as their snake_case names.
//!
//! A new enum is one entry of `reasons!`: the enum's doc, then each variant's
//! doc and its wire string. The macro derives the rest (`Display` and
//! `Serialize` both write the wire string) and admits it as an argument type.

use crate::arg::ArgValue;
use serde::{Serialize, Serializer};
use std::fmt;

/// Expands each entry into an enum whose `Display` and `Serialize` write its
/// wire strings.
macro_rules! reasons {
    ($(
        $(#[$meta:meta])*
        $name:ident { $( $(#[$variant_meta:meta])* $variant:ident => $wire:literal, )* }
    )*) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$variant_meta])* $variant, )*
        }

        impl $name {
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

    /// How a package XML stream breaks its schema (ECMA-376-2:2021 Annex C).
    SchemaReason {
        /// An element the schema does not allow where it stands: another
        /// root, an unknown child, or one in another namespace.
        UnexpectedElement => "unexpected_element",
        /// An attribute the element does not have.
        UnexpectedAttribute => "unexpected_attribute",
        /// A required attribute is not there.
        MissingAttribute => "missing_attribute",
        /// Text other than white space where only elements may stand.
        UnexpectedText => "unexpected_text",
        /// An attribute's value is not one its type allows.
        InvalidValue => "invalid_value",
    }
}

#[cfg(test)]
mod tests;

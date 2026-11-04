//! What an application that lacks a function or feature shows instead:
//! the vocabulary of a cell's fallback.

/// What a version of an application that lacks a function or feature does
/// with it when it opens the workbook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fallback {
    /// Skipped and not shown, as an unknown extension is.
    Ignored,
    /// Only the value saved with the cell is shown.
    ValueOnly,
    /// The formula evaluates to `#NAME?`.
    NameError,
    /// The cell shows `#VALUE!`.
    ValueError,
    /// A legacy array formula of a fixed size.
    LegacyArray,
    /// A legacy note (comment).
    AsNote,
    /// A shape that stands in for the object.
    PlaceholderShape,
    /// Part of it works; the entry's notes say which.
    Degraded,
    /// The application offers to repair the workbook.
    RepairPrompt,
    /// The workbook does not open.
    Unopenable,
}

impl Fallback {
    /// Every fallback, in declaration order.
    pub const ALL: &'static [Fallback] = &[
        Fallback::Ignored,
        Fallback::ValueOnly,
        Fallback::NameError,
        Fallback::ValueError,
        Fallback::LegacyArray,
        Fallback::AsNote,
        Fallback::PlaceholderShape,
        Fallback::Degraded,
        Fallback::RepairPrompt,
        Fallback::Unopenable,
    ];

    /// The stable snake_case name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Fallback::Ignored => "ignored",
            Fallback::ValueOnly => "value_only",
            Fallback::NameError => "name_error",
            Fallback::ValueError => "value_error",
            Fallback::LegacyArray => "legacy_array",
            Fallback::AsNote => "as_note",
            Fallback::PlaceholderShape => "placeholder_shape",
            Fallback::Degraded => "degraded",
            Fallback::RepairPrompt => "repair_prompt",
            Fallback::Unopenable => "unopenable",
        }
    }
}

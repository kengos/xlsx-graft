namespace XlsxGraft;

/// <summary>
/// One row a sheet writes: its number, from 1, and the value of each column
/// from A to its last cell, <see langword="null"/> where a column has none.
/// </summary>
/// <remarks>
/// <para>
/// A value is <see langword="null"/>, a <see cref="string"/>, a
/// <see cref="double"/>, a <see cref="bool"/>, a <see cref="DateOnly"/>, a
/// <see cref="TimeOnly"/>, a <see cref="DateTime"/> of
/// <see cref="DateTimeKind.Unspecified"/> (the workbook's wall-clock time:
/// no time zone is applied), a <see cref="TimeSpan"/> (an elapsed time, not
/// wrapped at a day) or a <see cref="CellError"/>.
/// </para>
/// <para>Two rows are equal when their numbers are and their values are, one by one.</para>
/// </remarks>
/// <param name="Number">The row's number, from 1 to 1,048,576.</param>
/// <param name="Values">The value of each column from A.</param>
public sealed record Row(int Number, IReadOnlyList<object?> Values)
{
    /// <inheritdoc/>
    public bool Equals(Row? other) => other is not null && Number == other.Number && Values.SequenceEqual(other.Values);

    /// <summary>Hashed by the number and how many values there are.</summary>
    /// <returns>The hash.</returns>
    public override int GetHashCode() => HashCode.Combine(Number, Values.Count);
}

/// <summary>
/// A cell holding an error value, its text as the workbook writes it
/// (<c>"#N/A"</c>, <c>"#DIV/0!"</c>). Kept apart from a <see cref="string"/>
/// so that a cell holding the text <c>"#N/A"</c> is not read as the error.
/// </summary>
/// <param name="Text">The error's text.</param>
public sealed record CellError(string Text);

/// <summary>
/// A sheet as the workbook lists it, from <see cref="Book.Sheets"/>.
/// </summary>
/// <param name="Index">
/// From 0, hidden sheets and chartsheets counted, as <see cref="Book.Sheet(int)"/> counts.
/// </param>
/// <param name="Name">The sheet's name.</param>
/// <param name="Kind">What the sheet is; only a worksheet has rows to read.</param>
/// <param name="State">Whether it is shown.</param>
public sealed record SheetInfo(int Index, string Name, SheetKind Kind, SheetState State);

/// <summary>What a sheet is.</summary>
public enum SheetKind
{
    /// <summary>The one kind with rows to read.</summary>
    Worksheet,

    /// <summary>Holds no cells.</summary>
    Chartsheet,

    /// <summary>Holds no cells.</summary>
    Dialogsheet,

    /// <summary>Its cells are not read.</summary>
    Macrosheet,

    /// <summary>Its part is not known (the diagnostics say why), or a kind newer than the package.</summary>
    Unknown,
}

/// <summary>Whether a sheet is shown.</summary>
public enum SheetState
{
    /// <summary>Shown.</summary>
    Visible,

    /// <summary>Hidden; a user can show it again.</summary>
    Hidden,

    /// <summary>Hidden; only a program can show it again.</summary>
    VeryHidden,

    /// <summary>A state newer than the package.</summary>
    Unknown,
}

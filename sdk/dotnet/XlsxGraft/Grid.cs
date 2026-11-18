namespace XlsxGraft;

/// <summary>Which of a grid's limits cut what it shows.</summary>
/// <param name="Rows">Whether rows were left out.</param>
/// <param name="Columns">Whether columns were left out.</param>
/// <param name="Merges">Whether merged ranges were left out.</param>
public sealed record GridTruncated(bool Rows, bool Columns, bool Merges);

/// <summary>One cell of a grid that holds a value.</summary>
/// <param name="Ref">The cell's reference (<c>B3</c>).</param>
/// <param name="Type">
/// The cell's type: <c>string</c>, <c>number</c>, <c>boolean</c>,
/// <c>date</c>, <c>time</c>, <c>datetime</c>, <c>elapsed</c> or <c>error</c>.
/// </param>
/// <param name="Value">
/// The JSON's value: a <see cref="string"/> (also for a date, a time, a
/// date-time and an error), a <see cref="bool"/>, a <see cref="long"/> for a
/// number written as an integer (an elapsed time's milliseconds) and a
/// <see cref="double"/> for any other number.
/// </param>
/// <param name="Truncated">Whether a long text was cut for display.</param>
public sealed record GridCell(string Ref, string Type, object Value, bool Truncated);

/// <summary>One row of a grid.</summary>
/// <param name="Row">The row's number, from 1.</param>
/// <param name="Cells">The row's cells that hold a value, from the left.</param>
public sealed record GridRow(int Row, IReadOnlyList<GridCell> Cells);

/// <summary>
/// The cells of one sheet, the value of
/// <see cref="Viewer.DescribeSheet(string, string?, int?, string?, int?, int?)"/>.
/// Names and text are made for display (control characters removed, long text
/// cut), so they are not exact; read the rows with <see cref="Book"/> for
/// exact values.
/// </summary>
/// <param name="Sheet">The sheet shown, or <see langword="null"/> when none was found.</param>
/// <param name="Dimension">The range the sheet declares, or <see langword="null"/> when it declares none.</param>
/// <param name="Used">The range that holds cells, or <see langword="null"/> when the sheet is empty.</param>
/// <param name="Shown">The range of the cells shown, or <see langword="null"/> when none are.</param>
/// <param name="Truncated">Which limits cut what is shown.</param>
/// <param name="Complete">Whether the whole sheet was read.</param>
/// <param name="Rows">The rows shown.</param>
/// <param name="Merges">The merged ranges (<c>A1:C1</c>).</param>
public sealed record Grid(
    SheetInfo? Sheet,
    string? Dimension,
    string? Used,
    string? Shown,
    GridTruncated Truncated,
    bool Complete,
    IReadOnlyList<GridRow> Rows,
    IReadOnlyList<string> Merges);

namespace XlsxGraft;

/// <summary>A sheet asked for by name or index, from <see cref="Book.Sheet(string)"/>.</summary>
public sealed class Sheet
{
    private readonly Book book;
    private readonly string? name;
    private readonly int index;

    internal Sheet(Book book, string? name, int index)
    {
        this.book = book;
        this.name = name;
        this.index = index;
    }

    /// <summary>
    /// Each row the sheet writes, in order, read one at a time: no more of the
    /// sheet is held than the row given. Only the rows the sheet writes come.
    /// Each enumeration reads the sheet from its start, when the first row is
    /// asked for.
    /// </summary>
    /// <remarks>
    /// Reading stops at the first problem in the sheet, the rows before it
    /// having come, and gives nothing when the engine refuses the sheet (no
    /// such sheet, not a worksheet); either way the book's
    /// <see cref="Book.Result"/> fails, saying why. An enumerator left
    /// unfinished holds its rows until it is disposed (a <c>foreach</c> does)
    /// or the book is.
    /// </remarks>
    /// <returns>The rows, read as they are enumerated.</returns>
    /// <exception cref="ObjectDisposedException">A row is asked for after the book is closed.</exception>
    public IEnumerable<Row> Rows()
    {
        var rows = book.OpenRows(name, index);
        if (rows is null)
        {
            yield break;
        }

        try
        {
            while (book.NextRow(rows) is { } row)
            {
                yield return row;
            }
        }
        finally
        {
            book.CloseRows(rows);
        }
    }
}

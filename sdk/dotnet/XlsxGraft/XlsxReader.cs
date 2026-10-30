namespace XlsxGraft;

/// <summary>Opens a workbook to read its sheets' rows, through the C ABI.</summary>
/// <example>
/// <code>
/// var book = XlsxGraft.XlsxReader.Open("book.xlsx");
/// using (book)
/// {
///     foreach (var row in book.Sheet("Data").Rows())
///     {
///         Console.WriteLine($"{row.Number}: {string.Join(", ", row.Values)}");
///     }
/// }
///
/// // after the using block
/// var result = book.Result;
/// </code>
/// </example>
public static class XlsxReader
{
    /// <summary>The workbook at <paramref name="path"/>, opened now.</summary>
    /// <remarks>
    /// A file that cannot be opened gives a book all the same, with no
    /// sheets, whose <see cref="Book.Result"/> says why (<c>io</c>). A path
    /// that is not Unicode (a lone surrogate) is refused with
    /// <c>invalid_arg</c>.
    /// </remarks>
    /// <param name="path">The workbook's path.</param>
    /// <param name="library">
    /// The engine library's path, used when <c>XLSX_GRAFT_LIBRARY</c> is not set.
    /// </param>
    /// <returns>The book, to read and dispose of.</returns>
    /// <exception cref="LibraryNotFoundException">No engine library was found or loaded, or it lacks a function.</exception>
    public static Book Open(string path, string? library = null) =>
        Open(path, library, Environment.GetEnvironmentVariable);

    /// <summary>
    /// As <see cref="Open(string, string?)"/>, the workbook being the bytes of
    /// <paramref name="data"/>, copied now: changing them after changes nothing.
    /// </summary>
    /// <param name="data">The workbook's bytes.</param>
    /// <param name="library">
    /// The engine library's path, used when <c>XLSX_GRAFT_LIBRARY</c> is not set.
    /// </param>
    /// <returns>The book, to read and dispose of.</returns>
    /// <exception cref="LibraryNotFoundException">No engine library was found or loaded, or it lacks a function.</exception>
    public static Book OpenBytes(ReadOnlySpan<byte> data, string? library = null) =>
        OpenBytes(data, library, Environment.GetEnvironmentVariable);

    internal static Book Open(string path, string? library, Func<string, string?> env)
    {
        ArgumentNullException.ThrowIfNull(path);
        var capi = new Capi(Library.Open(library, env));
        var encoded = Capi.Utf8(path);
        return encoded is null
            ? new Book(capi, (Capi _, ref IntPtr _) => StatusDiagnostic.InvalidArg)
            : new Book(capi, (Capi c, ref IntPtr book) => c.BookOpenPath(encoded, ref book));
    }

    internal static Book OpenBytes(ReadOnlySpan<byte> data, string? library, Func<string, string?> env)
    {
        var capi = new Capi(Library.Open(library, env));
        var copy = data.ToArray();
        return new Book(capi, (Capi c, ref IntPtr book) => c.BookOpenBytes(copy, ref book));
    }
}

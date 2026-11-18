// Shows a workbook's parts and a sheet's cells through the C ABI (ViewCall.cs):
// the engine's `tree` and `describe_sheet` views (docs/sdk.md § Views in an
// SDK). A Viewer holds only the ViewCall and does not change after it is
// made, so it needs no lock. The limits a workbook is read under are the
// CLI's defaults; no policy is taken.

using System.Text.Json;

namespace XlsxGraft;

/// <summary>
/// Shows the parts of a workbook and the cells of its sheets: the engine's
/// <c>tree</c> and <c>describe_sheet</c> views.
/// </summary>
/// <remarks>
/// A viewer does not change once it is made, so one may be shared by any
/// number of threads, and their calls may run at once.
/// </remarks>
/// <example>
/// <code>
/// var viewer = new XlsxGraft.Viewer();
/// var tree = viewer.Tree("book.xlsx").Value;
/// foreach (var part in tree?.Parts ?? [])
/// {
///     Console.WriteLine($"{part.Name} {part.ContentType}");
/// }
///
/// var result = viewer.DescribeSheet("book.xlsx", sheet: "Data", range: "A1:H50", maxRows: 20);
/// foreach (var row in result.Value?.Rows ?? [])
/// {
///     Console.WriteLine(string.Join(", ", row.Cells.Select(cell => cell.Value)));
/// }
/// </code>
/// </example>
public sealed class Viewer
{
    private readonly ViewCall call;

    /// <summary>Makes a viewer.</summary>
    /// <param name="library">
    /// The engine library's path, used when <c>XLSX_GRAFT_LIBRARY</c> is not set.
    /// </param>
    /// <exception cref="LibraryNotFoundException">No engine library was found or loaded, or it lacks a function.</exception>
    public Viewer(string? library = null)
        : this(library, Environment.GetEnvironmentVariable)
    {
    }

    internal Viewer(string? library, Func<string, string?> env)
        : this(new ViewCall(Library.Open(library, env)))
    {
    }

    internal Viewer(ViewCall call)
    {
        this.call = call;
    }

    /// <summary>The parts of the workbook at <paramref name="path"/> and the relationships between them.</summary>
    /// <remarks>
    /// A path that is not Unicode (a lone surrogate) cannot go to the engine,
    /// so this package reads the file and shows its bytes: a file that cannot
    /// be read is an <c>io</c> <see cref="Result.Error"/> with no value.
    /// </remarks>
    /// <param name="path">The workbook's path.</param>
    /// <returns>
    /// A success when the engine's <c>ok</c> is true. The value is a
    /// <see cref="XlsxGraft.Tree"/> also when it is not (a workbook that is
    /// broken, or a file that cannot be opened, gives a tree with the parts
    /// that were read, perhaps none); it is <see langword="null"/> only when
    /// this package failed to get one (<c>panic</c>, <c>null_arg</c>,
    /// <c>unknown_status</c>, <c>diagnostics_unreadable</c>, or <c>io</c> for
    /// a path that is not Unicode whose file this package cannot read). A
    /// file that cannot be opened is an <c>io</c> <see cref="Result.Error"/>.
    /// </returns>
    /// <exception cref="ArgumentNullException"><paramref name="path"/> is <see langword="null"/>.</exception>
    public Result<Tree> Tree(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        return PathInput.Call(
            path,
            encoded => Report(call.TreePath(encoded), ViewContent.Tree),
            data => Report(call.TreeBytes(data), ViewContent.Tree),
            Unreadable<Tree>);
    }

    /// <summary>
    /// As <see cref="Tree(string)"/>, for the workbook in <paramref name="data"/>,
    /// which is not copied: it is pinned and read during the call only, so it
    /// must not change meanwhile.
    /// </summary>
    /// <param name="data">The workbook's bytes.</param>
    /// <returns>As <see cref="Tree(string)"/>.</returns>
    public Result<Tree> TreeBytes(ReadOnlySpan<byte> data) =>
        Report(call.TreeBytes(data), ViewContent.Tree);

    /// <summary>The cells of one sheet of the workbook at <paramref name="path"/>.</summary>
    /// <param name="path">The workbook's path; as for <see cref="Tree(string)"/>.</param>
    /// <param name="sheet">The sheet's name; not together with <paramref name="sheetIndex"/>.</param>
    /// <param name="sheetIndex">The sheet's index from 0; the first sheet is shown without it and <paramref name="sheet"/>.</param>
    /// <param name="range">
    /// An A1 range such as <c>"A1:H50"</c>: the cells shown start at its top
    /// left cell (A1 without it) and never go past it.
    /// </param>
    /// <param name="maxRows">1 to 10000 (100 without it).</param>
    /// <param name="maxColumns">1 to 1000 (30 without it).</param>
    /// <returns>
    /// A success when the engine's <c>ok</c> is true. The value is a
    /// <see cref="XlsxGraft.Grid"/> also when it is not (a sheet that is not
    /// found gives a grid whose <see cref="Grid.Sheet"/> is
    /// <see langword="null"/>); it is <see langword="null"/> in the cases of
    /// <see cref="Tree(string)"/>.
    /// </returns>
    /// <exception cref="ArgumentNullException"><paramref name="path"/> is <see langword="null"/>.</exception>
    /// <exception cref="ArgumentException">
    /// The engine refuses a value, with its message (<paramref name="range"/>
    /// that is no A1 range, <paramref name="maxRows"/> out of range, a
    /// <paramref name="sheet"/> together with a <paramref name="sheetIndex"/>,
    /// a <paramref name="sheetIndex"/> below 0); or, before the call, a
    /// <paramref name="sheet"/> or a <paramref name="range"/> holds a lone
    /// surrogate, or the options are more than 2 MiB of JSON.
    /// </exception>
    public Result<Grid> DescribeSheet(
        string path,
        string? sheet = null,
        int? sheetIndex = null,
        string? range = null,
        int? maxRows = null,
        int? maxColumns = null)
    {
        ArgumentNullException.ThrowIfNull(path);
        var options = ViewOptions.Build(sheet, sheetIndex, range, maxRows, maxColumns);
        return PathInput.Call(
            path,
            encoded => Report(call.SheetPath(encoded, options), ViewContent.Grid),
            data => Report(call.SheetBytes(data, options), ViewContent.Grid),
            Unreadable<Grid>);
    }

    /// <summary>
    /// As <see cref="DescribeSheet"/>, for the workbook in <paramref name="data"/>,
    /// which is not copied: it is pinned and read during the call only, so it
    /// must not change meanwhile.
    /// </summary>
    /// <param name="data">The workbook's bytes.</param>
    /// <param name="sheet">As for <see cref="DescribeSheet"/>.</param>
    /// <param name="sheetIndex">As for <see cref="DescribeSheet"/>.</param>
    /// <param name="range">As for <see cref="DescribeSheet"/>.</param>
    /// <param name="maxRows">As for <see cref="DescribeSheet"/>.</param>
    /// <param name="maxColumns">As for <see cref="DescribeSheet"/>.</param>
    /// <returns>As <see cref="DescribeSheet"/>.</returns>
    /// <exception cref="ArgumentException">As for <see cref="DescribeSheet"/>.</exception>
    public Result<Grid> DescribeSheetBytes(
        ReadOnlySpan<byte> data,
        string? sheet = null,
        int? sheetIndex = null,
        string? range = null,
        int? maxRows = null,
        int? maxColumns = null)
    {
        var options = ViewOptions.Build(sheet, sheetIndex, range, maxRows, maxColumns);
        return Report(call.SheetBytes(data, options), ViewContent.Grid);
    }

    private static Result<T> Report<T>((int Status, byte[]? Json, string? Message) outcome, Func<JsonElement, T> content)
        where T : notnull =>
        ViewReport.Call(outcome.Status, outcome.Json, outcome.Message, content);

    private static Result<T> Unreadable<T>(Diagnostic error)
        where T : notnull =>
        Result<T>.Failure([], error, default);
}

namespace XlsxGraft.Tests;

public sealed class ViewerTests
{
    private static readonly Viewer Default = new(library: Repo.EngineLibrary);
    private static readonly string Table = VerifyCallTests.Fixture("table.xlsx");
    private static readonly string Merged = VerifyCallTests.Fixture("merged.xlsx");

    private static GridCell Cell(Grid grid, string reference) =>
        grid.Rows.SelectMany(row => row.Cells).Single(cell => cell.Ref == reference);

    [Fact]
    public void Tree_lists_the_parts_and_the_relationships_of_the_workbook_at_a_path()
    {
        var result = Default.Tree(Table);

        Assert.True(result.Success);
        Assert.Null(result.Error);
        var tree = result.Value!;
        Assert.True(tree.Complete);
        Assert.Contains("/xl/workbook.xml", tree.Parts.Select(part => part.Name));
        Assert.Contains("/xl/workbook.xml", tree.Relationships.Select(relationship => relationship.Target.Part));
    }

    [Fact]
    public void Tree_gives_the_same_tree_for_the_bytes_of_the_file()
    {
        Assert.Equivalent(Default.Tree(Table).Value!, Default.TreeBytes(File.ReadAllBytes(Table)).Value!);
    }

    [Fact]
    public void Tree_gives_a_target_outside_the_package_as_external_with_no_part()
    {
        var tree = Default.Tree(VerifyCallTests.Fixture("external-link.xlsx")).Value!;

        var external = tree.Parts.SelectMany(part => part.Relationships).Select(relationship => relationship.Target).First(target => target.External is not null);

        Assert.Null(external.Part);
        Assert.NotEmpty(external.External!);
    }

    [Fact]
    public void Tree_fails_with_an_io_error_and_a_tree_with_no_parts_for_a_file_that_does_not_exist()
    {
        var result = Default.Tree("/nowhere/book.xlsx");

        Assert.False(result.Success);
        Assert.Equal("io", result.Error?.Code);
        Assert.Empty(result.Value!.Parts);
    }

    [Fact]
    public void TreeBytes_fails_with_diagnostics_and_a_tree_with_no_parts_for_bytes_that_are_not_a_workbook()
    {
        var result = Default.TreeBytes("not a zip"u8);

        Assert.False(result.Success);
        Assert.Null(result.Error);
        Assert.NotEmpty(result.Diagnostics);
        Assert.Empty(result.Value!.Parts);
    }

    [Fact]
    public void Tree_reads_a_file_whose_path_is_not_Unicode_and_has_no_value_when_it_cannot_be_read()
    {
        var directory = Directory.CreateTempSubdirectory("xlsx-graft-");
        try
        {
            // File.Copy and File.ReadAllBytes both replace each lone surrogate
            // of the path with U+FFFD, so the file copied to this path is the
            // file that Tree reads for the same path.
            var path = Path.Combine(directory.FullName, "\uD800.xlsx");
            File.Copy(Table, path);

            var read = Default.Tree(path);
            var missing = Default.Tree(Path.Combine(directory.FullName, "\uD800\uD800.xlsx"));

            Assert.Equivalent(Default.Tree(Table).Value!, read.Value!);
            Assert.Equal("io", missing.Error?.Code);
            Assert.StartsWith("the file could not be read: ", missing.Error?.Message, StringComparison.Ordinal);
            Assert.Null(missing.Value);
        }
        finally
        {
            directory.Delete(recursive: true);
        }
    }

    [Fact]
    public void Tree_and_DescribeSheet_throw_for_a_path_that_is_null()
    {
        Assert.Throws<ArgumentNullException>(() => Default.Tree(null!));
        Assert.Throws<ArgumentNullException>(() => Default.DescribeSheet(null!));
    }

    [Fact]
    public void DescribeSheet_shows_the_sheet_the_ranges_and_the_merges_of_the_first_sheet()
    {
        var result = Default.DescribeSheet(Merged);

        Assert.True(result.Success);
        Assert.Null(result.Error);
        var grid = result.Value!;
        Assert.Equal(new SheetInfo(0, "Sheet1", SheetKind.Worksheet, SheetState.Visible), grid.Sheet);
        Assert.Equal(("A1:E8", "A1:E8", "A1:E8", true), (grid.Dimension, grid.Used, grid.Shown, grid.Complete));
        Assert.Equal(new GridTruncated(Rows: false, Columns: false, Merges: false), grid.Truncated);
        Assert.Equal(["A1:C1", "A3:A5"], grid.Merges);
    }

    [Fact]
    public void DescribeSheet_shows_the_cells_of_the_first_sheet()
    {
        var grid = Default.DescribeSheet(Merged).Value!;

        Assert.Equal(1, grid.Rows[0].Row);
        Assert.Equal(new GridCell("A1", "string", "Quarterly report", Truncated: false), grid.Rows[0].Cells[0]);
        Assert.Equal(1.5, Cell(grid, "C3").Value);
        Assert.Equal(2.0, Assert.IsType<double>(Cell(grid, "C4").Value));
        Assert.Equal(new GridCell("C6", "date", "2024-01-15", Truncated: false), Cell(grid, "C6"));
    }

    [Fact]
    public void DescribeSheet_gives_the_same_grid_for_the_bytes_of_the_file()
    {
        Assert.Equivalent(Default.DescribeSheet(Merged).Value!, Default.DescribeSheetBytes(File.ReadAllBytes(Merged)).Value!);
    }

    [Fact]
    public void DescribeSheet_takes_the_window_and_the_sheet_that_the_arguments_name()
    {
        Assert.Equal("A1:C3", Default.DescribeSheet(Merged, range: "A1:C3").Value!.Shown);
        Assert.Equal("A1:E2", Default.DescribeSheet(Merged, maxRows: 2).Value!.Shown);
        Assert.Equal("A1:B8", Default.DescribeSheet(Merged, maxColumns: 2).Value!.Shown);
        Assert.Equal("Sheet1", Default.DescribeSheet(Merged, sheet: "Sheet1").Value!.Sheet!.Name);
        Assert.Equivalent(Default.DescribeSheet(Merged).Value!, Default.DescribeSheet(Merged, sheetIndex: 0).Value!);
        Assert.Equal("A1:E2", Default.DescribeSheetBytes(File.ReadAllBytes(Merged), sheet: "Sheet1", maxRows: 2).Value!.Shown);
    }

    [Fact]
    public void DescribeSheet_fails_with_sheet_not_found_and_a_grid_whose_sheet_is_null()
    {
        var result = Default.DescribeSheet(Table, sheet: "Nope");

        Assert.False(result.Success);
        Assert.Equal("sheet_not_found", result.Error?.Code);
        Assert.Null(result.Value!.Sheet);
        Assert.Empty(result.Value.Rows);
    }

    [Fact]
    public void DescribeSheet_reads_a_file_whose_path_is_not_Unicode_as_bytes_and_shows_the_same_grid()
    {
        var directory = Directory.CreateTempSubdirectory("xlsx-graft-");
        try
        {
            var path = Path.Combine(directory.FullName, "\uD800.xlsx");
            File.Copy(Merged, path);

            Assert.Equivalent(Default.DescribeSheet(Merged).Value!, Default.DescribeSheet(path).Value!);
        }
        finally
        {
            directory.Delete(recursive: true);
        }
    }

    [Fact]
    public void DescribeSheet_throws_ArgumentException_with_the_engines_message_for_a_value_the_engine_refuses()
    {
        Assert.Contains("range", Assert.Throws<ArgumentException>(() => Default.DescribeSheet(Merged, range: "nope")).Message, StringComparison.Ordinal);
        Assert.Contains("maxRows", Assert.Throws<ArgumentException>(() => Default.DescribeSheet(Merged, maxRows: 0)).Message, StringComparison.Ordinal);
        Assert.Contains("sheet", Assert.Throws<ArgumentException>(() => Default.DescribeSheet(Merged, sheet: "Sheet1", sheetIndex: 0)).Message, StringComparison.Ordinal);
        Assert.Contains("sheetIndex", Assert.Throws<ArgumentException>(() => Default.DescribeSheet(Merged, sheetIndex: -1)).Message, StringComparison.Ordinal);
    }

    [Fact]
    public void DescribeSheet_throws_ArgumentException_before_the_call_for_options_that_cannot_be_sent()
    {
        var long_ = new string('A', 2 << 20);

        Assert.Contains("the engine takes 2097152", Assert.Throws<ArgumentException>(() => Default.DescribeSheet("/nowhere", range: long_)).Message, StringComparison.Ordinal);
        Assert.Contains("the engine takes 2097152", Assert.Throws<ArgumentException>(() => Default.DescribeSheetBytes([], range: long_)).Message, StringComparison.Ordinal);
        Assert.Throws<ArgumentException>(() => Default.DescribeSheet(Merged, range: "\uD800"));
        Assert.Throws<ArgumentException>(() => Default.DescribeSheetBytes([], sheet: "\uD800"));
    }

    [Fact]
    public void A_missing_engine_library_throws_LibraryNotFoundException()
    {
        Assert.Throws<LibraryNotFoundException>(() => new Viewer("/elsewhere.so", _ => null));
    }

    [Fact]
    public void A_panic_of_the_engine_is_a_failure_with_the_panic_error_and_no_value()
    {
        var viewer = new Viewer(new FakeCall
        {
            TreePathWith = _ => (StatusDiagnostic.Panic, null, null),
            TreeBytesWith = _ => (StatusDiagnostic.Panic, null, null),
            SheetPathWith = (_, _) => (StatusDiagnostic.Panic, null, null),
            SheetBytesWith = (_, _) => (StatusDiagnostic.Panic, null, null),
        });

        var results = new Result[] { viewer.Tree(Table), viewer.TreeBytes([]), viewer.DescribeSheet(Table), viewer.DescribeSheetBytes([]) };

        Assert.All(results, result => Assert.Equal("panic", result.Error?.Code));
        Assert.All(results, result => Assert.Empty(result.Diagnostics));
        Assert.Null(((Result<Tree>)results[0]).Value);
        Assert.Null(((Result<Grid>)results[3]).Value);
    }

    [Fact]
    public void A_status_the_package_does_not_know_is_a_failure_with_unknown_status()
    {
        var viewer = new Viewer(new FakeCall { SheetBytesWith = (_, _) => (99, null, null) });

        Assert.Equal("unknown_status", viewer.DescribeSheetBytes([]).Error?.Code);
    }

    [Fact]
    public void A_request_invalid_with_no_message_throws_ArgumentException_with_a_fixed_message()
    {
        var viewer = new Viewer(new FakeCall { SheetBytesWith = (_, _) => (StatusDiagnostic.RequestInvalid, null, null) });

        var error = Assert.Throws<ArgumentException>(() => viewer.DescribeSheetBytes([]));

        Assert.Contains("gave no reason", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void An_invalid_arg_throws_XlsxGraftException()
    {
        var viewer = new Viewer(new FakeCall { TreeBytesWith = _ => (StatusDiagnostic.InvalidArg, null, null) });

        Assert.Throws<XlsxGraftException>(() => viewer.TreeBytes([]));
    }

    [Fact]
    public async Task A_viewer_may_be_shared_by_threads_that_call_it_at_once()
    {
        var data = await File.ReadAllBytesAsync(Merged, TestContext.Current.CancellationToken);

        var tasks = Enumerable.Range(0, 4).Select(_ => Task.Run(() => Default.DescribeSheetBytes(data).Value!)).ToArray();
        var grids = await Task.WhenAll(tasks);

        Assert.All(grids, grid => Assert.Equivalent(grids[0], grid));
        Assert.NotEmpty(grids[0].Rows);
    }

    private sealed class FakeCall : ViewCall
    {
        internal FakeCall()
            : base(Library.Open(Repo.EngineLibrary, _ => null))
        {
        }

        internal Func<byte[], (int, byte[]?, string?)>? TreePathWith { get; set; }

        internal Func<byte[], (int, byte[]?, string?)>? TreeBytesWith { get; set; }

        internal Func<byte[], byte[], (int, byte[]?, string?)>? SheetPathWith { get; set; }

        internal Func<byte[], byte[], (int, byte[]?, string?)>? SheetBytesWith { get; set; }

        internal override (int Status, byte[]? Json, string? Message) TreePath(byte[] path) =>
            TreePathWith?.Invoke(path) ?? base.TreePath(path);

        internal override (int Status, byte[]? Json, string? Message) TreeBytes(ReadOnlySpan<byte> data) =>
            TreeBytesWith?.Invoke(data.ToArray()) ?? base.TreeBytes(data);

        internal override (int Status, byte[]? Json, string? Message) SheetPath(byte[] path, byte[] options) =>
            SheetPathWith?.Invoke(path, options) ?? base.SheetPath(path, options);

        internal override (int Status, byte[]? Json, string? Message) SheetBytes(ReadOnlySpan<byte> data, byte[] options) =>
            SheetBytesWith?.Invoke(data.ToArray(), options) ?? base.SheetBytes(data, options);
    }
}

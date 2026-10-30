namespace XlsxGraft.Tests;

public sealed class XlsxReaderTests
{
    private const string Data =
        "<row r=\"1\"><c r=\"A1\" t=\"s\"><v>1</v></c><c r=\"C1\"><v>3</v></c>"
        + "<c r=\"D1\" t=\"b\"><v>1</v></c></row>"
        + "<row r=\"3\"><c r=\"B3\" s=\"1\"><v>45322</v></c><c r=\"C3\" s=\"2\"><v>0.75</v></c>"
        + "<c r=\"D3\" s=\"3\"><v>45322.000005787037</v></c><c r=\"E3\" s=\"4\"><v>1.5</v></c>"
        + "<c r=\"F3\" t=\"e\"><v>#N/A</v></c><c r=\"G3\" t=\"inlineStr\"><is><t></t></is></c></row>"
        + "<row r=\"4\"/>";

    internal static readonly byte[] Bytes = Workbook.Xlsx(
        Workbook.Worksheet("Data", Data),
        Workbook.Chartsheet("Chart"),
        Workbook.Worksheet("Hidden", state: "veryHidden"));

    [Fact]
    public void OpenBytes_gives_each_row_with_its_number_and_its_values_from_column_A()
    {
        var (result, rows) = RowsOf(Bytes, book => book.Sheet("data"));

        Assert.True(result.Success);
        Assert.False(result.Failed);
        Assert.Null(result.Error);
        Assert.Empty(result.Diagnostics);
        Assert.Equal([1, 3, 4], rows.Select(row => row.Number));
        Assert.Equal(["b", null, 3.0, true], rows[0].Values);
        Assert.Equal(
            [null, new DateOnly(2024, 1, 31), new TimeOnly(18, 0), new DateTime(2024, 1, 31, 0, 0, 0, 500), TimeSpan.FromHours(36), new CellError("#N/A"), string.Empty],
            rows[1].Values);
        Assert.Empty(rows[2].Values);
        Assert.Equal(DateTimeKind.Unspecified, Assert.IsType<DateTime>(rows[1].Values[3]).Kind);
    }

    [Fact]
    public void Sheets_lists_the_sheets()
    {
        IReadOnlyList<SheetInfo> sheets;
        using (var book = XlsxReader.OpenBytes(Bytes))
        {
            sheets = book.Sheets();
        }

        Assert.Equal(
            [
                new SheetInfo(0, "Data", SheetKind.Worksheet, SheetState.Visible),
                new SheetInfo(1, "Chart", SheetKind.Chartsheet, SheetState.Visible),
                new SheetInfo(2, "Hidden", SheetKind.Worksheet, SheetState.VeryHidden),
            ],
            sheets);
    }

    [Fact]
    public void Each_enumeration_reads_the_sheet_again_from_its_start()
    {
        List<int> numbers;
        using (var book = XlsxReader.OpenBytes(Bytes))
        {
            var rows = book.Sheet().Rows();
            numbers = [rows.First().Number, .. rows.Select(row => row.Number)];
            Assert.Equal([1, 1, 3, 4], numbers);
        }
    }

    [Fact]
    public void OpenBytes_copies_the_bytes_when_called()
    {
        var data = Bytes.ToArray();
        using var book = XlsxReader.OpenBytes(data);
        Array.Clear(data);

        Assert.Equal(3, book.Sheets().Count);
    }

    [Theory]
    [InlineData("Missing", "sheet_not_found", "no sheet is named \"Missing\"")]
    [InlineData(9, "sheet_not_found", "there is no sheet at index 9: the workbook has 3, from index 0")]
    [InlineData("Chart", "sheet_not_worksheet", "sheet \"Chart\" is a chartsheet, which holds no cells")]
    [InlineData(1, "sheet_not_worksheet", "sheet \"Chart\" is a chartsheet, which holds no cells")]
    public void A_refused_sheet_fails_reading_no_rows(object which, string code, string message)
    {
        var (result, rows) = RowsOf(Bytes, book => which is string name ? book.Sheet(name) : book.Sheet((int)which));

        Assert.False(result.Success);
        Assert.Empty(rows);
        Assert.Equal(StatusDiagnostic.Of(code, message), result.Error);
    }

    [Fact]
    public void A_refused_sheet_goes_on_with_the_block_and_stays_the_error()
    {
        var book = XlsxReader.OpenBytes(Bytes);
        using (book)
        {
            Assert.Empty(book.Sheet("Missing").Rows());
            Assert.Equal(3, book.Sheet("Data").Rows().Count());
        }

        Assert.Equal("sheet_not_found", book.Result.Error?.Code);
    }

    [Fact]
    public void A_sheet_whose_part_is_not_known_fails_with_sheet_unreadable()
    {
        var (result, _) = RowsOf(Workbook.Xlsx(Workbook.Unrelated("Lost")), book => book.Sheet("Lost"));

        Assert.Equal(StatusDiagnostic.Of("sheet_unreadable", "sheet \"Lost\" cannot be read; the diagnostics say why"), result.Error);
        Assert.NotEmpty(result.Diagnostics);
    }

    [Fact]
    public void Two_sheets_of_the_name_fail_with_sheet_ambiguous()
    {
        var twice = Workbook.Xlsx(Workbook.Worksheet("Data"), Workbook.Worksheet("DATA"));
        var (result, _) = RowsOf(twice, book => book.Sheet("data"));

        Assert.Equal("more than one sheet is named \"data\", letter case aside", result.Error?.Message);
        Assert.Contains("sheet_name_duplicate", result.Diagnostics.Select(d => d.Code));
    }

    [Fact]
    public void A_sheet_name_that_is_not_Unicode_fails_with_invalid_arg()
    {
        var (result, rows) = RowsOf(Bytes, book => book.Sheet("\udcff"));

        Assert.Empty(rows);
        Assert.Equal("invalid_arg", result.Error?.Code);
    }

    [Fact]
    public void Bytes_that_are_no_workbook_fail_with_workbook_unreadable_listing_no_sheets()
    {
        var book = XlsxReader.OpenBytes("not a zip"u8);
        using (book)
        {
            Assert.Empty(book.Sheets());
            Assert.Empty(book.Sheet().Rows());
        }

        Assert.Equal("workbook_unreadable", book.Result.Error?.Code);
        Assert.StartsWith("the workbook cannot be read", book.Result.Error?.Message, StringComparison.Ordinal);
        Assert.NotEmpty(book.Result.Diagnostics);
    }

    [Fact]
    public void A_problem_that_stops_the_reading_fails_without_an_error()
    {
        const string twice = "<row r=\"1\"><c r=\"A1\"><v>1</v></c></row><row r=\"1\"/>";
        var (result, rows) = RowsOf(Workbook.Xlsx(Workbook.Worksheet("Data", twice)), book => book.Sheet());

        Assert.Equal([1], rows.Select(row => row.Number));
        Assert.False(result.Success);
        Assert.Null(result.Error);
        var first = result.Diagnostics[0];
        Assert.Equal(("cell_order_invalid", "error"), (first.Code, first.Severity));
        Assert.EndsWith("sheet1.xml", first.Location.GetProperty("part").GetString(), StringComparison.Ordinal);
    }

    [Fact]
    public void An_exception_the_block_throws_propagates_the_book_closed_first()
    {
        Book? book = null;
        void Block()
        {
            using (book = XlsxReader.OpenBytes(Bytes))
            {
                throw new InvalidOperationException("boom");
            }
        }

        var error = Assert.Throws<InvalidOperationException>(Block);

        Assert.Equal("boom", error.Message);
        Assert.NotNull(book);
        Assert.True(book.Result.Success);
        Assert.Throws<ObjectDisposedException>(book.Sheets);
        Assert.Throws<ObjectDisposedException>(() => book.Sheet("Data"));
        Assert.Throws<ObjectDisposedException>(() => book.Sheet(0));
    }

    [Fact]
    public void A_row_asked_for_after_the_book_is_closed_throws()
    {
        IEnumerable<Row> rows;
        IEnumerator<Row> started;
        using (var book = XlsxReader.OpenBytes(Bytes))
        {
            rows = book.Sheet().Rows();
            started = rows.GetEnumerator();
            Assert.True(started.MoveNext());
        }

        Assert.Throws<ObjectDisposedException>(() => rows.First());
        Assert.Throws<ObjectDisposedException>(() => started.MoveNext());
        started.Dispose();
    }

    [Fact]
    public void Result_throws_before_the_book_is_closed()
    {
        using var book = XlsxReader.OpenBytes(Bytes);

        var error = Assert.Throws<InvalidOperationException>(() => book.Result);
        Assert.Contains("once the book is closed", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Sheet_throws_for_a_null_name_or_an_index_below_0()
    {
        using var book = XlsxReader.OpenBytes(Bytes);

        Assert.Throws<ArgumentNullException>(() => book.Sheet(null!));
        Assert.Throws<ArgumentOutOfRangeException>(() => book.Sheet(-1));
    }

    [Fact]
    public void A_second_dispose_does_nothing()
    {
        var book = XlsxReader.OpenBytes(Bytes);
        book.Dispose();
        book.Dispose();

        Assert.True(book.Result.Success);
    }

    [Fact]
    public void Open_reads_the_workbook_at_a_path()
    {
        var path = Path.Combine(Path.GetTempPath(), $"{Guid.NewGuid()}.xlsx");
        File.WriteAllBytes(path, Bytes);
        try
        {
            var book = XlsxReader.Open(path);
            using (book)
            {
                Assert.Equal(3, book.Sheet().Rows().Count());
            }

            Assert.True(book.Result.Success);
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void A_file_that_cannot_be_opened_fails_with_io_saying_why()
    {
        var book = XlsxReader.Open("/nowhere/book.xlsx");
        using (book)
        {
            Assert.Empty(book.Sheets());
        }

        Assert.False(book.Result.Success);
        Assert.Empty(book.Result.Diagnostics);
        Assert.Equal("io", book.Result.Error?.Code);
        Assert.Contains("nowhere", book.Result.Error?.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void A_path_that_is_not_Unicode_fails_with_invalid_arg()
    {
        var book = XlsxReader.Open("/tmp/\udcff.xlsx");
        book.Dispose();

        Assert.Equal("invalid_arg", book.Result.Error?.Code);
    }

    [Fact]
    public void Open_takes_the_library_argument_when_XLSX_GRAFT_LIBRARY_is_unset()
    {
        XlsxReader.Open("/nowhere.xlsx", Repo.EngineLibrary, _ => null).Dispose();
        XlsxReader.OpenBytes(Bytes, Repo.EngineLibrary, _ => null).Dispose();

        Assert.Throws<LibraryNotFoundException>(() => XlsxReader.Open("/nowhere.xlsx", "/elsewhere.so", _ => null));
        Assert.Throws<LibraryNotFoundException>(() => XlsxReader.OpenBytes(Bytes, "/elsewhere.so", _ => null));
        Assert.Throws<ArgumentNullException>(() => XlsxReader.Open(null!));
    }

    [Fact]
    public void Two_enumerators_read_in_turns()
    {
        using var book = XlsxReader.OpenBytes(Bytes);

        var pairs = book.Sheet().Rows().Zip(book.Sheet("Data").Rows()).ToList();

        Assert.Equal(3, pairs.Count);
        Assert.All(pairs, pair => Assert.Equal(pair.First, pair.Second));
        Assert.Equal(pairs[0].First.GetHashCode(), pairs[0].Second.GetHashCode());
        Assert.NotEqual(pairs[0].First, pairs[1].First);
        Assert.NotEqual(pairs[0].First, pairs[0].First with { Number = 2 });
        Assert.False(pairs[0].First.Equals(null));
    }

    [Fact]
    public async Task Threads_read_one_book_at_once()
    {
        var book = XlsxReader.OpenBytes(Bytes);
        int[] counts;
        using (book)
        {
            counts = await Task.WhenAll(Enumerable.Range(0, 80).Select(_ => Task.Run(() => book.Sheet().Rows().Count())));
        }

        Assert.All(counts, count => Assert.Equal(3, count));
        Assert.True(book.Result.Success);
    }

    private static (Result Result, List<Row> Rows) RowsOf(byte[] data, Func<Book, Sheet> which)
    {
        var book = XlsxReader.OpenBytes(data);
        List<Row> rows;
        using (book)
        {
            rows = [.. which(book).Rows()];
        }

        return (book.Result, rows);
    }
}

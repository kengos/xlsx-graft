// What the engine cannot be made to do from a workbook: panic, fail the host
// while lending the diagnostics, answer a status or lend a value newer than
// the package. Each test reads a real workbook through a Capi of which one
// call or more is replaced.

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace XlsxGraft.Tests;

public sealed class BookTests
{
    private static readonly byte[] Bytes = Workbook.Xlsx(Workbook.Worksheet("Data", "<row r=\"1\"/><row r=\"2\"/>"));

    [Fact]
    public void A_panic_reading_a_row_fails_with_panic_reading_no_diagnostics()
    {
        // The engine refuses every call after a panic.
        var capi = new FakeCapi { RowsNextWith = _ => (2, 0, IntPtr.Zero, 0), BookDiagnosticsWith = _ => (2, []) };

        var (result, rows) = Read(capi);

        Assert.Empty(rows);
        Assert.False(result.Success);
        Assert.Empty(result.Diagnostics);
        Assert.Equal("panic", result.Error?.Code);
    }

    [Fact]
    public void A_status_the_package_does_not_know_fails_with_unknown_status()
    {
        var (result, _) = Read(new FakeCapi { RowsOpenWith = (_, _) => 42 });

        Assert.Equal("unknown_status", result.Error?.Code);
        Assert.Contains("42", result.Error?.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void A_value_of_a_kind_newer_than_the_package_stops_the_sheet_with_unknown_value()
    {
        var value = Marshal.AllocHGlobal(Unsafe.SizeOf<CValue>());
        try
        {
            Marshal.StructureToPtr(new CValue { Kind = 9 }, value, fDeleteOld: false);
            var capi = new FakeCapi { RowsNextWith = _ => (0, 1, value, 1) };
            var book = Open(capi);
            List<Row> rows;
            using (book)
            {
                rows = [.. book.Sheet().Rows()];

                // A refused sheet after it does not replace it.
                Assert.Empty(book.Sheet("Missing").Rows());
            }

            Assert.Empty(rows);
            Assert.Equal(
                StatusDiagnostic.Of("unknown_value", "row 1: the engine returned a value of kind 9, which this package does not know"),
                book.Result.Error);
        }
        finally
        {
            Marshal.FreeHGlobal(value);
        }
    }

    [Fact]
    public void Diagnostics_that_cannot_be_lent_fail_with_io_saying_why()
    {
        var capi = new FakeCapi { BookDiagnosticsWith = _ => (3, []), ErrorMessageWith = () => "the disk is full" };

        var (result, rows) = Read(capi);

        Assert.Equal([1, 2], rows);
        Assert.Equal(StatusDiagnostic.Of("io", "the disk is full"), result.Error);
    }

    [Theory]
    [InlineData("{}")]
    [InlineData("[]")]
    [InlineData("not json")]
    [InlineData("{\"error\": {}}")]
    [InlineData("{\"has_errors\": false, \"error\": {\"code\": null, \"message\": \"m\"}, \"diagnostics\": []}")]
    [InlineData("{\"has_errors\": false, \"error\": {\"code\": \"io\", \"message\": \"\xff\"}, \"diagnostics\": []}")]
    public void Diagnostics_not_of_the_shape_fail_with_diagnostics_unreadable(string json)
    {
        // Each char below 256 as one byte, so that "\xff" is a byte that is not UTF-8.
        var capi = new FakeCapi { BookDiagnosticsWith = _ => (0, Encoding.Latin1.GetBytes(json)) };

        var (result, _) = Read(capi);

        Assert.Equal("diagnostics_unreadable", result.Error?.Code);
        Assert.StartsWith("the engine's diagnostics could not be read: ", result.Error?.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void A_panic_freeing_a_rows_handle_fails_with_panic()
    {
        var (result, rows) = Read(new FakeCapi { RowsCloseWith = _ => 2 });

        Assert.Equal([1, 2], rows);
        Assert.Equal("panic", result.Error?.Code);
    }

    [Fact]
    public void A_sheet_of_a_kind_or_state_newer_than_the_package_is_listed_as_unknown()
    {
        using var book = Open(new FakeCapi { BookSheetsWith = _ => (0, [("New", 7, 9)]) });

        var sheet = Assert.Single(book.Sheets());

        Assert.Equal((SheetKind.Unknown, SheetState.Unknown), (sheet.Kind, sheet.State));
    }

    [Fact]
    public void An_exception_after_the_open_closes_the_handle_it_made()
    {
        var capi = new FakeCapi();
        var handle = IntPtr.Zero;

        Assert.Throws<InsufficientExecutionStackException>(() => new Book(capi, (Capi c, ref IntPtr book) =>
        {
            c.BookOpenBytes(Bytes, ref book);
            handle = book;
            throw new InsufficientExecutionStackException();
        }));

        Assert.NotEqual(IntPtr.Zero, handle);
        Assert.Equal([handle], capi.BookCloses);
    }

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public void A_close_an_exception_cuts_short_fails_with_interrupted(bool inDiagnostics)
    {
        var capi = new FakeCapi();
        if (inDiagnostics)
        {
            capi.BookDiagnosticsWith = _ => throw new InsufficientExecutionStackException();
        }
        else
        {
            capi.BookCloseWith = _ => throw new InsufficientExecutionStackException();
        }

        // Disposed again at the end, which does nothing: it is closed.
        using var book = Open(capi);
        Assert.Single(book.Sheet().Rows().Take(1));

        Assert.Throws<InsufficientExecutionStackException>(book.Dispose);
        Assert.False(book.Result.Success);
        Assert.Empty(book.Result.Diagnostics);
        Assert.Equal("interrupted", book.Result.Error?.Code);
        Assert.Equal(inDiagnostics ? 1 : 0, capi.BookCloses.Count);
    }

    [Fact]
    public void An_exception_after_a_rows_open_leaves_the_handle_for_the_book_to_free()
    {
        var handle = IntPtr.Zero;
        var capi = new FakeCapi();
        capi.RowsOpenWith = (book, slot) =>
        {
            capi.Base.RowsOpen(book, 0, ref slot.Handle);
            handle = slot.Handle;
            throw new InsufficientExecutionStackException();
        };

        using (var book = Open(capi))
        {
            Assert.Throws<InsufficientExecutionStackException>(() => book.Sheet().Rows().First());
        }

        Assert.NotEqual(IntPtr.Zero, handle);
        Assert.Equal([handle], capi.RowsCloses);
    }

    [Fact]
    public void An_enumerator_disposed_midway_frees_its_rows_once()
    {
        var capi = new FakeCapi();
        var book = Open(capi);
        using (book)
        {
            Assert.Single(book.Sheet().Rows().Take(1));
            Assert.Single(capi.RowsCloses);
        }

        Assert.Single(capi.RowsCloses);
        Assert.True(book.Result.Success);
    }

    [Fact]
    public void An_enumerator_left_open_is_freed_by_the_books_close()
    {
        var capi = new FakeCapi();
        var book = Open(capi);
        IEnumerator<Row> rows;
        using (book)
        {
            rows = book.Sheet().Rows().GetEnumerator();
            Assert.True(rows.MoveNext());
            Assert.Empty(capi.RowsCloses);
        }

        Assert.Single(capi.RowsCloses);
        Assert.True(book.Result.Success);

        // Its own dispose after frees nothing again.
        rows.Dispose();
        Assert.Single(capi.RowsCloses);
    }

    [Fact]
    public void Diagnostics_are_equal_by_every_field_and_hash_by_code_severity_and_message()
    {
        var one = Diagnostics(Workbook.Xlsx(Workbook.Unrelated("Lost")));
        var other = Diagnostics(Workbook.Xlsx(Workbook.Unrelated("Lost")));
        var made = StatusDiagnostic.Of("io", "m");
        var fixable = made with { Fix = JsonElement.Parse("\"do\"") };

        Assert.NotEmpty(one);
        Assert.Equal(one, other);
        Assert.Equal(one[0].GetHashCode(), other[0].GetHashCode());
        Assert.False(made.Equals(null));
        Assert.NotEqual(made, fixable);
        Assert.NotEqual(fixable, made);
        Assert.Equal(fixable, fixable with { });
        Assert.NotEqual(made, made with { Args = JsonElement.Parse("{\"a\": 1}") });
        Assert.Equal(made.GetHashCode(), fixable.GetHashCode());
    }

    [Fact]
    public void A_value_is_48_bytes_and_a_sheet_24()
    {
        Assert.Equal((48, 24), (Unsafe.SizeOf<CValue>(), Unsafe.SizeOf<CSheet>()));
    }

    private static Book Open(Capi capi) => new(capi, (Capi c, ref IntPtr book) => c.BookOpenBytes(Bytes, ref book));

    // Reads the first sheet: the result and the rows' numbers.
    private static (Result Result, List<int> Rows) Read(Capi capi)
    {
        var book = Open(capi);
        List<int> rows;
        using (book)
        {
            rows = [.. book.Sheet().Rows().Select(row => row.Number)];
        }

        return (book.Result, rows);
    }

    private static IReadOnlyList<Diagnostic> Diagnostics(byte[] data)
    {
        var book = XlsxReader.OpenBytes(data);
        book.Dispose();
        return book.Result.Diagnostics;
    }

    /// <summary>A Capi over the engine library whose calls a test may replace, counting the closes.</summary>
    private sealed class FakeCapi : Capi
    {
        internal FakeCapi()
            : base(Library.Open(Repo.EngineLibrary, _ => null))
        {
            Base = new Capi(Library.Open(Repo.EngineLibrary, _ => null));
        }

        // The same library, calls not replaced.
        internal Capi Base { get; }

        internal Func<IntPtr, (int, uint, IntPtr, nuint)>? RowsNextWith { get; set; }

        internal Func<IntPtr, Book.RowsSlot, int>? RowsOpenWith { get; set; }

        internal Func<IntPtr, int>? RowsCloseWith { get; set; }

        internal Func<IntPtr, (int, byte[])>? BookDiagnosticsWith { get; set; }

        internal Func<IntPtr, (int, IReadOnlyList<(string, uint, uint)>)>? BookSheetsWith { get; set; }

        internal Func<IntPtr, int>? BookCloseWith { get; set; }

        internal Func<string>? ErrorMessageWith { get; set; }

        internal List<IntPtr> BookCloses { get; } = [];

        internal List<IntPtr> RowsCloses { get; } = [];

        internal override (int Status, uint Number, IntPtr Values, nuint Count) RowsNext(IntPtr rows) =>
            RowsNextWith?.Invoke(rows) ?? base.RowsNext(rows);

        internal override int RowsOpen(IntPtr book, nuint index, ref IntPtr rows)
        {
            if (RowsOpenWith is null)
            {
                return base.RowsOpen(book, index, ref rows);
            }

            var slot = new Book.RowsSlot();
            try
            {
                return RowsOpenWith(book, slot);
            }
            finally
            {
                rows = slot.Handle;
            }
        }

        internal override int RowsClose(IntPtr rows)
        {
            RowsCloses.Add(rows);
            return RowsCloseWith?.Invoke(rows) ?? base.RowsClose(rows);
        }

        internal override (int Status, byte[] Json) BookDiagnostics(IntPtr book) =>
            BookDiagnosticsWith?.Invoke(book) ?? base.BookDiagnostics(book);

        internal override (int Status, IReadOnlyList<(string Name, uint Kind, uint State)> Sheets) BookSheets(IntPtr book) =>
            BookSheetsWith?.Invoke(book) ?? base.BookSheets(book);

        internal override int BookClose(IntPtr book)
        {
            var status = BookCloseWith?.Invoke(book) ?? base.BookClose(book);
            BookCloses.Add(book);
            return status;
        }

        internal override string ErrorMessage() => ErrorMessageWith?.Invoke() ?? base.ErrorMessage();
    }
}

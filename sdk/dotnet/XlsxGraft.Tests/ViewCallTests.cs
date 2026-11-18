using System.Text;

namespace XlsxGraft.Tests;

public sealed class ViewCallTests
{
    private static readonly ViewCall Call = new(Library.Open(Repo.EngineLibrary, _ => null));
    private static readonly string Merged = VerifyCallTests.Fixture("merged.xlsx");

    private static string Text(byte[]? json) => Encoding.UTF8.GetString(json!);

    [Fact]
    public void TreePath_gives_the_status_and_the_view_JSON_copied_out()
    {
        var (status, json, message) = Call.TreePath(Encoding.UTF8.GetBytes(Merged));

        Assert.Equal(StatusDiagnostic.Ok, status);
        Assert.StartsWith("{\"ok\":true", Text(json), StringComparison.Ordinal);
        Assert.Null(message);
    }

    [Fact]
    public void TreeBytes_gives_the_status_and_the_view_JSON_copied_out()
    {
        var (status, json, _) = Call.TreeBytes(File.ReadAllBytes(Merged));

        Assert.Equal(StatusDiagnostic.Ok, status);
        Assert.Contains("\"/xl/workbook.xml\"", Text(json), StringComparison.Ordinal);
    }

    [Fact]
    public void TreeBytes_takes_empty_data_and_gives_a_view_that_is_not_ok()
    {
        var (status, json, _) = Call.TreeBytes([]);

        Assert.Equal(StatusDiagnostic.Ok, status);
        Assert.StartsWith("{\"ok\":false", Text(json), StringComparison.Ordinal);
    }

    [Fact]
    public void SheetPath_gives_the_cells_the_options_ask_for()
    {
        var (status, json, _) = Call.SheetPath(Encoding.UTF8.GetBytes(Merged), "{\"maxRows\":2}"u8.ToArray());

        Assert.Equal(StatusDiagnostic.Ok, status);
        Assert.Contains("\"shown\":\"A1:E2\"", Text(json), StringComparison.Ordinal);
    }

    [Fact]
    public void SheetBytes_gives_the_cells_the_options_ask_for_and_takes_empty_options()
    {
        var data = File.ReadAllBytes(Merged);

        var (status, json, _) = Call.SheetBytes(data, "{\"maxColumns\":2}"u8.ToArray());
        var (defaults, all, _) = Call.SheetBytes(data, []);

        Assert.Equal(StatusDiagnostic.Ok, status);
        Assert.Contains("\"shown\":\"A1:B8\"", Text(json), StringComparison.Ordinal);
        Assert.Equal(StatusDiagnostic.Ok, defaults);
        Assert.Contains("\"shown\":\"A1:E8\"", Text(all), StringComparison.Ordinal);
    }

    [Fact]
    public void A_value_the_engine_refuses_gives_request_invalid_and_its_message()
    {
        var (status, json, message) = Call.SheetBytes([], "{\"maxRows\":0}"u8.ToArray());

        Assert.Equal(StatusDiagnostic.RequestInvalid, status);
        Assert.Null(json);
        Assert.Contains("maxRows", message, StringComparison.Ordinal);
    }

    [Fact]
    public void Options_that_are_not_JSON_give_invalid_arg_and_neither_JSON_nor_message()
    {
        Assert.Equal((StatusDiagnostic.InvalidArg, null, null), Call.SheetPath("a.xlsx"u8.ToArray(), "not json"u8.ToArray()));
        Assert.Equal((StatusDiagnostic.InvalidArg, null, null), Call.SheetBytes([], "not json"u8.ToArray()));
    }

    [Fact]
    public void A_path_that_is_not_UTF_8_gives_invalid_arg()
    {
        Assert.Equal((StatusDiagnostic.InvalidArg, null, null), Call.TreePath([0xFF]));
    }
}

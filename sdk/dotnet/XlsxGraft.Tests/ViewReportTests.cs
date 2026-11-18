using System.Text;

namespace XlsxGraft.Tests;

public sealed class ViewReportTests
{
    private const string Tree =
        "{\"ok\":true,\"error\":null,\"diagnostics\":[],\"complete\":false,\"relationshipsKnown\":true,"
        + "\"relationships\":[{\"id\":\"rId1\",\"type\":\"t\",\"target\":{\"part\":\"/a.xml\"}}],"
        + "\"parts\":[{\"name\":\"/a.xml\",\"contentType\":null,\"contentTypeKnown\":false,\"size\":3,"
        + "\"compressedSize\":2,\"readable\":true,\"referenced\":null,\"relationshipsKnown\":false,"
        + "\"relationships\":[{\"id\":\"rId2\",\"type\":\"x\",\"target\":{\"external\":\"http://e/\"}}],"
        + "\"later\":1}],\"laterKey\":[]}";

    private const string Record =
        "{\"code\":\"part_denied\",\"severity\":\"error\",\"message\":\"m\",\"args\":{},\"location\":{},"
        + "\"decidedBy\":null,\"fix\":null}";

    private static Result<Tree> ReadTree(string json) =>
        ViewReport.Call(0, Encoding.UTF8.GetBytes(json), null, ViewContent.Tree);

    private static Result<Grid> ReadGrid(string json) =>
        ViewReport.Call(0, Encoding.UTF8.GetBytes(json), null, ViewContent.Grid);

    private static string GridJson(string cells, string ok = "true", string error = "null", string records = "", string? sheet = null) =>
        $"{{\"ok\":{ok},\"error\":{error},\"diagnostics\":[{records}],"
        + $"\"sheet\":{sheet ?? "{\"index\":1,\"name\":\"S\",\"kind\":\"worksheet\",\"state\":\"hidden\",\"later\":2}"},"
        + "\"dimension\":null,\"used\":\"A1:A1\",\"shown\":\"A1:A1\","
        + "\"truncated\":{\"rows\":false,\"columns\":true,\"merges\":false},\"complete\":true,"
        + $"\"rows\":[{{\"row\":1,\"cells\":[{cells}]}}],\"merges\":[\"A1:B2\"]}}";

    [Fact]
    public void A_tree_is_read_and_a_key_it_does_not_know_is_ignored()
    {
        var result = ReadTree(Tree);

        Assert.True(result.Success);
        Assert.Null(result.Error);
        Assert.Empty(result.Diagnostics);
        var tree = result.Value!;
        Assert.False(tree.Complete);
        Assert.True(tree.RelationshipsKnown);
        Assert.Equivalent(new[] { new TreeRelationship("rId1", "t", new TreeTarget("/a.xml", null)) }, tree.Relationships);
        var part = Assert.Single(tree.Parts);
        Assert.Equal("/a.xml", part.Name);
        Assert.Null(part.ContentType);
        Assert.False(part.ContentTypeKnown);
        Assert.Equal(3UL, part.Size);
        Assert.Equal(2UL, part.CompressedSize);
        Assert.True(part.Readable);
        Assert.Null(part.Referenced);
        Assert.False(part.RelationshipsKnown);
        Assert.Equivalent(new[] { new TreeRelationship("rId2", "x", new TreeTarget(null, "http://e/")) }, part.Relationships);
    }

    [Fact]
    public void A_part_that_is_referenced_and_has_a_content_type_is_read()
    {
        var json = Tree.Replace("\"contentType\":null", "\"contentType\":\"text/xml\"", StringComparison.Ordinal)
            .Replace("\"referenced\":null", "\"referenced\":true", StringComparison.Ordinal)
            .Replace("\"size\":3", "\"size\":18446744073709551615", StringComparison.Ordinal);

        var part = Assert.Single(ReadTree(json).Value!.Parts);

        Assert.Equal("text/xml", part.ContentType);
        Assert.True(part.Referenced);
        Assert.Equal(ulong.MaxValue, part.Size);
    }

    [Fact]
    public void Each_type_of_cell_is_read_as_the_JSON_gives_it_and_a_cell_without_truncated_is_whole()
    {
        var cells = string.Join(
            ",",
            "{\"ref\":\"A1\",\"type\":\"number\",\"value\":2.0}",
            "{\"ref\":\"B1\",\"type\":\"elapsed\",\"value\":90000}",
            "{\"ref\":\"C1\",\"type\":\"boolean\",\"value\":true}",
            "{\"ref\":\"D1\",\"type\":\"boolean\",\"value\":false}",
            "{\"ref\":\"E1\",\"type\":\"error\",\"value\":\"#N/A\"}",
            "{\"ref\":\"F1\",\"type\":\"number\",\"value\":1.5}",
            "{\"ref\":\"G1\",\"type\":\"string\",\"value\":\"cut\",\"truncated\":true,\"later\":1}");

        var row = Assert.Single(ReadGrid(GridJson(cells)).Value!.Rows);

        Assert.Equal(new object[] { 2.0, 90000L, true, false, "#N/A", 1.5, "cut" }, row.Cells.Select(c => c.Value));
        Assert.Equal([false, false, false, false, false, false, true], row.Cells.Select(c => c.Truncated));
        Assert.IsType<double>(row.Cells[0].Value);
        Assert.IsType<long>(row.Cells[1].Value);
    }

    [Fact]
    public void The_sheet_the_flags_and_the_merges_of_a_grid_are_read()
    {
        var grid = ReadGrid(GridJson(string.Empty)).Value!;

        Assert.Equal(new SheetInfo(1, "S", SheetKind.Worksheet, SheetState.Hidden), grid.Sheet);
        Assert.Null(grid.Dimension);
        Assert.Equal("A1:A1", grid.Used);
        Assert.Equal("A1:A1", grid.Shown);
        Assert.Equal(new GridTruncated(Rows: false, Columns: true, Merges: false), grid.Truncated);
        Assert.True(grid.Complete);
        Assert.Equal(1, Assert.Single(grid.Rows).Row);
        Assert.Equal(["A1:B2"], grid.Merges);
    }

    [Theory]
    [InlineData("worksheet", "visible", SheetKind.Worksheet, SheetState.Visible)]
    [InlineData("chartsheet", "hidden", SheetKind.Chartsheet, SheetState.Hidden)]
    [InlineData("dialogsheet", "veryHidden", SheetKind.Dialogsheet, SheetState.VeryHidden)]
    [InlineData("macrosheet", "visible", SheetKind.Macrosheet, SheetState.Visible)]
    [InlineData("unknown", "visible", SheetKind.Unknown, SheetState.Visible)]
    [InlineData("newer", "newer", SheetKind.Unknown, SheetState.Unknown)]
    public void The_kind_and_the_state_of_a_sheet_are_read_as_enumerations_and_a_newer_one_is_Unknown(string kind, string state, SheetKind expectedKind, SheetState expectedState)
    {
        var sheet = $"{{\"index\":0,\"name\":\"S\",\"kind\":\"{kind}\",\"state\":\"{state}\"}}";

        Assert.Equal(new SheetInfo(0, "S", expectedKind, expectedState), ReadGrid(GridJson(string.Empty, sheet: sheet)).Value!.Sheet);
    }

    [Fact]
    public void A_null_sheet_is_a_grid_with_no_sheet_and_the_value_of_a_failure_is_kept()
    {
        var result = ReadGrid(GridJson(string.Empty, ok: "false", error: "{\"code\":\"io\",\"message\":\"gone\"}", records: Record, sheet: "null"));

        Assert.False(result.Success);
        Assert.Equal(StatusDiagnostic.Of("io", "gone"), result.Error);
        Assert.Equal(["part_denied"], result.Diagnostics.Select(d => d.Code));
        Assert.NotNull(result.Value);
        Assert.Null(result.Value.Sheet);
    }

    [Fact]
    public void A_view_whose_ok_is_true_needs_no_error_key()
    {
        var json = Tree.Replace("\"error\":null,", string.Empty, StringComparison.Ordinal);
        var result = ReadTree(json);

        Assert.DoesNotContain("\"error\"", json, StringComparison.Ordinal);
        Assert.True(result.Success);
        Assert.Null(result.Error);
        Assert.NotNull(result.Value);
    }

    [Fact]
    public void A_view_that_is_not_ok_and_has_no_error_fails_with_its_value_and_no_error()
    {
        var result = ReadTree(Tree.Replace("\"ok\":true", "\"ok\":false", StringComparison.Ordinal));

        Assert.False(result.Success);
        Assert.Null(result.Error);
        Assert.NotNull(result.Value);
    }

    [Fact]
    public void Request_invalid_throws_ArgumentException_with_the_message_and_with_a_fixed_one_when_there_is_none()
    {
        var error = Assert.Throws<ArgumentException>(() => ViewReport.Call(11, null, "maxRows 0 is not 1 to 10000", ViewContent.Grid));
        var none = Assert.Throws<ArgumentException>(() => ViewReport.Call(11, null, null, ViewContent.Grid));
        var empty = Assert.Throws<ArgumentException>(() => ViewReport.Call(11, null, string.Empty, ViewContent.Grid));

        Assert.Equal("maxRows 0 is not 1 to 10000", error.Message);
        Assert.Equal("the engine refused a value of the options (status 11) and gave no reason", none.Message);
        Assert.Equal(none.Message, empty.Message);
    }

    [Fact]
    public void Invalid_arg_throws_because_only_a_call_this_package_built_wrongly_gets_it()
    {
        var error = Assert.Throws<XlsxGraftException>(() => ViewReport.Call(4, null, null, ViewContent.Grid));

        Assert.Equal("the engine refused a call this package built (status 4): a bug of the package", error.Message);
    }

    [Fact]
    public void A_status_other_than_ok_request_invalid_and_invalid_arg_fails_with_an_error_and_no_value()
    {
        var panic = ViewReport.Call(2, null, null, ViewContent.Grid);
        var unknown = ViewReport.Call(99, null, null, ViewContent.Tree);

        Assert.False(panic.Success);
        Assert.Null(panic.Value);
        Assert.Empty(panic.Diagnostics);
        Assert.Equal("panic", panic.Error?.Code);
        Assert.Equal("unknown_status", unknown.Error?.Code);
    }

    [Theory]
    [InlineData("not json")]
    [InlineData("{}")]
    [InlineData("[]")]
    [InlineData("{\"ok\":true}")]
    public void JSON_that_is_not_a_view_fails_with_diagnostics_unreadable_and_no_value(string json)
    {
        var tree = ReadTree(json);
        var grid = ReadGrid(json);

        Assert.Equal("diagnostics_unreadable", tree.Error?.Code);
        Assert.Equal("diagnostics_unreadable", grid.Error?.Code);
        Assert.Null(tree.Value);
        Assert.Null(grid.Value);
        Assert.StartsWith("the engine's result could not be read: ", tree.Error?.Message, StringComparison.Ordinal);
    }

    [Theory]
    [InlineData("\"target\":{\"part\":\"/a.xml\"}", "\"target\":\"/a.xml\"")]
    [InlineData("\"size\":3", "\"size\":1.5")]
    [InlineData("\"size\":3", "\"size\":-1")]
    [InlineData("\"size\":3", "\"size\":\"3\"")]
    [InlineData("\"readable\":true", "\"readable\":1")]
    [InlineData("\"name\":\"/a.xml\"", "\"name\":null")]
    [InlineData("\"id\":\"rId1\"", "\"id\":null")]
    [InlineData("\"type\":\"t\"", "\"type\":null")]
    public void A_tree_of_the_wrong_types_fails_with_diagnostics_unreadable_and_no_value(string from, string to)
    {
        var result = ReadTree(Tree.Replace(from, to, StringComparison.Ordinal));

        Assert.Equal("diagnostics_unreadable", result.Error?.Code);
        Assert.Null(result.Value);
    }

    [Theory]
    [InlineData("\"merges\":[\"A1:B2\"]", "\"merges\":5")]
    [InlineData("\"merges\":[\"A1:B2\"]", "\"merges\":[null]")]
    [InlineData("\"row\":1", "\"row\":1.5")]
    [InlineData("\"index\":1", "\"index\":99999999999")]
    [InlineData("\"complete\":true", "\"complete\":\"yes\"")]
    public void A_grid_of_the_wrong_types_fails_with_diagnostics_unreadable_and_no_value(string from, string to)
    {
        var result = ReadGrid(GridJson(string.Empty).Replace(from, to, StringComparison.Ordinal));

        Assert.Equal("diagnostics_unreadable", result.Error?.Code);
        Assert.Null(result.Value);
    }

    [Theory]
    [InlineData("{\"ref\":\"A1\",\"type\":\"string\",\"value\":null}")]
    [InlineData("{\"ref\":\"A1\",\"type\":\"string\",\"value\":[]}")]
    [InlineData("{\"ref\":\"A1\",\"type\":null,\"value\":1}")]
    [InlineData("{\"ref\":null,\"type\":\"string\",\"value\":1}")]
    [InlineData("{\"ref\":\"A1\",\"type\":\"string\",\"value\":1,\"truncated\":1}")]
    public void A_cell_of_the_wrong_shape_fails_with_diagnostics_unreadable_and_no_value(string cell)
    {
        var result = ReadGrid(GridJson(cell));

        Assert.Equal("diagnostics_unreadable", result.Error?.Code);
        Assert.Null(result.Value);
    }

    [Fact]
    public void Bytes_that_are_not_UTF_8_fail_with_diagnostics_unreadable()
    {
        Assert.Equal("diagnostics_unreadable", ViewReport.Call(0, [0xFF, 0xFE], null, ViewContent.Tree).Error?.Code);
    }
}

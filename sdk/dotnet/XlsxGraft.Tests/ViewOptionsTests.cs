using System.Text;

namespace XlsxGraft.Tests;

public sealed class ViewOptionsTests
{
    private static string Json(string? sheet = null, int? sheetIndex = null, string? range = null, int? maxRows = null, int? maxColumns = null) =>
        Encoding.UTF8.GetString(ViewOptions.Build(sheet, sheetIndex, range, maxRows, maxColumns));

    [Fact]
    public void Nothing_given_is_an_empty_object()
    {
        Assert.Equal("{}", Json());
    }

    [Fact]
    public void Each_value_given_is_written_by_the_engines_key_and_the_rest_are_not_written()
    {
        Assert.Equal("{\"sheet\":\"Data\"}", Json(sheet: "Data"));
        Assert.Equal("{\"sheetIndex\":0}", Json(sheetIndex: 0));
        Assert.Equal("{\"range\":\"A1:H50\"}", Json(range: "A1:H50"));
        Assert.Equal("{\"maxRows\":20}", Json(maxRows: 20));
        Assert.Equal("{\"maxColumns\":0}", Json(maxColumns: 0));
        Assert.Equal(
            "{\"sheet\":\"S\",\"sheetIndex\":1,\"range\":\"A1\",\"maxRows\":2,\"maxColumns\":3}",
            Json("S", 1, "A1", 2, 3));
    }

    [Fact]
    public void A_string_is_escaped_as_JSON()
    {
        Assert.Equal("{\"sheet\":\"a\\u0022b\\\\\"}", Json(sheet: "a\"b\\"));
    }

    [Fact]
    public void A_sheet_or_a_range_that_is_not_Unicode_throws_ArgumentException_naming_the_argument()
    {
        var sheet = Assert.Throws<ArgumentException>(() => Json(sheet: "a\uD800"));
        var range = Assert.Throws<ArgumentException>(() => Json(range: "\uDC00"));

        Assert.Equal("sheet", sheet.ParamName);
        Assert.StartsWith("sheet: the string holds a lone surrogate, which UTF-8 cannot encode", sheet.Message, StringComparison.Ordinal);
        Assert.Equal("range", range.ParamName);
    }

    [Fact]
    public void Options_of_exactly_2_MiB_are_built_and_one_byte_more_throw_ArgumentException()
    {
        var overhead = ViewOptions.Build(null, null, "A", null, null).Length - 1;

        var exact = ViewOptions.Build(null, null, new string('A', ViewOptions.MaxBytes - overhead), null, null);
        var error = Assert.Throws<ArgumentException>(() => ViewOptions.Build(null, null, new string('A', ViewOptions.MaxBytes - overhead + 1), null, null));

        Assert.Equal(ViewOptions.MaxBytes, exact.Length);
        Assert.Equal($"the options are {ViewOptions.MaxBytes + 1} bytes of JSON; the engine takes 2097152", error.Message);
    }
}

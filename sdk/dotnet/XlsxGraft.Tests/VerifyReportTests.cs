using System.Text;

namespace XlsxGraft.Tests;

public sealed class VerifyReportTests
{
    private const string Record =
        "{\"code\":\"part_denied\",\"severity\":\"error\",\"message\":\"m\",\"args\":{},\"location\":{},"
        + "\"decidedBy\":null,\"fix\":null,\"origin\":\"x\"}";

    [Fact]
    public void A_result_whose_ok_is_true_is_a_success_with_the_diagnostics_it_holds()
    {
        var result = VerifyReport.Call(0, Json(ok: "true"));
        var warned = VerifyReport.Call(0, Json(ok: "true", records: Record.Replace("error", "warn", StringComparison.Ordinal)));

        Assert.True(result.Success);
        Assert.Null(result.Error);
        Assert.Empty(result.Diagnostics);
        Assert.True(warned.Success);
        Assert.Equal(["warn"], warned.Diagnostics.Select(d => d.Severity));
    }

    [Fact]
    public void A_result_whose_ok_is_false_fails_with_its_diagnostics_and_no_error()
    {
        var result = VerifyReport.Call(0, Json(ok: "false", records: Record));

        Assert.False(result.Success);
        Assert.Null(result.Error);
        Assert.Equal(["part_denied"], result.Diagnostics.Select(d => d.Code));
    }

    [Fact]
    public void The_io_error_of_a_result_is_the_error_of_the_failure()
    {
        var result = VerifyReport.Call(0, Json(ok: "false", error: "{\"code\":\"io\",\"message\":\"gone\"}"));

        Assert.False(result.Success);
        Assert.Empty(result.Diagnostics);
        Assert.Equal(StatusDiagnostic.Of("io", "gone"), result.Error);
    }

    [Fact]
    public void A_status_other_than_ok_and_invalid_arg_fails_with_an_error_naming_the_status()
    {
        Assert.Equal("panic", VerifyReport.Call(2, null).Error?.Code);
        Assert.Equal("null_arg", VerifyReport.Call(1, null).Error?.Code);
        Assert.Equal("unknown_status", VerifyReport.Call(99, null).Error?.Code);
        Assert.Empty(VerifyReport.Call(2, null).Diagnostics);
    }

    [Fact]
    public void Invalid_arg_throws_because_only_a_call_this_package_built_wrongly_gets_it()
    {
        var error = Assert.Throws<XlsxGraftException>(() => VerifyReport.Call(4, null));

        Assert.Equal("the engine refused a call this package built (status 4): a bug of the package", error.Message);
    }

    [Theory]
    [InlineData("not json")]
    [InlineData("{}")]
    [InlineData("[]")]
    [InlineData("{\"ok\":1,\"diagnostics\":[],\"error\":null}")]
    [InlineData("{\"ok\":true,\"diagnostics\":[1],\"error\":null}")]
    [InlineData("{\"ok\":false,\"diagnostics\":[],\"error\":{\"code\":null,\"message\":\"m\"}}")]
    [InlineData("{\"ok\":false,\"diagnostics\":[],\"error\":{\"code\":\"io\"}}")]
    public void JSON_that_is_not_a_result_fails_with_diagnostics_unreadable(string json)
    {
        var result = VerifyReport.Call(0, Encoding.UTF8.GetBytes(json));

        Assert.False(result.Success);
        Assert.Empty(result.Diagnostics);
        Assert.Equal("diagnostics_unreadable", result.Error?.Code);
        Assert.StartsWith("the engine's result could not be read: ", result.Error?.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Bytes_that_are_not_UTF_8_fail_with_diagnostics_unreadable()
    {
        Assert.Equal("diagnostics_unreadable", VerifyReport.Call(0, [0xFF, 0xFE]).Error?.Code);
    }

    private static byte[] Json(string ok, string error = "null", string records = "") =>
        Encoding.UTF8.GetBytes($"{{\"ok\":{ok},\"failOn\":\"error\",\"counts\":{{}},\"error\":{error},\"diagnostics\":[{records}]}}");
}

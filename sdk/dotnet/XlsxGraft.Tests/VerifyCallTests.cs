using System.Text;

namespace XlsxGraft.Tests;

public sealed class VerifyCallTests
{
    private static readonly VerifyCall Call = new(Library.Open(Repo.EngineLibrary, _ => null));

    [Fact]
    public void Path_gives_the_status_and_the_result_JSON_copied_out()
    {
        var (status, json) = Call.Path(Encoding.UTF8.GetBytes(Fixture("table.xlsx")), "null"u8.ToArray());

        Assert.Equal(StatusDiagnostic.Ok, status);
        Assert.StartsWith("{\"ok\":true", Encoding.UTF8.GetString(json!), StringComparison.Ordinal);
    }

    [Fact]
    public void Bytes_gives_the_status_and_the_result_JSON_copied_out()
    {
        var (status, json) = Call.Bytes(File.ReadAllBytes(Fixture("macro.xlsm")), "{\"rules\":{\"parts\":{\"allow\":[\"macro\"]}}}"u8.ToArray());

        Assert.Equal(StatusDiagnostic.Ok, status);
        Assert.StartsWith("{\"ok\":true", Encoding.UTF8.GetString(json!), StringComparison.Ordinal);
    }

    [Fact]
    public void Bytes_takes_empty_data_and_empty_options()
    {
        var (status, json) = Call.Bytes([], []);

        Assert.Equal(StatusDiagnostic.Ok, status);
        Assert.StartsWith("{\"ok\":false", Encoding.UTF8.GetString(json!), StringComparison.Ordinal);
    }

    [Fact]
    public void Policy_gives_the_result_of_checking_it()
    {
        var (status, json) = Call.Policy("{\"maxEntries\":0}"u8.ToArray());

        Assert.Equal(StatusDiagnostic.Ok, status);
        Assert.Contains("policy_invalid", Encoding.UTF8.GetString(json!), StringComparison.Ordinal);
    }

    [Fact]
    public void Options_that_are_not_JSON_give_invalid_arg_and_no_JSON()
    {
        Assert.Equal((StatusDiagnostic.InvalidArg, null), Call.Path("a.xlsx"u8.ToArray(), "not json"u8.ToArray()));
        Assert.Equal((StatusDiagnostic.InvalidArg, null), Call.Bytes([], "not json"u8.ToArray()));
    }

    [Fact]
    public void A_path_that_is_not_UTF_8_gives_invalid_arg_and_no_JSON()
    {
        Assert.Equal((StatusDiagnostic.InvalidArg, null), Call.Path([0xFF], []));
    }

    internal static string Fixture(string name) => Path.Combine(Repo.Root, "fixtures", name);
}

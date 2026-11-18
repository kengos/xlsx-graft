namespace XlsxGraft.Tests;

public sealed class PathInputTests
{
    private static string Run(string path, Func<Diagnostic, string>? unreadable = null) =>
        PathInput.Call(
            path,
            encoded => $"path:{System.Text.Encoding.UTF8.GetString(encoded)}",
            data => $"bytes:{data.Length}",
            unreadable ?? (error => $"unreadable:{error.Code}"));

    [Fact]
    public void A_path_of_Unicode_text_goes_as_its_UTF_8_bytes()
    {
        Assert.Equal("path:/a/b.xlsx", Run("/a/b.xlsx"));
        Assert.Equal("path:/a/é.xlsx", Run("/a/é.xlsx"));
    }

    [Fact]
    public void A_path_that_is_not_Unicode_goes_as_the_bytes_of_the_file()
    {
        var directory = Directory.CreateTempSubdirectory("xlsx-graft-");
        try
        {
            var path = Path.Combine(directory.FullName, "\uD800.xlsx");
            File.WriteAllBytes(path, [1, 2, 3]);

            Assert.Equal("bytes:3", Run(path));
        }
        finally
        {
            directory.Delete(recursive: true);
        }
    }

    [Fact]
    public void A_file_that_cannot_be_read_is_an_io_error_and_neither_call_is_made()
    {
        var made = Run("/nowhere/\uD800.xlsx", error => $"{error.Code}|{error.Severity}|{error.Message}");

        Assert.StartsWith("io|error|the file could not be read: ", made, StringComparison.Ordinal);
    }
}

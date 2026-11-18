// A path the caller gave, as the C ABI can take it: a path of Unicode text
// goes as its UTF-8 bytes, and a path that is not Unicode (a lone surrogate)
// goes as the bytes of the file this package reads, since the engine takes no
// path that is not UTF-8.

namespace XlsxGraft;

internal static class PathInput
{
    /// <summary>Runs the call that fits <paramref name="path"/>.</summary>
    /// <typeparam name="T">What the call returns.</typeparam>
    /// <param name="path">The workbook's path.</param>
    /// <param name="byPath">The call that takes the path as UTF-8 bytes.</param>
    /// <param name="byBytes">The call that takes the workbook's bytes.</param>
    /// <param name="unreadable">Makes the result for a file that cannot be read.</param>
    /// <returns>
    /// What <paramref name="byPath"/> or <paramref name="byBytes"/> returned,
    /// or what <paramref name="unreadable"/> made (an <c>io</c> error) for a
    /// path that is not Unicode whose file cannot be read; then neither call
    /// is made.
    /// </returns>
    internal static T Call<T>(string path, Func<byte[], T> byPath, Func<byte[], T> byBytes, Func<Diagnostic, T> unreadable)
    {
        if (Capi.Utf8(path) is { } encoded)
        {
            return byPath(encoded);
        }

        byte[] data;
        try
        {
            data = File.ReadAllBytes(path);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
        {
            return unreadable(StatusDiagnostic.Of("io", $"the file could not be read: {error.Message}"));
        }

        return byBytes(data);
    }
}

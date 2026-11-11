// Turns what a verify function returned into a Result: its status and, on
// OK, the result's JSON {"ok", "failOn", "counts", "error", "diagnostics"}.
// The diagnostics are read as the book's are (EngineDiagnostics.Of).

using System.Text.Json;

namespace XlsxGraft;

internal static class VerifyReport
{
    /// <summary>The result of a verify call that returned <paramref name="status"/> and <paramref name="json"/>.</summary>
    /// <param name="status">The status.</param>
    /// <param name="json">The result's JSON as UTF-8 bytes, meaningful when the status is OK.</param>
    /// <returns>
    /// A success when the result's <c>ok</c> is true, with no value; else a
    /// failure with the result's diagnostics, and its <c>error</c> (<c>io</c>)
    /// as <see cref="Result.Error"/>. <c>counts</c> and <c>failOn</c> are not
    /// kept.
    /// </returns>
    /// <exception cref="XlsxGraftException">
    /// The engine refused a call this package built (<c>XLSX_GRAFT_ERR_INVALID_ARG</c>), which only a bug of the package causes.
    /// </exception>
    internal static Result Call(int status, byte[]? json)
    {
        if (status == StatusDiagnostic.Ok)
        {
            return Parsed(json!);
        }

        return status == StatusDiagnostic.InvalidArg
            ? throw new XlsxGraftException($"the engine refused a call this package built (status {status}): a bug of the package")
            : Result.Failure([], StatusDiagnostic.For(status));
    }

    private static Diagnostic? ErrorOf(JsonElement error) =>
        error.ValueKind == JsonValueKind.Null ? null : EngineDiagnostics.Failure(error);

    private static Result Parsed(byte[] json)
    {
        try
        {
            // Bytes that are not UTF-8 are a JsonException too.
            using var document = JsonDocument.Parse(json);
            var report = document.RootElement;
            var diagnostics = report.GetProperty("diagnostics").EnumerateArray().Select(EngineDiagnostics.Of).ToList();
            return report.GetProperty("ok").GetBoolean()
                ? Result.Ok(diagnostics)
                : Result.Failure(diagnostics, ErrorOf(report.GetProperty("error")));
        }
        catch (Exception error) when (EngineDiagnostics.Unreadable(error))
        {
            return Result.Failure([], StatusDiagnostic.Of("diagnostics_unreadable", $"the engine's result could not be read: {error.Message}"));
        }
    }
}

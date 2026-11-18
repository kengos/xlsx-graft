// Turns what a view function returned into a Result<T>: its status and, on
// OK, the view's JSON {"ok", "error", "diagnostics", ...} with the keys of
// what the view shows, which the caller's content function reads into a Tree
// or a Grid. The diagnostics are read as the book's are
// (EngineDiagnostics.Of).

using System.Text.Json;

namespace XlsxGraft;

internal static class ViewReport
{
    private const string NoReason = "the engine refused a value of the options (status 11) and gave no reason";

    /// <summary>The result of a view call that returned these.</summary>
    /// <typeparam name="T">The <see cref="Tree"/> or the <see cref="Grid"/> the view shows.</typeparam>
    /// <param name="status">The status.</param>
    /// <param name="json">The view's JSON as UTF-8 bytes, meaningful when the status is OK.</param>
    /// <param name="message">The engine's message, meaningful when the status is REQUEST_INVALID; <see langword="null"/> when it could not be read.</param>
    /// <param name="content">Reads the keys of what the view shows into its value.</param>
    /// <returns>
    /// A success when the view's <c>ok</c> is true; else a failure with the
    /// view's diagnostics and its <c>error</c> as <see cref="Result.Error"/>.
    /// The value is there either way; it is <see langword="null"/> only when
    /// this package failed to get one (a status other than OK, or JSON it
    /// cannot read).
    /// </returns>
    /// <exception cref="ArgumentException">The engine refused a value of the options (REQUEST_INVALID), with its message.</exception>
    /// <exception cref="XlsxGraftException">
    /// The engine refused a call this package built (<c>XLSX_GRAFT_ERR_INVALID_ARG</c>), which only a bug of the package causes.
    /// </exception>
    internal static Result<T> Call<T>(int status, byte[]? json, string? message, Func<JsonElement, T> content)
        where T : notnull
    {
        if (status == StatusDiagnostic.Ok)
        {
            return Parsed(json!, content);
        }

        return status switch
        {
            StatusDiagnostic.RequestInvalid => throw new ArgumentException(string.IsNullOrEmpty(message) ? NoReason : message),
            StatusDiagnostic.InvalidArg => throw new XlsxGraftException($"the engine refused a call this package built (status {status}): a bug of the package"),
            _ => Result<T>.Failure([], StatusDiagnostic.For(status), default),
        };
    }

    private static Diagnostic? ErrorOf(JsonElement error) =>
        error.ValueKind == JsonValueKind.Null ? null : EngineDiagnostics.Failure(error);

    private static Result<T> Parsed<T>(byte[] json, Func<JsonElement, T> content)
        where T : notnull
    {
        try
        {
            // Bytes that are not UTF-8 are a JsonException too.
            using var document = JsonDocument.Parse(json);
            var view = document.RootElement;
            var value = content(view);
            var diagnostics = view.GetProperty("diagnostics").EnumerateArray().Select(EngineDiagnostics.Of).ToList();
            return view.GetProperty("ok").GetBoolean()
                ? Result<T>.Ok(value, diagnostics)
                : Result<T>.Failure(diagnostics, ErrorOf(view.GetProperty("error")), value);
        }
        catch (Exception error) when (EngineDiagnostics.Unreadable(error) || error is FormatException)
        {
            return Result<T>.Failure([], StatusDiagnostic.Of("diagnostics_unreadable", $"the engine's result could not be read: {error.Message}"), default);
        }
    }
}

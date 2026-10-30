// Reads the JSON xlsx_graft_book_diagnostics writes,
// {"has_errors": B, "error": E, "diagnostics": [...]}.

using System.Text.Json;

namespace XlsxGraft;

internal static class EngineDiagnostics
{
    /// <summary>What a book whose diagnostics could not be read reports: nothing but the package's failure.</summary>
    internal static readonly Report Unread = new(HasErrors: false, Error: null, Diagnostics: []);

    /// <summary>Whether <paramref name="error"/> is what reading JSON not of that shape throws.</summary>
    internal static bool Unreadable(Exception error) =>
        error is JsonException or KeyNotFoundException or InvalidOperationException;

    /// <summary>The report <paramref name="json"/> holds.</summary>
    /// <exception cref="Exception">One that <see cref="Unreadable"/> names, for JSON not of that shape.</exception>
    internal static Report Call(byte[] json)
    {
        // Bytes that are not UTF-8 are a JsonException too.
        using var document = JsonDocument.Parse(json);
        var report = document.RootElement;
        var error = report.GetProperty("error");
        return new Report(
            report.GetProperty("has_errors").GetBoolean(),
            error.ValueKind == JsonValueKind.Null
                ? null
                : StatusDiagnostic.Of(Text(error, "code"), Text(error, "message")),
            [.. report.GetProperty("diagnostics").EnumerateArray().Select(Of)]);
    }

    private static Diagnostic Of(JsonElement record) =>
        new(Text(record, "code"), Text(record, "severity"), Text(record, "message"))
        {
            Args = record.GetProperty("args").Clone(),
            Location = record.GetProperty("location").Clone(),
            DecidedBy = Json(record.GetProperty("decidedBy")),
            Fix = Json(record.GetProperty("fix")),
        };

    // A string member; a JSON null throws as another kind does.
    private static string Text(JsonElement record, string name) =>
        record.GetProperty(name).GetString() ?? throw new InvalidOperationException($"`{name}` is null");

    private static JsonElement? Json(JsonElement value) =>
        value.ValueKind == JsonValueKind.Null ? null : value.Clone();

    /// <summary>What the book said.</summary>
    /// <param name="HasErrors">Whether a diagnostic is an error.</param>
    /// <param name="Error">Its first failure, or <see langword="null"/>.</param>
    /// <param name="Diagnostics">Its diagnostics.</param>
    internal sealed record Report(bool HasErrors, Diagnostic? Error, IReadOnlyList<Diagnostic> Diagnostics);
}

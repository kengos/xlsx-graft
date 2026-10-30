// Turns a C ABI status other than XLSX_GRAFT_OK into a Diagnostic, for a
// status the engine keeps no failure of: one with no book to keep it (an
// open), or after which its diagnostics cannot be read (a panic).
//
// The statuses that refuse a sheet are named as `xlsx-graft read` names them
// in its end line's `error.code`; the book's diagnostics say them in full,
// naming the sheet. The others name the status codes of
// engine/capi/include/xlsx_graft.h.

namespace XlsxGraft;

internal static class StatusDiagnostic
{
    internal const int Ok = 0;
    internal const int Panic = 2;
    internal const int IO = 3;
    internal const int InvalidArg = 4;

    private static readonly Dictionary<int, (string Code, string Message)> Codes = new()
    {
        [1] = ("null_arg", "the engine was passed a null pointer"),
        [Panic] = ("panic", "the engine panicked; the call did not complete"),
        [IO] = ("io", "the host failed to open, read or write a file"),
        [InvalidArg] = ("invalid_arg", "the engine was passed text that is not UTF-8"),
        [5] = ("workbook_unreadable", "the workbook cannot be read; the diagnostics say why"),
        [6] = ("sheet_not_found", "no such sheet"),
        [7] = ("sheet_ambiguous", "more than one sheet has the name, letter case aside"),
        [8] = ("sheet_not_worksheet", "the sheet is not a worksheet"),
        [9] = ("sheet_unreadable", "the sheet cannot be read; the diagnostics say why"),
        [10] = ("closed", "the rows' book was closed"),
    };

    /// <summary>The diagnostic for <paramref name="status"/>, which a C ABI function returned.</summary>
    /// <param name="status">The status.</param>
    /// <param name="message">
    /// What to say in place of the status's own message, for one that knows
    /// more (why the host failed).
    /// </param>
    internal static Diagnostic For(int status, string? message = null)
    {
        var (code, known) = Codes.TryGetValue(status, out var named)
            ? named
            : ("unknown_status", $"the engine returned status {status}, which this package does not know");
        return Of(code, string.IsNullOrEmpty(message) ? known : message);
    }

    /// <summary>An <c>"error"</c> diagnostic of the package's own, not from the engine's diagnostics.</summary>
    internal static Diagnostic Of(string code, string message) => new(code, "error", message);
}

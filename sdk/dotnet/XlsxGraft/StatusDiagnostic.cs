// Turns a C ABI status other than XLSX_GRAFT_OK into a Diagnostic.
//
// The codes are provisional: they name the status codes of
// engine/capi/include/xlsx_graft.h until the engine reports diagnostics of its
// own, which replace them.

namespace XlsxGraft;

internal static class StatusDiagnostic
{
    internal const int Ok = 0;

    /// <summary>The diagnostic for <paramref name="status"/>, which a C ABI function returned.</summary>
    internal static Diagnostic For(int status) => status switch
    {
        1 => new Diagnostic("null_arg", "the engine was passed a null pointer"),
        2 => new Diagnostic("panic", "the engine panicked; the call did not complete"),
        _ => new Diagnostic(
            "unknown_status",
            $"the engine returned status {status}, which this package does not know"),
    };
}

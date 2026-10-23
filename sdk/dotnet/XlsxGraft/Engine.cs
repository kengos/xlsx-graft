// The package's entry points.
//
// Every one opens the engine library (Library.cs) and calls into the C ABI
// through an unmanaged function pointer, not [LibraryImport]: that names its
// library at compile time and resolves it through one resolver per assembly,
// set once, so it cannot follow docs/sdk.md's lookup order (XLSX_GRAFT_LIBRARY, then the
// caller's path, then the packaged copy) per call. The pointer types still
// spell out every argument, and only blittable values cross.

namespace XlsxGraft;

/// <summary>Calls into the xlsx-graft engine.</summary>
/// <example>
/// <code>
/// var result = XlsxGraft.Engine.Version();
/// if (result.Success)
/// {
///     Console.WriteLine(result.Value);
/// }
/// </code>
/// </example>
public static class Engine
{
    /// <summary>The engine's version, such as <c>"0.0.0"</c>, on success.</summary>
    /// <param name="library">
    /// The engine library's path, used when <c>XLSX_GRAFT_LIBRARY</c> is not set.
    /// </param>
    /// <returns>The version, or a failure carrying the engine's diagnostics.</returns>
    /// <exception cref="LibraryNotFoundException">
    /// No engine library was found or loaded, or it exports no <c>xlsx_graft_version</c>.
    /// </exception>
    public static Result<string> Version(string? library = null) =>
        Version(library, Environment.GetEnvironmentVariable);

    internal static Result<string> Version(string? library, Func<string, string?> env) =>
        VersionCall.Call(Library.Open(library, env).Export(VersionCall.Name));
}

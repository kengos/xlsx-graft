// Finds and opens the engine's shared library (libxlsx_graft_capi).
//
// Looked up in this order, and the first one set wins (a set path that does
// not load throws rather than falls through):
//
// 1. the XLSX_GRAFT_LIBRARY environment variable
// 2. the caller's path
// 3. a library in native/ beside the application
//
// The environment comes first because where the engine lives is the
// deployment's decision, and it has to be able to override application code.
// Nothing here downloads anything.

using System.Runtime.InteropServices;

namespace XlsxGraft;

/// <summary>One opened engine library.</summary>
internal sealed class Library
{
    internal const string EnvVar = "XLSX_GRAFT_LIBRARY";

    /// <summary>The file name on each platform. Windows has no <c>lib</c> prefix.</summary>
    internal static readonly string[] Names =
        ["libxlsx_graft_capi.so", "libxlsx_graft_capi.dylib", "xlsx_graft_capi.dll"];

    private readonly IntPtr handle;

    private Library(string path, IntPtr handle)
    {
        Path = path;
        this.handle = handle;
    }

    /// <summary>The file that was opened.</summary>
    internal string Path { get; }

    /// <summary>The library <paramref name="env"/> or <paramref name="path"/> names, or the packaged copy.</summary>
    /// <param name="path">The caller's path, used when <c>XLSX_GRAFT_LIBRARY</c> is not set.</param>
    /// <param name="env">Reads an environment variable.</param>
    /// <param name="baseDirectory">Where the packaged copy is looked for; the application's directory by default.</param>
    /// <exception cref="LibraryNotFoundException">No library was found, or it did not load.</exception>
    internal static Library Open(string? path, Func<string, string?> env, string? baseDirectory = null)
    {
        // An empty value counts as unset, as it does in every SDK.
        var found = NonEmpty(env(EnvVar)) ?? NonEmpty(path) ?? Packaged(baseDirectory ?? AppContext.BaseDirectory);
        if (found is null)
        {
            throw new LibraryNotFoundException(InstallHint("no engine library was found"));
        }

        return new Library(found, Load(found));
    }

    /// <summary>The address of the function the library exports as <paramref name="name"/>.</summary>
    /// <exception cref="LibraryNotFoundException">The library does not export it.</exception>
    internal IntPtr Export(string name)
    {
        if (NativeLibrary.TryGetExport(handle, name, out var address))
        {
            return address;
        }

        throw new LibraryNotFoundException($"{Path} exports no `{name}`");
    }

    private static string? NonEmpty(string? value) => string.IsNullOrEmpty(value) ? null : value;

    // native/ only, as the wheel's xlsx_graft/native/. NuGet's own layouts
    // (runtimes/<rid>/native/, flattened by a RID-specific publish) are for
    // the platform packages to choose when there are any.
    private static string? Packaged(string baseDirectory) =>
        Names.Select(name => System.IO.Path.Combine(baseDirectory, "native", name)).FirstOrDefault(File.Exists);

    // Never freed: unloading a Rust library that has run thread-local
    // destructors is not safe.
    private static IntPtr Load(string path)
    {
        try
        {
            return NativeLibrary.Load(path);
        }
        catch (Exception error) when (error is DllNotFoundException or BadImageFormatException)
        {
            throw new LibraryNotFoundException(InstallHint($"{path} could not be loaded ({error.Message})"), error);
        }
    }

    private static string InstallHint(string reason) =>
        $"{reason}.\n\n"
        + "This package never downloads the engine. Point it at one of these ways:\n"
        + $"  * set {EnvVar} to the path of a libxlsx_graft_capi library\n"
        + "  * pass Engine.Version(library: \"/path/to/libxlsx_graft_capi.so\")\n"
        + "  * put the library in native/ beside the application";
}

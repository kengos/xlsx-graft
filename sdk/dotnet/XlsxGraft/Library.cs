// Finds and opens the engine's shared library (libxlsx_graft_capi).
//
// Looked up in this order, and the first one set wins (a set path that does
// not load throws rather than falls through):
//
// 1. the XLSX_GRAFT_LIBRARY environment variable
// 2. the caller's path
// 3. the library the package ships: in runtimes/<rid>/native/ beside the
//    application (what NuGet copies into a build's output), or beside the
//    application itself (what a publish for one runtime identifier writes)
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

    private static string? Packaged(string baseDirectory) =>
        Candidates(baseDirectory, CurrentRid()).FirstOrDefault(File.Exists);

    /// <summary>Where the packaged library may be, in the order it is looked for: under <c>runtimes/&lt;rid&gt;/native/</c>, then beside the application.</summary>
    /// <param name="baseDirectory">The application's directory.</param>
    /// <param name="rid">This machine's runtime identifier, or null when the package ships none for it.</param>
    internal static IEnumerable<string> Candidates(string baseDirectory, string? rid)
    {
        var directories = rid is null
            ? new[] { baseDirectory }
            : new[] { System.IO.Path.Combine(baseDirectory, "runtimes", rid, "native"), baseDirectory };
        return directories.SelectMany(directory => Names.Select(name => System.IO.Path.Combine(directory, name)));
    }

    /// <summary>This machine's runtime identifier, as the package's <c>runtimes/</c> folders name it.</summary>
    internal static string? CurrentRid() =>
        Rid(
            new[] { OSPlatform.Linux, OSPlatform.OSX, OSPlatform.Windows }.FirstOrDefault(RuntimeInformation.IsOSPlatform),
            RuntimeInformation.ProcessArchitecture);

    /// <summary>The runtime identifier of an operating system and processor, or null for one the package ships no library for.</summary>
    internal static string? Rid(OSPlatform os, Architecture architecture)
    {
        string? system = null;
        if (os == OSPlatform.Linux)
        {
            system = "linux";
        }
        else if (os == OSPlatform.OSX)
        {
            system = "osx";
        }
        else if (os == OSPlatform.Windows)
        {
            system = "win";
        }

        var cpu = architecture switch
        {
            Architecture.X64 => "x64",
            Architecture.Arm64 => "arm64",
            _ => null,
        };
        return system is null || cpu is null ? null : $"{system}-{cpu}";
    }

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
        + "  * pass Engine.Version(library: \"/path/to/libxlsx_graft_capi.so\"),\n"
        + "    or XlsxReader.Open(path, library: ...)\n"
        + "  * put the library in runtimes/<rid>/native/ beside the application,\n"
        + "    or in the application's own directory";
}

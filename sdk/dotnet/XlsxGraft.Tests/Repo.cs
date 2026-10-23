// What every test shares: the engine library and the engine's version.

using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;

namespace XlsxGraft.Tests;

internal static partial class Repo
{
    /// <summary>
    /// The repository's root, from this file's path at compile time: the test
    /// assembly is built under /tmp, outside the checkout.
    /// </summary>
    internal static string Root { get; } = Path.GetFullPath(Path.Combine(SourceDirectory(), "..", "..", ".."));

    /// <summary>The library `make engine:capi-lib` built; the gate points XLSX_GRAFT_LIBRARY at it.</summary>
    internal static string EngineLibrary { get; } =
        Environment.GetEnvironmentVariable("XLSX_GRAFT_LIBRARY")
        ?? throw new InvalidOperationException("XLSX_GRAFT_LIBRARY is not set; the gate sets it");

    /// <summary>The engine workspace's version, from engine/Cargo.toml's [workspace.package].</summary>
    internal static string EngineVersion { get; } =
        WorkspaceVersion().Match(File.ReadAllText(Path.Combine(Root, "engine", "Cargo.toml"))).Groups[1].Value;

    private static string SourceDirectory([CallerFilePath] string path = "") => Path.GetDirectoryName(path)!;

    [GeneratedRegex(@"^\[workspace\.package\]\s*\nversion = ""([^""]+)""", RegexOptions.Multiline)]
    private static partial Regex WorkspaceVersion();
}

// Run by `make sdk:verify:dotnet` with the XlsxGraft package the gate packed
// installed into this project's own package cache: checks the assembly it
// loaded is that package's, what the package holds, and that it reaches the
// engine.
//
//     dotnet run -- <package cache> <engine/Cargo.toml> [release]
//
// A release runs it with `release` on the package it assembled, with
// XLSX_GRAFT_LIBRARY unset: the package then holds the engine, and the one it
// reaches is the package's own.

using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.RegularExpressions;
using System.Xml.Linq;

var packageCache = args[0];
var release = args.Length > 2 && args[2] == "release";
var engine = Regex.Match(File.ReadAllText(args[1]), @"^\[workspace\.package\]\s*\nversion = ""([^""]+)""", RegexOptions.Multiline).Groups[1].Value;

var installed = Path.Combine(packageCache, "xlsxgraft", "0.0.0");
var dll = Path.Combine(installed, "lib", "net10.0", "XlsxGraft.dll");
var loaded = typeof(XlsxGraft.Engine).Assembly.Location;

Check(File.Exists(dll), $"{installed} holds no lib/net10.0/XlsxGraft.dll");
Check(Hash(loaded) == Hash(dll), $"loaded XlsxGraft from {loaded}, which is not {dll}");

// What the nupkg holds, read from the copy the restore keeps beside what it
// extracted: the .NET image's NUGET_XMLDOC_MODE=skip extracts no XML docs.
using var nupkg = ZipFile.OpenRead(Path.Combine(installed, "xlsxgraft.0.0.0.nupkg"));
var entries = nupkg.Entries.Select(e => e.FullName).ToList();
Check(entries.Contains("lib/net10.0/XlsxGraft.xml"), "the package ships no lib/net10.0/XlsxGraft.xml");
Check(
    entries.Any(e => e.StartsWith("runtimes/", StringComparison.Ordinal)) == release,
    release ? "the package ships no runtimes/" : "the package ships runtimes/");
Check(
    !release || Environment.GetEnvironmentVariable("XLSX_GRAFT_LIBRARY") is null,
    "XLSX_GRAFT_LIBRARY is set, so the package's own engine would not be the one reached");
foreach (var notice in new[] { "LICENSE-APACHE", "LICENSE-BSD", "LICENSE-MIT", "NOTICE" })
{
    Check(entries.Contains(notice), $"the package ships no {notice}");
}

Check(
    entries.Contains("THIRD-PARTY-LICENSES") == release,
    release ? "the package ships no THIRD-PARTY-LICENSES" : "the package ships THIRD-PARTY-LICENSES, with no engine in it");

var nuspec = XDocument.Load(Path.Combine(installed, "xlsxgraft.nuspec"));
var dependencies = nuspec.Descendants().Where(e => e.Name.LocalName == "dependency").Select(e => e.Attribute("id")?.Value);
Check(!dependencies.Any(), $"XlsxGraft depends on {string.Join(", ", dependencies)}, not on nothing");

var result = XlsxGraft.Engine.Version();
Check(result.Success, $"Engine.Version() failed: {string.Join(", ", result.Diagnostics)}");
Check(result.Value == engine, $"Engine.Version() is {result.Value}, the engine is {engine}");

// The shipped engine has the verify functions: an empty workbook is refused
// by a diagnostic, not by a failure of the call.
var verified = new XlsxGraft.Verifier().VerifyBytes([]);
Check(
    verified.Failed && verified.Error is null && verified.Diagnostics.Count > 0,
    $"Verifier.VerifyBytes gave success {verified.Success}, error {verified.Error}, {verified.Diagnostics.Count} diagnostics");

Console.WriteLine($"installed XlsxGraft 0.0.0 from {installed} loads engine {result.Value}");

static string Hash(string path) => Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));

static void Check(bool condition, string failure)
{
    if (!condition)
    {
        Console.Error.WriteLine(failure);
        Environment.Exit(1);
    }
}

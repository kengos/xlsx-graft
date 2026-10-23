// The package downloads nothing and depends on nothing (docs/sdk.md § Finding
// the engine library): System.Runtime.InteropServices is in the framework.

using System.Reflection;
using System.Xml.Linq;

namespace XlsxGraft.Tests;

public sealed class NoDownloadTests
{
    private static readonly Assembly Package = typeof(Engine).Assembly;

    [Fact]
    public void References_nothing_but_the_framework()
    {
        // netstandard is the framework's facade, which coverlet's
        // instrumentation of the library references.
        var referenced = Package.GetReferencedAssemblies().Select(name => name.Name!);

        Assert.All(referenced, name => Assert.True(name is "System" or "netstandard" || name.StartsWith("System.", StringComparison.Ordinal), name));
    }

    [Fact]
    public void References_no_networking_or_process_assembly()
    {
        // A method body that used HttpClient or Process would reference these
        // even where no signature names their types.
        string[] forbidden = ["System.Net", "System.Diagnostics.Process"];

        Assert.DoesNotContain(
            Package.GetReferencedAssemblies(),
            name => forbidden.Any(prefix => name.Name!.StartsWith(prefix, StringComparison.Ordinal)));
    }

    [Fact]
    public void No_member_takes_or_returns_a_networking_or_process_type()
    {
        string[] forbidden = ["System.Net", "System.Diagnostics.Process"];
        const BindingFlags all = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly;
        var types = Package.GetTypes()
            .SelectMany(type => type.GetMethods(all))
            .SelectMany(method => method.GetParameters().Select(p => p.ParameterType).Append(method.ReturnType))
            .Select(type => type.FullName ?? string.Empty)
            .ToList();

        // A sweep over no members reports no offenders and proves nothing.
        Assert.Contains(types, name => name.StartsWith("XlsxGraft.Result`1", StringComparison.Ordinal));
        Assert.DoesNotContain(types, name => forbidden.Any(prefix => name.StartsWith(prefix, StringComparison.Ordinal)));
    }

    [Fact]
    public void Declares_no_package_references()
    {
        var project = XDocument.Load(Path.Combine(Repo.Root, "sdk", "dotnet", "XlsxGraft", "XlsxGraft.csproj"));

        Assert.Empty(project.Descendants("PackageReference"));
        Assert.NotEmpty(project.Descendants("PackageId"));
    }
}

using System.Runtime.InteropServices;

namespace XlsxGraft.Tests;

public sealed class LibraryTests : IDisposable
{
    private static readonly Func<string, string?> NoEnv = _ => null;

    // Where a test puts a packaged copy; empty unless it does.
    private readonly DirectoryInfo baseDirectory = Directory.CreateTempSubdirectory("xlsx-graft-");

    public void Dispose() => baseDirectory.Delete(recursive: true);

    [Fact]
    public void Takes_XLSX_GRAFT_LIBRARY_over_the_path_argument()
    {
        var opened = Library.Open("/nowhere.so", Env(Repo.EngineLibrary), baseDirectory.FullName);

        Assert.Equal(Repo.EngineLibrary, opened.Path);
    }

    [Fact]
    public void Takes_the_path_argument_when_XLSX_GRAFT_LIBRARY_is_empty()
    {
        var opened = Library.Open(Repo.EngineLibrary, Env(string.Empty), baseDirectory.FullName);

        Assert.Equal(Repo.EngineLibrary, opened.Path);
    }

    [Fact]
    public void Falls_back_to_the_packaged_copy()
    {
        var native = baseDirectory.CreateSubdirectory(Path.Combine("runtimes", Library.CurrentRid()!, "native"));
        var packaged = Path.Combine(native.FullName, "libxlsx_graft_capi.so");
        File.Copy(Repo.EngineLibrary, packaged);

        var opened = Library.Open(string.Empty, NoEnv, baseDirectory.FullName);

        Assert.Equal(packaged, opened.Path);
    }

    [Fact]
    public void Falls_back_to_the_library_beside_the_application_as_a_publish_for_one_runtime_writes_it()
    {
        var packaged = Path.Combine(baseDirectory.FullName, "libxlsx_graft_capi.so");
        File.Copy(Repo.EngineLibrary, packaged);

        var opened = Library.Open(string.Empty, NoEnv, baseDirectory.FullName);

        Assert.Equal(packaged, opened.Path);
    }

    [Fact]
    public void Looks_under_runtimes_for_this_machine_before_the_application_directory()
    {
        var candidates = Library.Candidates("/app", "linux-arm64").ToList();

        Assert.Equal(6, candidates.Count);
        Assert.Equal(Path.Combine("/app", "runtimes", "linux-arm64", "native", "libxlsx_graft_capi.so"), candidates[0]);
        Assert.Equal(Path.Combine("/app", "libxlsx_graft_capi.so"), candidates[3]);
    }

    [Fact]
    public void Looks_only_in_the_application_directory_when_the_package_ships_nothing_for_this_machine()
    {
        var candidates = Library.Candidates("/app", null).ToList();

        Assert.Equal(3, candidates.Count);
        Assert.All(candidates, candidate => Assert.Equal("/app", Path.GetDirectoryName(candidate)));
    }

    [Theory]
    [InlineData("Linux", Architecture.X64, "linux-x64")]
    [InlineData("Linux", Architecture.Arm64, "linux-arm64")]
    [InlineData("OSX", Architecture.X64, "osx-x64")]
    [InlineData("OSX", Architecture.Arm64, "osx-arm64")]
    [InlineData("Windows", Architecture.X64, "win-x64")]
    [InlineData("Windows", Architecture.Arm64, "win-arm64")]
    [InlineData("FreeBSD", Architecture.X64, null)]
    [InlineData("Linux", Architecture.X86, null)]
    public void Names_the_runtime_identifier_of_an_operating_system_and_processor(
        string os, Architecture architecture, string? expected)
    {
        Assert.Equal(expected, Library.Rid(OSPlatform.Create(os), architecture));
    }

    [Fact]
    public void Names_the_runtime_identifier_of_every_platform_a_release_builds()
    {
        // The release puts each platform's library in runtimes/<rid>/native/ by
        // the table's rid column; the SDK looks it up by its own mapping.
        var rows = File.ReadAllLines(Path.Combine(Repo.Root, "scripts", "release", "platforms.tsv"))
            .Where(line => !line.StartsWith('#'))
            .Select(line => line.Split('\t'))
            .ToList();
        var header = rows[0];
        var slug = Array.IndexOf(header, "slug");
        var rid = Array.IndexOf(header, "rid");

        var platforms = rows.Skip(1).ToList();
        Assert.Equal(5, platforms.Count);
        foreach (var row in platforms)
        {
            var words = row[slug].Split('-');
            var os = words[0] switch
            {
                "darwin" => OSPlatform.OSX,
                "win32" => OSPlatform.Windows,
                _ => OSPlatform.Create(words[0].ToUpperInvariant()[..1] + words[0][1..]),
            };
            var architecture = words[1] == "arm64" ? Architecture.Arm64 : Architecture.X64;

            Assert.Equal(row[rid], Library.Rid(os, architecture));
        }
    }

    [Fact]
    public void Names_a_runtime_identifier_for_the_machine_the_tests_run_on()
    {
        Assert.NotNull(Library.CurrentRid());
    }

    [Fact]
    public void Throws_with_the_ways_to_install_when_nothing_is_found()
    {
        var error = Assert.Throws<LibraryNotFoundException>(() => Library.Open(null, NoEnv, baseDirectory.FullName));

        Assert.StartsWith("no engine library was found.", error.Message, StringComparison.Ordinal);
        Assert.Contains("set XLSX_GRAFT_LIBRARY", error.Message, StringComparison.Ordinal);
        Assert.Contains("Engine.Version(library:", error.Message, StringComparison.Ordinal);
        Assert.Contains("XlsxReader.Open(path, library:", error.Message, StringComparison.Ordinal);
        Assert.Contains("runtimes/<rid>/native/", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Throws_naming_the_file_when_it_is_not_a_library()
    {
        var notALibrary = Path.Combine(baseDirectory.FullName, "not_a_library.so");
        File.WriteAllText(notALibrary, "text");

        var error = Assert.Throws<LibraryNotFoundException>(() => Library.Open(notALibrary, NoEnv, baseDirectory.FullName));

        Assert.StartsWith($"{notALibrary} could not be loaded", error.Message, StringComparison.Ordinal);
        Assert.IsType<DllNotFoundException>(error.InnerException);
    }

    [Fact]
    public void Looks_for_the_packaged_copy_beside_the_application_by_default()
    {
        // The test host's directory is under /tmp, outside the checkout, and
        // no other test opens the library from it.
        var native = Directory.CreateDirectory(Path.Combine(AppContext.BaseDirectory, "runtimes"));
        var packaged = Path.Combine(native.FullName, Library.CurrentRid()!, "native", "libxlsx_graft_capi.so");
        Directory.CreateDirectory(Path.GetDirectoryName(packaged)!);
        File.Copy(Repo.EngineLibrary, packaged);
        try
        {
            Assert.Equal(packaged, Library.Open(null, NoEnv).Path);
        }
        finally
        {
            native.Delete(recursive: true);
        }
    }

    [Fact]
    public void Export_throws_when_the_library_does_not_export_the_name()
    {
        var opened = Library.Open(Repo.EngineLibrary, NoEnv);

        var error = Assert.Throws<LibraryNotFoundException>(() => opened.Export("xlsx_graft_nothing"));

        Assert.Equal($"{Repo.EngineLibrary} exports no `xlsx_graft_nothing`", error.Message);
    }

    [Fact]
    public void Looks_for_each_platforms_file_name_and_Windows_has_no_lib_prefix()
    {
        Assert.Equal(["libxlsx_graft_capi.so", "libxlsx_graft_capi.dylib", "xlsx_graft_capi.dll"], Library.Names);
    }

    private static Func<string, string?> Env(string value) =>
        name => name == "XLSX_GRAFT_LIBRARY" ? value : null;
}

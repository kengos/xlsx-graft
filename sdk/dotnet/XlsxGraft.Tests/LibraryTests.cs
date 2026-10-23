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
        var native = baseDirectory.CreateSubdirectory("native");
        var packaged = Path.Combine(native.FullName, "libxlsx_graft_capi.so");
        File.Copy(Repo.EngineLibrary, packaged);

        var opened = Library.Open(string.Empty, NoEnv, baseDirectory.FullName);

        Assert.Equal(packaged, opened.Path);
    }

    [Fact]
    public void Throws_with_the_ways_to_install_when_nothing_is_found()
    {
        var error = Assert.Throws<LibraryNotFoundException>(() => Library.Open(null, NoEnv, baseDirectory.FullName));

        Assert.StartsWith("no engine library was found.", error.Message, StringComparison.Ordinal);
        Assert.Contains("set XLSX_GRAFT_LIBRARY", error.Message, StringComparison.Ordinal);
        Assert.Contains("Engine.Version(library:", error.Message, StringComparison.Ordinal);
        Assert.Contains("native/", error.Message, StringComparison.Ordinal);
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
        var native = Directory.CreateDirectory(Path.Combine(AppContext.BaseDirectory, "native"));
        var packaged = Path.Combine(native.FullName, "libxlsx_graft_capi.so");
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

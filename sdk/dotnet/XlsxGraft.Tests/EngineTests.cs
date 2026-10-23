namespace XlsxGraft.Tests;

public sealed class EngineTests
{
    [Fact]
    public void Version_returns_the_engines_version_from_the_library()
    {
        var result = Engine.Version();

        Assert.True(result.Success);
        Assert.False(result.Failed);
        Assert.Equal(Repo.EngineVersion, result.Value);
        Assert.Empty(result.Diagnostics);
    }

    [Fact]
    public void The_package_version_tracks_the_engines()
    {
        Assert.Equal(Repo.EngineVersion, typeof(Engine).Assembly.GetName().Version!.ToString(3));
    }

    [Fact]
    public void Version_opens_the_library_argument_when_XLSX_GRAFT_LIBRARY_is_unset()
    {
        Assert.Equal(Repo.EngineVersion, Engine.Version(Repo.EngineLibrary, _ => null).Value);

        var error = Assert.Throws<LibraryNotFoundException>(() => Engine.Version("/elsewhere/libxlsx_graft_capi.so", _ => null));
        Assert.StartsWith("/elsewhere/libxlsx_graft_capi.so could not be loaded", error.Message, StringComparison.Ordinal);
    }
}

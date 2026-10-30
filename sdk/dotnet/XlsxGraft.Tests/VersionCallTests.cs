using System.Runtime.InteropServices;

namespace XlsxGraft.Tests;

public sealed class VersionCallTests
{
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int VersionFunction(IntPtr outPtr, IntPtr outLen);

    [Fact]
    public void A_panic_status_returns_a_failure_carrying_the_panic_diagnostic()
    {
        var result = CallReturning(2);

        Assert.False(result.Success);
        Assert.True(result.Failed);
        Assert.Null(result.Value);
        Assert.Null(result.Error);
        Assert.Equal([new Diagnostic("panic", "error", "the engine panicked; the call did not complete")], result.Diagnostics);
    }

    [Fact]
    public void A_null_arg_status_returns_a_failure_carrying_the_null_arg_diagnostic()
    {
        Assert.Equal(["null_arg"], CallReturning(1).Diagnostics.Select(d => d.Code));
    }

    [Fact]
    public void A_status_this_package_does_not_know_returns_a_failure_naming_it()
    {
        var diagnostic = Assert.Single(CallReturning(42).Diagnostics);

        Assert.Equal("unknown_status", diagnostic.Code);
        Assert.Contains("status 42", diagnostic.Message, StringComparison.Ordinal);
    }

    // A stand-in for xlsx_graft_version that returns `status` with a length
    // written, as a panic may leave one: a failure must not read it.
    private static Result<string> CallReturning(int status)
    {
        VersionFunction function = (_, outLen) =>
        {
            Marshal.WriteIntPtr(outLen, -1);
            return status;
        };
        var result = VersionCall.Call(Marshal.GetFunctionPointerForDelegate(function));
        GC.KeepAlive(function);
        return result;
    }
}
